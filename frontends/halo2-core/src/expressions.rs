//! Traits defining the behavior of expressions.

use crate::{
    info_traits::{ChallengeInfo, CreateQuery, QueryInfo, SelectorInfo},
    query::{Advice, Fixed, Instance},
};

/// Trait with types associated to expressions.
pub trait ExpressionTypes: Sized {
    /// Type used for Expression::Selector.
    type Selector: SelectorInfo + Copy + std::fmt::Debug;
    /// Type used for Expression::Fixed.
    type FixedQuery: QueryInfo<Kind = Fixed> + CreateQuery<Self> + Copy + std::fmt::Debug;
    /// Type used for Expression::Advice.
    type AdviceQuery: QueryInfo<Kind = Advice> + CreateQuery<Self> + Copy + std::fmt::Debug;
    /// Type used for Expression::Instance.
    type InstanceQuery: QueryInfo<Kind = Instance> + CreateQuery<Self> + Copy + std::fmt::Debug;
    /// Type used for Expression::Challenge.
    type Challenge: ChallengeInfo + Copy + std::fmt::Debug;
}

/// Trait for querying information about expressions.
pub trait ExpressionInfo: ExpressionTypes {
    /// If the expression is a negation returns a reference to the inner expression. Otherwise
    /// should return `None`.
    fn as_negation(&self) -> Option<&Self>;

    /// If the expression is a query to a fixed cells returns a reference to the query. Otherwise
    /// should return `None`.
    fn as_fixed_query(&self) -> Option<&Self::FixedQuery>;
}

/// Factory trait for creating expressions.
pub trait ExprBuilder<F>: ExpressionTypes {
    /// Create the Expression::Constant case.
    fn constant(f: F) -> Self;

    /// Create the Expression::Selector case.
    fn selector(selector: Self::Selector) -> Self;

    /// Create the Expression::Fixed case.
    fn fixed(fixed_query: Self::FixedQuery) -> Self;

    /// Create the Expression::Advice case.
    fn advice(advice_query: Self::AdviceQuery) -> Self;

    /// Create the Expression::Instance case.
    fn instance(instance_query: Self::InstanceQuery) -> Self;

    /// Create the Expression::Challenge case.
    fn challenge(challenge: Self::Challenge) -> Self;

    /// Create the Expression::Negated case.
    fn negated(expr: Self) -> Self;

    /// Create the Expression::Sum case.
    fn sum(lhs: Self, rhs: Self) -> Self;

    /// Create the Expression::Product case.
    fn product(lhs: Self, rhs: Self) -> Self;

    /// Create the Expression::Scaled case.
    fn scaled(lhs: Self, rhs: F) -> Self;
}

/// Allows evaluating the type with an [`EvalExpression`] evaluator.
pub trait EvaluableExpr<F>: ExpressionTypes {
    /// Evaluates the expression.
    fn evaluate<E: EvalExpression<F, Self>>(&self, evaluator: &E) -> E::Output;
}

/// Evaluates an [`EvaluableExpr`].
pub trait EvalExpression<F, E>
where
    E: ExpressionTypes,
{
    /// Output of the evaluation.
    type Output;

    /// Evaluate the [`Expression::Constant`] case.
    fn constant(&self, f: &F) -> Self::Output;

    /// Evaluate the [`Expression::Selector`] case.
    fn selector(&self, selector: &E::Selector) -> Self::Output;

    /// Evaluate the [`Expression::Fixed`] case.
    fn fixed(&self, fixed_query: &E::FixedQuery) -> Self::Output;

    /// Evaluate the [`Expression::Advice`] case.
    fn advice(&self, advice_query: &E::AdviceQuery) -> Self::Output;

    /// Evaluate the [`Expression::Instance`] case.
    fn instance(&self, instance_query: &E::InstanceQuery) -> Self::Output;

    /// Evaluate the [`Expression::Challenge`] case.
    fn challenge(&self, challenge: &E::Challenge) -> Self::Output;

    /// Evaluate the [`Expression::Negated`] case.
    fn negated(&self, expr: Self::Output) -> Self::Output;

    /// Evaluate the [`Expression::Sum`] case.
    fn sum(&self, lhs: Self::Output, rhs: Self::Output) -> Self::Output;

    /// Evaluate the [`Expression::Product`] case.
    fn product(&self, lhs: Self::Output, rhs: Self::Output) -> Self::Output;

    /// Evaluate the [`Expression::Scaled`] case.
    fn scaled(&self, lhs: Self::Output, rhs: &F) -> Self::Output;
}
