use crate::{
    gates::AnyQuery,
    halo2::{
        AdviceQuery, Challenge, Expression, Field, FixedQuery, Gate, InstanceQuery, Selector, Value,
    },
    ir::CircuitStmt,
};
use anyhow::{bail, Result};

use super::{QueryResolver, SelectorResolver};

pub trait Lowering {
    type CellOutput;
    type F: Field;

    fn generate_constraint(
        &self,
        lhs: &Value<Self::CellOutput>,
        rhs: &Value<Self::CellOutput>,
    ) -> Result<()>;

    fn num_constraints(&self) -> usize;

    fn checked_generate_constraint(
        &self,
        lhs: &Value<Self::CellOutput>,
        rhs: &Value<Self::CellOutput>,
    ) -> Result<()> {
        let before = self.num_constraints();
        self.generate_constraint(lhs, rhs)?;
        let after = self.num_constraints();
        if before >= after {
            bail!("Last constraint was not generated!");
        }
        Ok(())
    }

    fn generate_comment(&self, s: String) -> Result<()>;

    fn generate_call(
        &self,
        name: &str,
        selectors: &[Value<Self::CellOutput>],
        queries: &[Value<Self::CellOutput>],
    ) -> Result<()>;

    fn lower_sum<'a, 'l: 'a>(
        &'l self,
        lhs: &Value<Self::CellOutput>,
        rhs: &Value<Self::CellOutput>,
    ) -> Result<Value<Self::CellOutput>>
    where
        Self::CellOutput: 'a;

    fn lower_product<'a>(
        &'a self,
        lhs: &Value<Self::CellOutput>,
        rhs: &Value<Self::CellOutput>,
    ) -> Result<Value<Self::CellOutput>>
    where
        Self::CellOutput: 'a;

    fn lower_neg<'a>(&'a self, expr: &Value<Self::CellOutput>) -> Result<Value<Self::CellOutput>>
    where
        Self::CellOutput: 'a;

    fn lower_scaled<'a>(
        &'a self,
        expr: &Value<Self::CellOutput>,
        scale: &Value<Self::CellOutput>,
    ) -> Result<Value<Self::CellOutput>>
    where
        Self::CellOutput: 'a;

    fn lower_challenge<'a>(&'a self, challenge: &Challenge) -> Result<Value<Self::CellOutput>>
    where
        Self::CellOutput: 'a;

    fn lower_selector<'a, 'l: 'a>(
        &'l self,
        sel: &Selector,
        resolver: &dyn SelectorResolver,
    ) -> Result<Value<Self::CellOutput>>
    where
        Self::CellOutput: 'a;

    fn lower_advice_query<'a>(
        &'a self,
        query: &AdviceQuery,
        resolver: &dyn QueryResolver<Self::F>,
    ) -> Result<Value<Self::CellOutput>>
    where
        Self::CellOutput: 'a;

    fn lower_instance_query<'a>(
        &'a self,
        query: &InstanceQuery,
        resolver: &dyn QueryResolver<Self::F>,
    ) -> Result<Value<Self::CellOutput>>
    where
        Self::CellOutput: 'a;

    fn lower_fixed_query<'a>(
        &'a self,
        query: &FixedQuery,
        resolver: &dyn QueryResolver<Self::F>,
    ) -> Result<Value<Self::CellOutput>>
    where
        Self::CellOutput: 'a;

    fn lower_constant<'a, 'f>(&'a self, f: &'f Self::F) -> Result<Value<Self::CellOutput>>
    where
        Self::CellOutput: 'a,
        'a: 'f;

    fn lower_expr<'a, 'l: 'a>(
        &'l self,
        expr: &Expression<Self::F>,
        query_resolver: &impl QueryResolver<Self::F>,
        selector_resolver: &dyn SelectorResolver,
    ) -> Result<Value<Self::CellOutput>>
    where
        Self::CellOutput: 'a,
    {
        let constant = |f| self.lower_constant(&f);

        let selector_column = |selector| self.lower_selector(&selector, selector_resolver);

        let fixed_column = |query| self.lower_fixed_query(&query, query_resolver);

        let advice_column = |query| self.lower_advice_query(&query, query_resolver);

        let instance_column = |query| self.lower_instance_query(&query, query_resolver);

        let challenge = |challenge| self.lower_challenge(&challenge);

        let negated = |expr| self.lower_neg(&expr?);

        let sum = |lhs, rhs| self.lower_sum(&lhs?, &rhs?);

        let product = |lhs, rhs| self.lower_product(&rhs?, &lhs?);

        let scaled = |expr, scaled| self.lower_scaled(&expr?, &self.lower_constant(&scaled)?);

        expr.evaluate::<Result<Value<Self::CellOutput>>>(
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
    fn lower_exprs<'a, 'l: 'a>(
        &'l self,
        exprs: &[Expression<Self::F>],
        query_resolver: &impl QueryResolver<Self::F>,
        selector_resolver: &dyn SelectorResolver,
    ) -> Result<Vec<Value<Self::CellOutput>>>
    where
        Self::CellOutput: 'a,
    {
        exprs
            .iter()
            .map(|e| self.lower_expr(e, query_resolver, selector_resolver))
            .collect()
    }

    fn lower_selectors<'a, 'l: 'a>(
        &'l self,
        sels: &[&Selector],
        resolver: &dyn SelectorResolver,
    ) -> Result<Vec<Value<Self::CellOutput>>>
    where
        Self::CellOutput: 'a,
    {
        sels.iter()
            .map(|e| self.lower_selector(e, resolver))
            .collect()
    }

    fn lower_any_query<'a>(
        &'a self,
        query: &AnyQuery,
        resolver: &dyn QueryResolver<Self::F>,
    ) -> Result<Value<Self::CellOutput>>
    where
        Self::CellOutput: 'a,
    {
        match query {
            AnyQuery::Advice(advice_query) => self.lower_advice_query(advice_query, resolver),
            AnyQuery::Instance(instance_query) => {
                self.lower_instance_query(instance_query, resolver)
            }
            AnyQuery::Fixed(fixed_query) => self.lower_fixed_query(fixed_query, resolver),
        }
    }

    fn lower_any_queries<'a>(
        &'a self,
        queries: &[AnyQuery],
        resolver: &dyn QueryResolver<Self::F>,
    ) -> Result<Vec<Value<Self::CellOutput>>>
    where
        Self::CellOutput: 'a,
    {
        queries
            .iter()
            .map(|q| self.lower_any_query(q, resolver))
            .collect()
    }

    #[allow(clippy::needless_lifetimes)]
    fn lower_constraints<'c, R>(
        &'c self,
        gate: &Gate<Self::F>,
        resolver: R,
        region_name: &str,
        row: Option<usize>,
    ) -> impl Iterator<Item = Result<CircuitStmt<Self::CellOutput>>>
    where
        R: QueryResolver<Self::F> + SelectorResolver,
    {
        let stmts = match row {
            Some(row) => vec![Ok(CircuitStmt::Comment(format!(
                "gate '{}' @ region {:?} @ row {}",
                gate.name(),
                region_name,
                row
            )))],
            None => vec![],
        };
        stmts
            .into_iter()
            .chain(gate.polynomials().iter().map(move |lhs| {
                Ok(CircuitStmt::EqConstraint(
                    self.lower_expr(lhs, &resolver, &resolver)?,
                    self.lower_expr(&Expression::Constant(Self::F::ZERO), &resolver, &resolver)?,
                ))
            }))
    }
}
