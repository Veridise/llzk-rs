use std::fmt;

use crate::{
    gates::AnyQuery,
    halo2::{
        AdviceQuery, Challenge, Expression, Field, FixedQuery, Gate, InstanceQuery, Selector, Value,
    },
    ir::{BinaryBoolOp, CircuitStmt},
};
use anyhow::{bail, Result};

use super::{func::FuncIO, QueryResolver, SelectorResolver};

pub trait Lowering {
    type CellOutput;
    type F: Field;

    fn generate_constraint(
        &self,
        op: BinaryBoolOp,
        lhs: &Self::CellOutput,
        rhs: &Self::CellOutput,
    ) -> Result<()>;

    fn num_constraints(&self) -> usize;

    fn checked_generate_constraint(
        &self,
        op: BinaryBoolOp,
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

    fn lower_sum(&self, lhs: &Self::CellOutput, rhs: &Self::CellOutput)
        -> Result<Self::CellOutput>;

    fn lower_product(
        &self,
        lhs: &Self::CellOutput,
        rhs: &Self::CellOutput,
    ) -> Result<Self::CellOutput>;

    fn lower_neg(&self, expr: &Self::CellOutput) -> Result<Self::CellOutput>;

    fn lower_scaled(
        &self,
        expr: &Self::CellOutput,
        scale: &Self::CellOutput,
    ) -> Result<Self::CellOutput>;

    fn lower_challenge(&self, challenge: &Challenge) -> Result<Self::CellOutput>;

    fn lower_selector(
        &self,
        sel: &Selector,
        resolver: &dyn SelectorResolver,
    ) -> Result<Self::CellOutput>;

    fn lower_advice_query(
        &self,
        query: &AdviceQuery,
        resolver: &dyn QueryResolver<Self::F>,
    ) -> Result<Self::CellOutput>;

    fn lower_instance_query(
        &self,
        query: &InstanceQuery,
        resolver: &dyn QueryResolver<Self::F>,
    ) -> Result<Self::CellOutput>;

    fn lower_fixed_query(
        &self,
        query: &FixedQuery,
        resolver: &dyn QueryResolver<Self::F>,
    ) -> Result<Self::CellOutput>;

    fn lower_constant(&self, f: Self::F) -> Result<Self::CellOutput>;

    fn lower_expr(
        &self,
        expr: &Expression<Self::F>,
        query_resolver: &dyn QueryResolver<Self::F>,
        selector_resolver: &dyn SelectorResolver,
    ) -> Result<Self::CellOutput> {
        let constant = |f| self.lower_constant(f);

        let selector_column = |selector| self.lower_selector(&selector, selector_resolver);

        let fixed_column = |query| self.lower_fixed_query(&query, query_resolver);

        let advice_column = |query| self.lower_advice_query(&query, query_resolver);

        let instance_column = |query| self.lower_instance_query(&query, query_resolver);

        let challenge = |challenge| self.lower_challenge(&challenge);

        let negated = |expr| self.lower_neg(&expr?);

        let sum = |lhs, rhs| self.lower_sum(&lhs?, &rhs?);

        let product = |lhs, rhs| self.lower_product(&lhs?, &rhs?);

        let scaled = |expr, scaled| self.lower_scaled(&expr?, &self.lower_constant(scaled)?);

        expr.evaluate::<Result<Self::CellOutput>>(
            &constant,
            &selector_column,
            &fixed_column,
            &advice_column,
            &instance_column,
            &challenge,
            &negated,
            &sum,
            &product,
            &scaled,
        )
    }

    #[allow(dead_code)]
    fn lower_exprs(
        &self,
        exprs: &[Expression<Self::F>],
        query_resolver: &dyn QueryResolver<Self::F>,
        selector_resolver: &dyn SelectorResolver,
    ) -> Result<Vec<Self::CellOutput>> {
        exprs
            .iter()
            .map(|e| self.lower_expr(e, query_resolver, selector_resolver))
            .collect()
    }

    fn lower_selectors(
        &self,
        sels: &[&Selector],
        resolver: &dyn SelectorResolver,
    ) -> Result<Vec<Self::CellOutput>> {
        sels.iter()
            .map(|e| self.lower_selector(e, resolver))
            .collect()
    }

    fn lower_any_query(
        &self,
        query: &AnyQuery,
        resolver: &dyn QueryResolver<Self::F>,
    ) -> Result<Self::CellOutput> {
        match query {
            AnyQuery::Advice(advice_query) => self.lower_advice_query(advice_query, resolver),
            AnyQuery::Instance(instance_query) => {
                self.lower_instance_query(instance_query, resolver)
            }
            AnyQuery::Fixed(fixed_query) => self.lower_fixed_query(fixed_query, resolver),
        }
    }

    fn lower_any_queries(
        &self,
        queries: &[AnyQuery],
        resolver: &dyn QueryResolver<Self::F>,
    ) -> Result<Vec<Self::CellOutput>> {
        queries
            .iter()
            .map(|q| self.lower_any_query(q, resolver))
            .collect()
    }

    #[allow(clippy::needless_lifetimes)]
    fn lower_constraints<R, S>(
        &self,
        gate: &Gate<Self::F>,
        resolver: R,
        region_header: S,
        row: Option<usize>,
    ) -> impl Iterator<Item = Result<CircuitStmt<Self::CellOutput>>>
    where
        R: QueryResolver<Self::F> + SelectorResolver,
        S: ToString,
    {
        let stmts = match row {
            Some(row) => vec![Ok(CircuitStmt::Comment(format!(
                "gate '{}' @ {} @ row {}",
                gate.name(),
                region_header.to_string(),
                row
            )))],
            None => vec![],
        };
        stmts
            .into_iter()
            .chain(gate.polynomials().iter().map(move |lhs| {
                Ok(CircuitStmt::Constraint(
                    BinaryBoolOp::Eq,
                    self.lower_expr(lhs, &resolver, &resolver)?,
                    self.lower_expr(&Expression::Constant(Self::F::ZERO), &resolver, &resolver)?,
                ))
            }))
    }
}
