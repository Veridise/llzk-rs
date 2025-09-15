use std::ops::Range;

use crate::{
    halo2::Challenge,
    ir::{expr::Felt, CmpOp},
};
use anyhow::{bail, Result};

use super::func::FuncIO;

pub mod lowerable;

pub mod tag {
    pub trait LoweringOutput {}
}

pub trait Lowering: ExprLowering {
    fn generate_constraint(
        &self,
        op: CmpOp,
        lhs: &Self::CellOutput,
        rhs: &Self::CellOutput,
    ) -> Result<()>;

    fn num_constraints(&self) -> usize;

    fn checked_generate_constraint(
        &self,
        op: CmpOp,
        lhs: &Self::CellOutput,
        rhs: &Self::CellOutput,
    ) -> Result<()> {
        let before = self.num_constraints();
        self.generate_constraint(op, lhs, rhs)?;
        let after = self.num_constraints();
        if before >= after {
            bail!("Last constraint was not generated!");
        }
        Ok(())
    }

    fn generate_comment(&self, s: String) -> Result<()>;

    fn generate_assume_deterministic(&self, func_io: FuncIO) -> Result<()>;

    fn generate_call(
        &self,
        name: &str,
        selectors: &[Self::CellOutput],
        outputs: &[FuncIO],
    ) -> Result<()>;

    fn generate_assert(&self, expr: &Self::CellOutput) -> Result<()>;
}

pub trait ExprLowering {
    type CellOutput: tag::LoweringOutput;

    fn lower_sum(&self, lhs: &Self::CellOutput, rhs: &Self::CellOutput)
        -> Result<Self::CellOutput>;

    fn lower_product(
        &self,
        lhs: &Self::CellOutput,
        rhs: &Self::CellOutput,
    ) -> Result<Self::CellOutput>;

    fn lower_neg(&self, expr: &Self::CellOutput) -> Result<Self::CellOutput>;

    fn lower_challenge(&self, challenge: &Challenge) -> Result<Self::CellOutput>;

    //fn lower_selector(
    //    &self,
    //    sel: &Selector,
    //    resolver: &dyn SelectorResolver,
    //) -> Result<Self::CellOutput>;
    //
    //fn lower_advice_query(
    //    &self,
    //    query: &AdviceQuery,
    //    resolver: &dyn QueryResolver<Self::F>,
    //) -> Result<Self::CellOutput>;
    //
    //fn lower_instance_query(
    //    &self,
    //    query: &InstanceQuery,
    //    resolver: &dyn QueryResolver<Self::F>,
    //) -> Result<Self::CellOutput>;
    //
    //fn lower_fixed_query(
    //    &self,
    //    query: &FixedQuery,
    //    resolver: &dyn QueryResolver<Self::F>,
    //) -> Result<Self::CellOutput>;

    fn lower_constant(&self, f: Felt) -> Result<Self::CellOutput>;

    //fn lower_expr(
    //    &self,
    //    expr: &Expression<Self::F>,
    //    resolvers: &dyn ResolversProvider<Self::F>,
    //) -> Result<Self::CellOutput> {
    //    let constant = |f| self.lower_constant(f);
    //
    //    let selector_column =
    //        |selector| self.lower_selector(&selector, resolvers.selector_resolver());
    //
    //    let fixed_column = |query| self.lower_fixed_query(&query, resolvers.query_resolver());
    //
    //    let advice_column = |query| self.lower_advice_query(&query, resolvers.query_resolver());
    //
    //    let instance_column = |query| self.lower_instance_query(&query, resolvers.query_resolver());
    //
    //    let challenge = |challenge| self.lower_challenge(&challenge);
    //
    //    let negated = |expr| self.lower_neg(&expr?);
    //
    //    let sum = |lhs, rhs| self.lower_sum(&lhs?, &rhs?);
    //
    //    let product = |lhs, rhs| self.lower_product(&lhs?, &rhs?);
    //
    //    let scaled = |expr, scaled| self.lower_scaled(&expr?, &self.lower_constant(scaled)?);
    //
    //    expr.evaluate::<Result<Self::CellOutput>>(
    //        &constant,
    //        &selector_column,
    //        &fixed_column,
    //        &advice_column,
    //        &instance_column,
    //        &challenge,
    //        &negated,
    //        &sum,
    //        &product,
    //        &scaled,
    //    )
    //}

    //#[allow(dead_code)]
    //fn lower_exprs(
    //    &self,
    //    exprs: &[Expression<Self::F>],
    //    resolvers: &dyn ResolversProvider<Self::F>,
    //) -> Result<Vec<Self::CellOutput>> {
    //    exprs
    //        .iter()
    //        .map(|e| self.lower_expr(e, resolvers))
    //        .collect()
    //}

    //#[allow(dead_code)]
    //fn lower_expr_refs(
    //    &self,
    //    exprs: &[&Expression<Self::F>],
    //    resolvers: &dyn ResolversProvider<Self::F>,
    //) -> Result<Vec<Self::CellOutput>> {
    //    exprs
    //        .iter()
    //        .copied()
    //        .map(|e| self.lower_expr(e, resolvers))
    //        .collect()
    //}

    //fn lower_selectors(
    //    &self,
    //    sels: &[&Selector],
    //    resolver: &dyn SelectorResolver,
    //) -> Result<Vec<Self::CellOutput>> {
    //    sels.iter()
    //        .map(|e| self.lower_selector(e, resolver))
    //        .collect()
    //}
    //
    //fn lower_any_query(
    //    &self,
    //    query: &AnyQuery,
    //    resolver: &dyn QueryResolver<Self::F>,
    //) -> Result<Self::CellOutput> {
    //    match query {
    //        AnyQuery::Advice(advice_query) => self.lower_advice_query(advice_query, resolver),
    //        AnyQuery::Instance(instance_query) => {
    //            self.lower_instance_query(instance_query, resolver)
    //        }
    //        AnyQuery::Fixed(fixed_query) => self.lower_fixed_query(fixed_query, resolver),
    //    }
    //}
    //
    //fn lower_any_queries(
    //    &self,
    //    queries: &[AnyQuery],
    //    resolver: &dyn QueryResolver<Self::F>,
    //) -> Result<Vec<Self::CellOutput>> {
    //    queries
    //        .iter()
    //        .map(|q| self.lower_any_query(q, resolver))
    //        .collect()
    //}

    fn lower_eq(&self, lhs: &Self::CellOutput, rhs: &Self::CellOutput) -> Result<Self::CellOutput>;
    fn lower_lt(&self, lhs: &Self::CellOutput, rhs: &Self::CellOutput) -> Result<Self::CellOutput>;
    fn lower_le(&self, lhs: &Self::CellOutput, rhs: &Self::CellOutput) -> Result<Self::CellOutput>;
    fn lower_gt(&self, lhs: &Self::CellOutput, rhs: &Self::CellOutput) -> Result<Self::CellOutput>;
    fn lower_ge(&self, lhs: &Self::CellOutput, rhs: &Self::CellOutput) -> Result<Self::CellOutput>;
    fn lower_ne(&self, lhs: &Self::CellOutput, rhs: &Self::CellOutput) -> Result<Self::CellOutput>;
    fn lower_and(&self, lhs: &Self::CellOutput, rhs: &Self::CellOutput)
        -> Result<Self::CellOutput>;
    fn lower_or(&self, lhs: &Self::CellOutput, rhs: &Self::CellOutput) -> Result<Self::CellOutput>;
    fn lower_not(&self, value: &Self::CellOutput) -> Result<Self::CellOutput>;

    fn lower_function_input(&self, i: usize) -> FuncIO;
    fn lower_function_output(&self, o: usize) -> FuncIO;

    fn lower_function_inputs(&self, ins: Range<usize>) -> Vec<FuncIO> {
        ins.map(|i| self.lower_function_input(i)).collect()
    }
    fn lower_function_outputs(&self, outs: Range<usize>) -> Vec<FuncIO> {
        outs.map(|o| self.lower_function_output(o)).collect()
    }

    fn lower_funcio<IO>(&self, io: IO) -> Result<Self::CellOutput>
    where
        IO: Into<FuncIO>;
}
