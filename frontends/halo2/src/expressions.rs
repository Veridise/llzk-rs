//! Traits and types related to expressions.

use std::borrow::Cow;

use crate::{
    halo2::Field,
    resolvers::{
        FixedQueryResolver, QueryResolver, ResolversProvider, SelectorResolver, boxed_resolver,
    },
    synthesis::regions::{RegionData, RegionRow},
};

pub(crate) mod constant_folding;
pub(crate) mod utils;

/// Trait for querying information about expressions.
pub trait ExpressionInfo {
    /// If the expression is a negation returns a reference to the inner expression. Otherwise
    /// should return `None`.
    fn as_negation(&self) -> Option<&Self>;

    /// If the expression is a query to a fixed cells returns a reference to the query. Otherwise
    /// should return `None`.
    fn as_fixed_query(&self) -> Option<&crate::halo2::FixedQuery>;
}

/// Factory trait for creating expressions.
pub trait ExprBuilder<F> {
    /// Create the Expression::Constant case.
    fn constant(f: F) -> Self;

    /// Create the Expression::Selector case.
    fn selector(selector: crate::halo2::Selector) -> Self;

    /// Create the Expression::Fixed case.
    fn fixed(fixed_query: crate::halo2::FixedQuery) -> Self;

    /// Create the Expression::Advice case.
    fn advice(advice_query: crate::halo2::AdviceQuery) -> Self;

    /// Create the Expression::Instance case.
    fn instance(instance_query: crate::halo2::InstanceQuery) -> Self;

    /// Create the Expression::Challenge case.
    fn challenge(challenge: crate::halo2::Challenge) -> Self;

    /// Create the Expression::Negated case.
    fn negated(expr: Self) -> Self;

    /// Create the Expression::Sum case.
    fn sum(lhs: Self, rhs: Self) -> Self;

    /// Create the Expression::Product case.
    fn product(lhs: Self, rhs: Self) -> Self;

    /// Create the Expression::Scaled case.
    fn scaled(lhs: Self, rhs: F) -> Self;

    /// Create an expression from a column.
    fn from_column<C: crate::halo2::ColumnType>(
        c: crate::halo2::Column<C>,
        rot: crate::halo2::Rotation,
    ) -> Self;
}

/// Allows evaluating the type with an [`EvalExpression`] evaluator.
pub trait EvaluableExpr<F> {
    /// Evaluates the expression.
    fn evaluate<E: EvalExpression<F>>(&self, evaluator: &E) -> E::Output;
}

/// Evaluates an [`EvaluableExpr`].
pub trait EvalExpression<F> {
    /// Output of the evaluation.
    type Output;

    /// Evaluate the [`Expression::Constant`] case.
    fn constant(&self, f: &F) -> Self::Output;

    /// Evaluate the [`Expression::Selector`] case.
    fn selector(&self, selector: &crate::halo2::Selector) -> Self::Output;

    /// Evaluate the [`Expression::Fixed`] case.
    fn fixed(&self, fixed_query: &crate::halo2::FixedQuery) -> Self::Output;

    /// Evaluate the [`Expression::Advice`] case.
    fn advice(&self, advice_query: &crate::halo2::AdviceQuery) -> Self::Output;

    /// Evaluate the [`Expression::Instance`] case.
    fn instance(&self, instance_query: &crate::halo2::InstanceQuery) -> Self::Output;

    /// Evaluate the [`Expression::Challenge`] case.
    fn challenge(&self, challenge: &crate::halo2::Challenge) -> Self::Output;

    /// Evaluate the [`Expression::Negated`] case.
    fn negated(&self, expr: Self::Output) -> Self::Output;

    /// Evaluate the [`Expression::Sum`] case.
    fn sum(&self, lhs: Self::Output, rhs: Self::Output) -> Self::Output;

    /// Evaluate the [`Expression::Product`] case.
    fn product(&self, lhs: Self::Output, rhs: Self::Output) -> Self::Output;

    /// Evaluate the [`Expression::Scaled`] case.
    fn scaled(&self, lhs: Self::Output, rhs: &F) -> Self::Output;
}

/// Indicates to the driver that the expression should be scoped in that row of the circuit.
///
/// The expression is internally handled by a [`std::borrow::Cow`] and can be a reference or owned.
#[derive(Debug, Clone)]
pub struct ExpressionInRow<'e, E: Clone> {
    expr: Cow<'e, E>,
    row: usize,
}

impl<'e, E: Clone> ExpressionInRow<'e, E> {
    /// Creates a new struct owning the expression.
    pub fn new(row: usize, expr: E) -> Self {
        Self {
            expr: Cow::Owned(expr),
            row,
        }
    }

    /// Creates a new struct from a reference to an expression.
    pub fn from_ref(expr: &'e E, row: usize) -> Self {
        Self {
            expr: Cow::Borrowed(expr),
            row,
        }
    }

    /// Creates a [`ScopedExpression`] scoped by a
    /// [`crate::synthesis::regions::RegionRow`].
    pub(crate) fn scoped_in_region_row<'r, F>(
        self,
        region: RegionData<'r>,
        advice_io: &'r crate::io::AdviceIO,
        instance_io: &'r crate::io::InstanceIO,
        fqr: &'r dyn FixedQueryResolver<F>,
    ) -> anyhow::Result<ScopedExpression<'e, 'r, F, E>>
    where
        F: Field,
    {
        // Rows in injected IR are relative offsets to the region but RegionRow expects the absolute
        // row number.
        let start = region.start().ok_or_else(|| {
            anyhow::anyhow!(
                "Region {:?} (\"{}\") does not have a start row",
                region.index(),
                region.name()
            )
        })?;
        Ok(ScopedExpression::from_cow(
            self.expr,
            RegionRow::new(region, start + self.row, advice_io, instance_io, fqr),
        ))
    }
}

impl<E: Clone> From<(usize, E)> for ExpressionInRow<'_, E> {
    fn from((row, expr): (usize, E)) -> Self {
        Self::new(row, expr)
    }
}

/// Represents an expression associated to a scope.
///
/// The scope is represented by a [`ResolversProvider`] that returns
/// the resolvers required for lowering the expression.
///
/// The expression can be either a reference or owned.
pub(crate) struct ScopedExpression<'e, 'r, F, E>
where
    F: Field,
    E: Clone,
{
    expression: Cow<'e, E>,
    resolvers: Box<dyn ResolversProvider<F> + 'r>,
}

impl<'e, 'r, F, E> ScopedExpression<'e, 'r, F, E>
where
    F: Field,
    E: Clone,
{
    /// Creates a new scope owning the expression
    pub fn new<R>(expression: E, resolvers: R) -> Self
    where
        R: ResolversProvider<F> + 'r,
    {
        Self {
            expression: Cow::Owned(expression),
            resolvers: boxed_resolver(resolvers),
        }
    }

    /// Creates a new scope with a refernece to an expression.
    pub fn from_ref<R>(expression: &'e E, resolvers: R) -> Self
    where
        R: ResolversProvider<F> + 'r,
    {
        Self {
            expression: Cow::Borrowed(expression),
            resolvers: boxed_resolver(resolvers),
        }
    }

    pub(crate) fn from_cow<R>(expression: Cow<'e, E>, resolvers: R) -> Self
    where
        R: ResolversProvider<F> + 'r,
    {
        Self {
            expression,
            resolvers: boxed_resolver(resolvers),
        }
    }

    /// Returns a factory method that creates scopes with always the same resolvers.
    ///
    /// Use it in situations where you can't create the resolvers when you need to create instances
    /// of this struct.
    pub fn make_ctor<R>(resolvers: R) -> impl Fn(E) -> Self + 'r
    where
        R: Clone + ResolversProvider<F> + 'r,
    {
        move |e| Self::new(e, resolvers.clone())
    }

    pub(crate) fn resolvers(&self) -> &dyn ResolversProvider<F> {
        self.resolvers.as_ref()
    }

    pub(crate) fn selector_resolver(&self) -> &dyn SelectorResolver {
        self.resolvers.selector_resolver()
    }

    pub(crate) fn query_resolver(&self) -> &dyn QueryResolver<F> {
        self.resolvers.query_resolver()
    }
}

impl<F, E> std::fmt::Debug for ScopedExpression<'_, '_, F, E>
where
    F: Field,
    E: std::fmt::Debug + Clone,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ScopedExpression")
            .field("expression", &self.expression)
            .finish()
    }
}

impl<F, E> AsRef<E> for ScopedExpression<'_, '_, F, E>
where
    F: Field,
    E: Clone,
{
    fn as_ref(&self) -> &E {
        self.expression.as_ref()
    }
}
