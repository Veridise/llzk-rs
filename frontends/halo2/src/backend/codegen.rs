use std::marker::PhantomData;
use std::rc::Rc;

use super::events::{BackendMessages, BackendResponse, EventReceiver, Message};
use super::lowering::lowerable::{LowerableExpr, LowerableStmt};
use super::lowering::ExprLowering as _;
use super::{func::FuncIO, lowering::Lowering, resolvers::ResolversProvider};
use crate::ir::expr::IRAexpr;
use crate::{
    expressions::{ExpressionFactory, ScopedExpression},
    gates::AnyQuery,
    halo2::{
        Advice, ColumnType, Expression, Field, Gate, Instance, Rotation, Selector,
    },
    ir::{stmt::IRStmt, CmpOp},
    lookups::callbacks::LookupCallbacks,
    synthesis::{
        constraint::EqConstraint,
        regions::{RegionRow, Row},
        CircuitSynthesis,
    },
    CircuitIO, GateCallbacks,
};
use anyhow::Result;

pub mod lookup;
pub mod queue;
pub mod strats;

pub type BodyResult<O> = Result<Vec<IRStmt<O>>>;

pub trait Codegen<'c: 's, 's>: Sized + 's {
    type FuncOutput: Lowering<F = Self::F>;
    type Output;
    type F: Field + Clone;
    type State: 'c;

    fn initialize(state: &'s Self::State) -> Self;

    fn within_main<FN, L, I>(&self, syn: &CircuitSynthesis<Self::F>, f: FN) -> Result<()>
    where
        FN: FnOnce(&Self::FuncOutput) -> Result<I>,
        I: IntoIterator<Item = L>,
        L: LowerableStmt<F = Self::F>,
    {
        let main = self.define_main_function(syn)?;
        log::debug!("Defined main function");
        let stmts = f(&main)?;
        log::debug!("Collected function body");
        for stmt in stmts {
            stmt.lower(&main)?;
        }
        log::debug!("Lowered function body");
        self.on_scope_end(main)
    }

    fn define_gate_function(
        &self,
        name: &str,
        selectors: &[&Selector],
        input_queries: &[AnyQuery],
        output_queries: &[AnyQuery],
        syn: &CircuitSynthesis<Self::F>,
    ) -> Result<Self::FuncOutput>;

    fn define_function(
        &self,
        name: &str,
        inputs: usize,
        outputs: usize,
        syn: Option<&CircuitSynthesis<Self::F>>,
    ) -> Result<Self::FuncOutput>;

    fn define_function_with_body<FN, L, I>(
        &self,
        name: &str,
        inputs: usize,
        outputs: usize,
        f: FN,
    ) -> Result<()>
    where
        FN: FnOnce(&Self::FuncOutput, &[FuncIO], &[FuncIO]) -> Result<I>,
        I: IntoIterator<Item = L>,
        L: LowerableStmt<F = Self::F>,
    {
        let func = self.define_function(name, inputs, outputs, None)?;
        let inputs = func.lower_function_inputs(0..inputs);
        let outputs = func.lower_function_outputs(0..outputs);
        let stmts = f(&func, &inputs, &outputs)?;
        for stmt in stmts {
            stmt.lower(&func)?;
        }
        self.on_scope_end(func)
    }

    fn define_main_function(&self, syn: &CircuitSynthesis<Self::F>) -> Result<Self::FuncOutput>;

    //fn lower_stmts(
    //    &self,
    //    scope: &Self::FuncOutput,
    //    stmts: impl Iterator<Item = Result<IRStmt<impl LowerableStmt<F = Self::F>>>>,
    //) -> Result<()> {
    //    lower_stmts(scope, stmts)
    //}
    //
    fn on_scope_end(&self, _: Self::FuncOutput) -> Result<()> {
        Ok(())
    }

    fn generate_output(self) -> Result<Self::Output>
    where
        Self::Output: 'c;
}

pub trait CodegenQueue<'c: 's, 's>: Codegen<'c, 's> {
    fn event_receiver(
        state: &'s Self::State,
    ) -> impl EventReceiver<Message = BackendMessages<Self::F>> + Clone {
        CodegenEventReceiver::new(Self::initialize(state))
    }

    fn enqueue_stmts(
        &self,
        region: crate::halo2::RegionIndex,
        stmts: Vec<IRStmt<Expression<Self::F>>>,
    ) -> Result<()>;
}

pub trait CodegenStrategy: Default {
    fn codegen<'c: 'st, 's, 'st, C>(
        &self,
        codegen: &C,
        syn: &'s CircuitSynthesis<C::F>,
        lookups: &dyn LookupCallbacks<C::F>,
        gate_cbs: &dyn GateCallbacks<C::F>,
    ) -> Result<()>
    where
        C: Codegen<'c, 'st>,
        Row<'s>: ResolversProvider<C::F> + 's,
        RegionRow<'s, 's>: ResolversProvider<C::F> + 's;
}

pub struct CodegenEventReceiver<'c: 's, 's, C> {
    codegen: Rc<C>,
    _marker: PhantomData<(&'s (), &'c ())>,
}

impl<C> Clone for CodegenEventReceiver<'_, '_, C> {
    fn clone(&self) -> Self {
        Self {
            codegen: self.codegen.clone(),
            _marker: Default::default(),
        }
    }
}

impl<C> CodegenEventReceiver<'_, '_, C> {
    pub fn new(codegen: C) -> Self {
        Self {
            codegen: Rc::new(codegen),
            _marker: Default::default(),
        }
    }
}

impl<'c: 's, 's, C> EventReceiver for CodegenEventReceiver<'c, 's, C>
where
    C: CodegenQueue<'c, 's>,
{
    type Message = BackendMessages<C::F>;

    fn accept(&self, msg: Self::Message) -> Result<<Self::Message as Message>::Response> {
        match msg {
            BackendMessages::EmitStmts(msg) => self
                .codegen
                .enqueue_stmts(msg.0, msg.1)
                .map(BackendResponse::EmitStmts),
        }
    }
}

/// Generates constraint expressions for the equality constraints.
///
/// This function accepts an iterator of equality constraints to facilitate
/// filtering the equality constraints of a group from the global equality constraints graph.
pub fn inter_region_constraints<'s, F: Field>(
    constraints: impl IntoIterator<Item = EqConstraint<F>>,
    advice_io: &'s CircuitIO<Advice>,
    instance_io: &'s CircuitIO<Instance>,
) -> Vec<IRStmt<ScopedExpression<'s, 's, F>>> {
    constraints
        .into_iter()
        .map(|constraint| match constraint {
            EqConstraint::AnyToAny(from, from_row, to, to_row) => (
                ScopedExpression::new(
                    from.query_cell(Rotation::cur()),
                    Row::new(from_row, advice_io, instance_io),
                ),
                ScopedExpression::new(
                    to.query_cell(Rotation::cur()),
                    Row::new(to_row, advice_io, instance_io),
                ),
            ),
            EqConstraint::FixedToConst(column, row, f) => (
                ScopedExpression::new(
                    column.query_cell(Rotation::cur()),
                    Row::new(row, advice_io, instance_io),
                ),
                ScopedExpression::new(
                    Expression::Constant(f),
                    Row::new(row, advice_io, instance_io),
                ),
            ),
        })
        .map(|(lhs, rhs)| IRStmt::constraint(CmpOp::Eq, lhs, rhs))
        .collect()
}

pub fn lower_constraints<'g, F, R, S>(
    gate: &'g Gate<F>,
    resolvers: R,
    region_header: S,
    row: Option<usize>,
) -> impl Iterator<Item = IRStmt<ScopedExpression<'g, 'g, F>>> + 'g
where
    R: ResolversProvider<F> + Clone + 'g,
    S: ToString,
    F: Field,
{
    // Prepend a comment if the row number is available
    row.map(|row| {
        IRStmt::comment(format!(
            "gate '{}' @ {} @ row {}",
            gate.name(),
            region_header.to_string(),
            row
        ))
    })
    .into_iter()
    .chain(gate.polynomials().iter().map(move |lhs| {
        IRStmt::constraint(
            CmpOp::Eq,
            resolvers.clone().create_ref(lhs),
            resolvers.clone().create(Expression::Constant(F::ZERO)),
        )
    }))
}

pub fn lower_stmts<Scope: Lowering>(
    scope: &Scope,
    mut stmts: impl Iterator<Item = Result<IRStmt<impl LowerableExpr<F = Scope::F>>>>,
) -> Result<()> {
    stmts.try_for_each(|stmt| stmt.and_then(|stmt| stmt.lower(scope)))
}

#[inline]
fn lower_io<O>(count: usize, f: impl Fn(usize) -> O) -> Vec<O> {
    (0..count).map(f).collect()
}

/// If the given statement is not empty prepends a comment
/// with contextual information.
#[inline]
fn prepend_comment<'a, F: Field>(
    stmt: IRStmt<ScopedExpression<'a, 'a, F>>,
    comment: impl FnOnce() -> IRStmt<ScopedExpression<'a, 'a, F>>,
) -> IRStmt<ScopedExpression<'a, 'a, F>> {
    if stmt.is_empty() {
        return stmt;
    }
    [comment(), stmt].into_iter().collect()
}

/// Converts scoped expressions into concrete arith expressions, disconecting the statements from
/// the lifetime of the scope.
#[inline]
fn scoped_exprs_to_aexpr<'a, F: Field>(
    stmts: Vec<IRStmt<ScopedExpression<'a, 'a, F>>>,
) -> Result<IRStmt<IRAexpr<F>>> {
    stmts
        .into_iter()
        .map(|stmt| stmt.try_map(&IRAexpr::try_from))
        .collect()
}
