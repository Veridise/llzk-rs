use std::borrow::Cow;

use crate::{
    halo2::{Expression, Field},
    resolvers::{
        boxed_resolver, FixedQueryResolver, QueryResolver, ResolversProvider, SelectorResolver,
    },
    synthesis::regions::{RegionData, RegionRow},
};

pub mod constant_folding;
pub mod rewriter;
pub mod utils;

/// Indicates to the driver that the expression should be scoped in that row of the circuit.
///
/// The expression is internally handled by a [`std::borrow::Cow`] and can be a reference or owned.
#[derive(Debug, Clone)]
pub struct ExpressionInRow<'e, F: Clone> {
    expr: Cow<'e, Expression<F>>,
    row: usize,
}

impl<'e, F: Clone> ExpressionInRow<'e, F> {
    /// Creates a new struct owning the expression.
    pub fn new(row: usize, expr: Expression<F>) -> Self {
        Self {
            expr: Cow::Owned(expr),
            row,
        }
    }

    /// Creates a new struct from a reference to an expression.
    pub fn from_ref(expr: &'e Expression<F>, row: usize) -> Self {
        Self {
            expr: Cow::Borrowed(expr),
            row,
        }
    }

    /// Creates a [`ScopedExpression`] scoped by a
    /// [`crate::synthesis::regions::RegionRow`].
    pub(crate) fn scoped_in_region_row<'r>(
        self,
        region: RegionData<'r>,
        advice_io: &'r crate::io::AdviceIO,
        instance_io: &'r crate::io::InstanceIO,
        fqr: &'r dyn FixedQueryResolver<F>,
    ) -> anyhow::Result<ScopedExpression<'e, 'r, F>>
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

impl<F: Clone> From<(usize, Expression<F>)> for ExpressionInRow<'_, F> {
    fn from((row, expr): (usize, Expression<F>)) -> Self {
        Self::new(row, expr)
    }
}

/// Represents an expression associated to a scope.
///
/// The scope is represented by a [`ResolversProvider`] that returns
/// the resolvers required for lowering the expression.
///
/// The expression can be either a reference or owned.
pub struct ScopedExpression<'e, 'r, F>
where
    F: Field,
{
    expression: Cow<'e, Expression<F>>,
    resolvers: Box<dyn ResolversProvider<F> + 'r>,
}

impl<'e, 'r, F> ScopedExpression<'e, 'r, F>
where
    F: Field,
{
    /// Creates a new scope owning the expression
    pub fn new<R>(expression: Expression<F>, resolvers: R) -> Self
    where
        R: ResolversProvider<F> + 'r,
    {
        Self {
            expression: Cow::Owned(expression),
            resolvers: boxed_resolver(resolvers),
        }
    }

    /// Creates a new scope with a refernece to an expression.
    pub fn from_ref<R>(expression: &'e Expression<F>, resolvers: R) -> Self
    where
        R: ResolversProvider<F> + 'r,
    {
        Self {
            expression: Cow::Borrowed(expression),
            resolvers: boxed_resolver(resolvers),
        }
    }

    pub(crate) fn from_cow<R>(expression: Cow<'e, Expression<F>>, resolvers: R) -> Self
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
    pub fn make_ctor<R>(resolvers: R) -> impl Fn(Expression<F>) -> Self + 'r
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

impl<F: Field> std::fmt::Debug for ScopedExpression<'_, '_, F> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ScopedExpression")
            .field("expression", &self.expression)
            .finish()
    }
}

impl<F> AsRef<Expression<F>> for ScopedExpression<'_, '_, F>
where
    F: Field,
{
    fn as_ref(&self) -> &Expression<F> {
        self.expression.as_ref()
    }
}
