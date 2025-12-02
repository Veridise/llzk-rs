//! Traits and types related to expressions.

use std::{borrow::Cow, convert::Infallible, marker::PhantomData, rc::Rc};

use crate::{
    expressions::constant_folding::ConstantFolding,
    resolvers::{
        ChallengeResolver, FixedQueryResolver, QueryResolver, ResolvedQuery, ResolvedSelector,
        ResolversProvider, SelectorResolver, boxed_resolver,
    },
    synthesis::regions::{RegionData, RegionRow},
};

pub(crate) mod constant_folding;

use ff::{Field, PrimeField};
use halo2_frontend_core::expressions::{
    EvalExpression, EvaluableExpr, ExprBuilder, ExpressionInfo, ExpressionTypes,
};
use haloumi_ir::{expr::IRAexpr, felt::Felt};

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
#[derive(Clone)]
pub(crate) struct ScopedExpression<'e, 'r, F, E>
where
    F: Field,
    E: Clone,
{
    expression: Cow<'e, E>,
    resolvers: Rc<dyn ResolversProvider<F> + 'r>,
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

    pub fn simplified<'x>(self) -> ScopedExpression<'x, 'r, F, E>
    where
        E: EvaluableExpr<F> + ExpressionInfo + ExprBuilder<F>,
    {
        let expression =
            ConstantFolding::new(self.resolvers()).constant_fold(self.expression.as_ref());
        ScopedExpression {
            expression: Cow::Owned(expression),
            resolvers: self.resolvers,
        }
    }

    pub fn simplify(&mut self)
    where
        E: EvaluableExpr<F> + ExpressionInfo + ExprBuilder<F>,
    {
        let expression =
            ConstantFolding::new(self.resolvers()).constant_fold(self.expression.as_ref());
        self.expression = Cow::Owned(expression);
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

    pub(crate) fn challenge_resolver(&self) -> &dyn ChallengeResolver {
        self.resolvers.challenge_resolver()
    }
}

impl<F, E> haloumi_ir::traits::ConstantFolding for ScopedExpression<'_, '_, F, E>
where
    E: EvaluableExpr<F> + ExpressionInfo + ExprBuilder<F> + Clone,
    F: Field,
{
    type F = ();

    type Error = Infallible;

    type T = F;

    fn constant_fold(&mut self, _: Self::F) -> Result<(), Self::Error> {
        self.simplify();
        Ok(())
    }

    fn const_value(&self) -> Option<Self::T> {
        struct ConstEval;

        impl<F: Field, E: ExpressionTypes> EvalExpression<F, E> for ConstEval {
            type Output = Option<F>;

            fn constant(&self, f: &F) -> Self::Output {
                Some(*f)
            }

            fn selector(&self, _selector: &E::Selector) -> Self::Output {
                None
            }

            fn fixed(&self, _fixed_query: &E::FixedQuery) -> Self::Output {
                None
            }

            fn advice(&self, _advice_query: &E::AdviceQuery) -> Self::Output {
                None
            }

            fn instance(&self, _instance_query: &E::InstanceQuery) -> Self::Output {
                None
            }

            fn challenge(&self, _challenge: &E::Challenge) -> Self::Output {
                None
            }

            fn negated(&self, expr: Self::Output) -> Self::Output {
                expr.map(|f| -f)
            }

            fn sum(&self, lhs: Self::Output, rhs: Self::Output) -> Self::Output {
                lhs.zip(rhs).map(|(lhs, rhs)| lhs + rhs)
            }

            fn product(&self, lhs: Self::Output, rhs: Self::Output) -> Self::Output {
                lhs.zip(rhs).map(|(lhs, rhs)| lhs * rhs)
            }

            fn scaled(&self, lhs: Self::Output, rhs: &F) -> Self::Output {
                lhs.map(|f| f * rhs)
            }
        }

        self.expression.as_ref().evaluate(&ConstEval)
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

impl<F, E> TryFrom<ScopedExpression<'_, '_, F, E>> for IRAexpr
where
    F: PrimeField,
    E: EvaluableExpr<F> + Clone,
{
    type Error = anyhow::Error;

    fn try_from(expr: ScopedExpression<'_, '_, F, E>) -> Result<Self, Self::Error> {
        expr.as_ref().evaluate(&PolyToAexpr::new(
            expr.selector_resolver(),
            expr.query_resolver(),
            expr.challenge_resolver(),
        ))
    }
}

/// Implements the conversion logic between an [`ScopedExpression`] and [`IRAexpr`].
struct PolyToAexpr<'r, F, E> {
    sr: &'r dyn SelectorResolver,
    qr: &'r dyn QueryResolver<F>,
    cr: &'r dyn ChallengeResolver,
    _marker: PhantomData<E>,
}

impl<'r, F, E> PolyToAexpr<'r, F, E> {
    pub fn new(
        sr: &'r dyn SelectorResolver,
        qr: &'r dyn QueryResolver<F>,
        cr: &'r dyn ChallengeResolver,
    ) -> Self {
        Self {
            sr,
            qr,
            cr,
            _marker: Default::default(),
        }
    }
}

impl<F: PrimeField, E: ExpressionTypes> EvalExpression<F, E> for PolyToAexpr<'_, F, E> {
    type Output = anyhow::Result<IRAexpr>;

    fn constant(&self, f: &F) -> Self::Output {
        Ok(IRAexpr::Constant(Felt::new(*f)))
    }

    fn selector(&self, selector: &E::Selector) -> Self::Output {
        Ok(match self.sr.resolve_selector(selector)? {
            ResolvedSelector::Const(bool) => IRAexpr::Constant(Felt::new::<F>(bool.to_f())),
            ResolvedSelector::Arg(arg) => IRAexpr::IO(arg.into()),
        })
    }

    fn fixed(&self, fixed_query: &E::FixedQuery) -> Self::Output {
        Ok(match self.qr.resolve_fixed_query(fixed_query)? {
            ResolvedQuery::IO(io) => IRAexpr::IO(io),
            ResolvedQuery::Lit(f) => IRAexpr::Constant(Felt::new(f)),
        })
    }

    fn advice(&self, advice_query: &E::AdviceQuery) -> Self::Output {
        Ok(match self.qr.resolve_advice_query(advice_query)? {
            ResolvedQuery::IO(io) => IRAexpr::IO(io),
            ResolvedQuery::Lit(f) => IRAexpr::Constant(Felt::new(f)),
        })
    }

    fn instance(&self, instance_query: &E::InstanceQuery) -> Self::Output {
        Ok(match self.qr.resolve_instance_query(instance_query)? {
            ResolvedQuery::IO(io) => IRAexpr::IO(io),
            ResolvedQuery::Lit(f) => IRAexpr::Constant(Felt::new(f)),
        })
    }

    fn challenge(&self, challenge: &E::Challenge) -> Self::Output {
        Ok(IRAexpr::IO(self.cr.resolve_challenge(challenge)?))
    }

    fn negated(&self, expr: Self::Output) -> Self::Output {
        Ok(IRAexpr::Negated(Box::new(expr?)))
    }

    fn sum(&self, lhs: Self::Output, rhs: Self::Output) -> Self::Output {
        Ok(IRAexpr::Sum(Box::new(lhs?), Box::new(rhs?)))
    }

    fn product(&self, lhs: Self::Output, rhs: Self::Output) -> Self::Output {
        Ok(IRAexpr::Product(Box::new(lhs?), Box::new(rhs?)))
    }

    fn scaled(&self, lhs: Self::Output, rhs: &F) -> Self::Output {
        Ok(IRAexpr::Product(
            Box::new(lhs?),
            Box::new(self.constant(rhs)?),
        ))
    }
}
