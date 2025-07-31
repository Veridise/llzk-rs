use super::{func::FuncIO, lowering::Lowering};
use crate::{
    gates::AnyQuery,
    halo2::{Any, Column, Expression, Field, Fixed, Rotation, Selector},
    ir::{BinaryBoolOp, CircuitStmt},
    synthesis::{regions::Row, CircuitSynthesis},
    CircuitWithIO,
};
use anyhow::Result;

pub mod lookup;
pub mod strats;

pub type BodyResult<O> = Result<Vec<CircuitStmt<O>>>;

#[inline]
fn lower_io<O>(count: usize, f: impl Fn(usize) -> O) -> Vec<O> {
    (0..count).map(f).collect()
}

pub trait Codegen<'c>: Sized {
    type FuncOutput: Lowering<F = Self::F>;
    type Output;
    type F: Field + Clone;

    fn within_main<FN>(&self, syn: &CircuitSynthesis<Self::F>, f: FN) -> Result<()>
    where
        FN: FnOnce(&Self::FuncOutput) -> BodyResult<<Self::FuncOutput as Lowering>::CellOutput>,
    {
        let main = self.define_main_function(syn)?;
        let stmts = f(&main)?;
        self.lower_stmts(&main, stmts.into_iter().map(Ok))?;
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
        syn: &CircuitSynthesis<Self::F>,
    ) -> Result<Self::FuncOutput>;

    fn define_function_with_body<FN>(
        &self,
        name: &str,
        inputs: usize,
        outputs: usize,
        syn: &CircuitSynthesis<Self::F>,
        f: FN,
    ) -> Result<()>
    where
        FN: FnOnce(
            &Self::FuncOutput,
            &[FuncIO],
            &[FuncIO],
        ) -> BodyResult<<Self::FuncOutput as Lowering>::CellOutput>,
    {
        let func = self.define_function(name, inputs, outputs, syn)?;
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
        stmts: impl Iterator<Item = Result<CircuitStmt<<Self::FuncOutput as Lowering>::CellOutput>>>,
    ) -> Result<()> {
        lower_stmts(scope, stmts)
    }

    fn on_scope_end(&self, _: Self::FuncOutput) -> Result<()> {
        Ok(())
    }

    fn generate_output(self) -> Result<Self::Output>;
}

pub trait CodegenStrategy: Default {
    fn codegen<'c, C>(&self, codegen: &C, syn: &CircuitSynthesis<C::F>) -> Result<()>
    where
        C: Codegen<'c>;

    fn inter_region_constraints<'c, F, L>(
        &self,
        scope: &'c L,
        syn: &'c CircuitSynthesis<F>,
    ) -> impl Iterator<Item = Result<CircuitStmt<L::CellOutput>>> + 'c
    where
        F: Field,
        L: Lowering<F = F>,
    {
        let lower_cell = |(col, row): &(Column<Any>, usize)| -> Result<L::CellOutput> {
            let q = col.query_cell::<L::F>(Rotation::cur());
            let row = Row::new(*row, syn.advice_io(), syn.instance_io());
            scope.lower_expr(&q, &row, &row)
        };

        let mut constraints = syn.constraints().collect::<Vec<_>>();
        constraints.sort();
        constraints
            .into_iter()
            .map(move |(from, to)| {
                log::debug!("{from:?} == {to:?}");
                Ok(CircuitStmt::Constraint(
                    BinaryBoolOp::Eq,
                    lower_cell(from)?,
                    lower_cell(to)?,
                ))
            })
            .chain(
                syn.fixed_constraints()
                    .inspect(|r| log::debug!("Fixed constraint: {r:?}"))
                    .map(|r| {
                        r.and_then(|(col, row, f): (Column<Fixed>, _, _)| {
                            let r = Row::new(row, syn.advice_io(), syn.instance_io());
                            let lhs = scope.lower_expr(&col.query_cell(Rotation::cur()), &r, &r)?;
                            let rhs = scope.lower_expr(&Expression::Constant(f), &r, &r)?;
                            Ok(CircuitStmt::Constraint(BinaryBoolOp::Eq, lhs, rhs))
                        })
                    }),
            )
    }
}

pub fn lower_stmts<Scope: Lowering>(
    scope: &Scope,
    mut stmts: impl Iterator<Item = Result<CircuitStmt<<Scope as Lowering>::CellOutput>>>,
) -> Result<()> {
    stmts.try_for_each(|stmt| {
        stmt.and_then(|stmt| {
            stmt.reduce(
                &|name, inputs, outputs| scope.generate_call(&name, &inputs, &outputs),
                &|op, lhs, rhs| scope.checked_generate_constraint(op, &lhs, &rhs),
                &|s| scope.generate_comment(s),
                &|func_io| scope.generate_assume_deterministic(func_io),
                &|expr| scope.generate_assert(&expr),
                &|_, _| (),
            )
        })
    })
}
