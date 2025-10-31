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
mod traits;

pub use traits::*;

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
