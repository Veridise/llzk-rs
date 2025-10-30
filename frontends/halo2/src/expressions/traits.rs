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
