use std::ops::Range;

use crate::{
    gates::AnyQuery,
    halo2::{AdviceQuery, Challenge, Expression, Field, FixedQuery, InstanceQuery, Selector},
    ir::{stmt::IRStmt, CmpOp},
};
use anyhow::{bail, Result};

use super::{func::FuncIO, resolvers::ResolversProvider, QueryResolver, SelectorResolver};

pub enum LoweringOutput<L: Lowering + ?Sized> {
    Value(L::CellOutput),
    Stmt,
}

impl<L: Lowering + ?Sized> From<()> for LoweringOutput<L> {
    fn from(_: ()) -> Self {
        Self::Stmt
    }
}

impl<O: tag::LoweringOutput, L: Lowering<CellOutput = O> + ?Sized> From<O> for LoweringOutput<L> {
    fn from(value: O) -> Self {
        Self::Value(value)
    }
}

pub trait Lowerable {
    type F: Field;

    fn lower<L>(self, l: &L) -> Result<impl Into<LoweringOutput<L>>>
    where
        L: Lowering<F = Self::F> + ?Sized;
}

impl<T> Lowerable for Result<T>
where
    T: Lowerable,
{
    type F = T::F;

    fn lower<L>(self, l: &L) -> Result<impl Into<LoweringOutput<L>>>
    where
        L: Lowering<F = Self::F> + ?Sized,
    {
        self.and_then(|t| t.lower(l))
    }
}

pub enum LowerableOrIO<L> {
    Lowerable(L),
    IO(FuncIO),
}

impl<L> From<L> for LowerableOrIO<L>
where
    L: Lowerable,
{
    fn from(value: L) -> Self {
        Self::Lowerable(value)
    }
}

impl<L> From<FuncIO> for LowerableOrIO<L>
where
    L: Lowerable,
{
    fn from(value: FuncIO) -> Self {
        Self::IO(value)
    }
}

impl<LW> Lowerable for LowerableOrIO<LW>
where
    LW: Lowerable,
{
    type F = LW::F;

    fn lower<L>(self, l: &L) -> Result<impl Into<LoweringOutput<L>>>
    where
        L: Lowering<F = Self::F> + ?Sized,
    {
        match self {
            LowerableOrIO::Lowerable(lowerable) => lowerable.lower(l).map(Into::into),
            LowerableOrIO::IO(func_io) => l.lower_funcio(func_io).map(LoweringOutput::Value),
        }
    }
}

pub enum EitherLowerable<L, R> {
    Left(L),
    Right(R),
}

impl<L, R> EitherLowerable<L, R>
where
    L: Into<R>,
{
    /// If L: Into<R> fold this enum into R
    pub fn fold_right(self) -> R {
        match self {
            EitherLowerable::Left(l) => l.into(),
            EitherLowerable::Right(r) => r,
        }
    }
}

impl<L, R> EitherLowerable<L, R>
where
    R: Into<L>,
{
    /// If R: Into<L> fold this enum into L
    pub fn fold_left(self) -> L {
        match self {
            EitherLowerable::Left(l) => l,
            EitherLowerable::Right(r) => r.into(),
        }
    }
}

impl<T> EitherLowerable<T, T> {
    pub fn unwrap(self) -> T {
        match self {
            EitherLowerable::Left(l) => l,
            EitherLowerable::Right(r) => r,
        }
    }
}

impl<L, R> EitherLowerable<IRStmt<L>, IRStmt<R>>
where
    L: Into<R>,
{
    /// If L: Into<R> fold this enum into R
    pub fn fold_stmt_right(self) -> IRStmt<R> {
        match self {
            EitherLowerable::Left(l) => l.map(&Into::into),
            EitherLowerable::Right(r) => r,
        }
    }
}

impl<L, R> EitherLowerable<IRStmt<L>, IRStmt<R>>
where
    R: Into<L>,
{
    /// If R: Into<L> fold this enum into L
    pub fn fold_stmt_left(self) -> IRStmt<L> {
        match self {
            EitherLowerable::Left(l) => l,
            EitherLowerable::Right(r) => r.map(&Into::into),
        }
    }
}

impl<Left, Right> Lowerable for EitherLowerable<Left, Right>
where
    Left: Lowerable,
    Right: Lowerable<F = Left::F>,
{
    type F = Left::F;

    fn lower<L>(self, l: &L) -> Result<impl Into<LoweringOutput<L>>>
    where
        L: Lowering<F = Self::F> + ?Sized,
    {
        match self {
            EitherLowerable::Left(left) => left.lower(l).map(Into::into),
            EitherLowerable::Right(right) => right.lower(l).map(Into::into),
        }
    }
}

impl<Lw: Lowerable> Lowerable for Box<Lw> {
    type F = Lw::F;

    fn lower<L>(self, l: &L) -> Result<impl Into<LoweringOutput<L>>>
    where
        L: Lowering<F = Self::F> + ?Sized,
    {
        (*self).lower(l)
    }
}

pub mod tag {
    pub trait LoweringOutput {}
}

pub trait Lowering {
    type CellOutput: tag::LoweringOutput;
    type F: Field;

    fn lower_value(&self, l: impl Lowerable<F = Self::F>) -> Result<Self::CellOutput> {
        l.lower(self).and_then(|r| match r.into() {
            LoweringOutput::Value(e) => Ok(e),
            LoweringOutput::Stmt => anyhow::bail!("Expected value but got statement"),
        })
    }

    fn lower_stmt(&self, l: impl Lowerable<F = Self::F>) -> Result<()> {
        l.lower(self).and_then(|r| match r.into() {
            LoweringOutput::Value(_) => anyhow::bail!("Expected statement but got value"),
            LoweringOutput::Stmt => Ok(()),
        })
    }

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
        resolvers: &dyn ResolversProvider<Self::F>,
    ) -> Result<Self::CellOutput> {
        let constant = |f| self.lower_constant(f);

        let selector_column =
            |selector| self.lower_selector(&selector, resolvers.selector_resolver());

        let fixed_column = |query| self.lower_fixed_query(&query, resolvers.query_resolver());

        let advice_column = |query| self.lower_advice_query(&query, resolvers.query_resolver());

        let instance_column = |query| self.lower_instance_query(&query, resolvers.query_resolver());

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
        resolvers: &dyn ResolversProvider<Self::F>,
    ) -> Result<Vec<Self::CellOutput>> {
        exprs
            .iter()
            .map(|e| self.lower_expr(e, resolvers))
            .collect()
    }

    #[allow(dead_code)]
    fn lower_expr_refs(
        &self,
        exprs: &[&Expression<Self::F>],
        resolvers: &dyn ResolversProvider<Self::F>,
    ) -> Result<Vec<Self::CellOutput>> {
        exprs
            .iter()
            .copied()
            .map(|e| self.lower_expr(e, resolvers))
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

    fn generate_assert(&self, expr: &Self::CellOutput) -> Result<()>;

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
