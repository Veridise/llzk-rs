use std::marker::PhantomData;
use std::rc::Rc;

use super::events::{BackendMessages, BackendResponse, EventReceiver, Message};
use super::{
    func::FuncIO,
    lowering::{lowerable::Lowerable, Lowering},
    resolvers::ResolversProvider,
};
use crate::{
    expressions::{ExpressionFactory, ScopedExpression},
    gates::AnyQuery,
    halo2::{Column, ColumnType, Expression, Field, Fixed, Gate, Rotation, Selector},
    ir::{stmt::IRStmt, CmpOp},
    lookups::callbacks::LookupCallbacks,
    synthesis::{
        regions::{RegionRow, Row},
        CircuitSynthesis,
    },
    GateCallbacks,
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
        I: IntoIterator<Item = IRStmt<L>>,
        L: Lowerable<F = Self::F>,
    {
        let main = self.define_main_function(syn)?;
        log::debug!("Defined main function");
        let stmts = f(&main)?;
        log::debug!("Collected function body");
        self.lower_stmts(&main, stmts.into_iter().map(Ok))?;
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

    fn define_function_with_body<FN, L>(
        &self,
        name: &str,
        inputs: usize,
        outputs: usize,
        f: FN,
    ) -> Result<()>
    where
        FN: FnOnce(&Self::FuncOutput, &[FuncIO], &[FuncIO]) -> BodyResult<L>,
        L: Lowerable<F = Self::F>,
    {
        let func = self.define_function(name, inputs, outputs, None)?;
        let inputs = func.lower_function_inputs(0..inputs);
        let outputs = func.lower_function_outputs(0..outputs);
        let stmts = f(&func, &inputs, &outputs)?;
        self.lower_stmts(&func, stmts.into_iter().map(Ok))?;
        self.on_scope_end(func)
    }

    fn define_main_function(&self, syn: &CircuitSynthesis<Self::F>) -> Result<Self::FuncOutput>;

    fn lower_stmts(
        &self,
        scope: &Self::FuncOutput,
        stmts: impl Iterator<Item = Result<IRStmt<impl Lowerable<F = Self::F>>>>,
    ) -> Result<()> {
        lower_stmts(scope, stmts)
    }

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
        Row<'s, C::F>: ResolversProvider<C::F> + 's,
        RegionRow<'s, 's, C::F>: ResolversProvider<C::F> + 's;
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

#[derive(Copy, Clone)]
struct IRCHelper<'s, F: Field> {
    syn: &'s CircuitSynthesis<F>,
}

impl<'s, F: Field> IRCHelper<'s, F> {
    fn lower_cell<'e, C: ColumnType>(
        self,
        (col, row): (Column<C>, usize),
    ) -> ScopedExpression<'e, 's, F>
    where
        's: 'e,
    {
        self.lower_expr(col.query_cell::<F>(Rotation::cur()), row)
    }

    fn lower_const<'e>(self, c: F, row: usize) -> ScopedExpression<'e, 's, F>
    where
        's: 'e,
    {
        self.lower_expr(Expression::Constant(c), row)
    }

    fn lower_expr<'e>(self, e: Expression<F>, row: usize) -> ScopedExpression<'e, 's, F>
    where
        's: 'e,
    {
        Row::new(row, self.syn.advice_io(), self.syn.instance_io()).create(e)
    }
}

pub fn inter_region_constraints<'r, F: Field>(
    syn: &'r CircuitSynthesis<F>,
) -> Result<impl IntoIterator<Item = IRStmt<ScopedExpression<'r, 'r, F>>> + 'r> {
    syn.sorted_constraints()
        .into_iter()
        .map(move |(from, to)| {
            log::debug!("{from:?} == {to:?}");
            let helper = IRCHelper { syn };
            Ok(IRStmt::constraint(
                CmpOp::Eq,
                helper.lower_cell(from),
                helper.lower_cell(to),
            ))
        })
        .chain(
            syn.fixed_constraints()
                .inspect(|r| log::debug!("Fixed constraint: {r:?}"))
                .map(|r| {
                    r.map(|(col, row, f): (Column<Fixed>, _, _)| {
                        let helper = IRCHelper { syn };
                        IRStmt::constraint(
                            CmpOp::Eq,
                            helper.lower_cell((col, row)),
                            helper.lower_const(f, row),
                        )
                    })
                }),
        )
        .collect::<Result<Vec<_>>>()
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
    let stmts = match row {
        Some(row) => vec![IRStmt::comment(format!(
            "gate '{}' @ {} @ row {}",
            gate.name(),
            region_header.to_string(),
            row
        ))],
        None => vec![],
    };
    stmts
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
    mut stmts: impl Iterator<Item = Result<IRStmt<impl Lowerable<F = Scope::F>>>>,
) -> Result<()> {
    stmts.try_for_each(|stmt| stmt.and_then(|stmt| scope.lower_stmt(stmt)))
}

#[inline]
fn lower_io<O>(count: usize, f: impl Fn(usize) -> O) -> Vec<O> {
    (0..count).map(f).collect()
}
