use std::borrow::Cow;

use crate::backend::lowering::lowerable::LowerableExpr;
use crate::backend::lowering::ExprLowering;
use crate::backend::resolvers::{
    boxed_resolver, QueryResolver, ResolversProvider, SelectorResolver,
};

use crate::halo2::{Expression, Field};
use anyhow::Result;
use constant_folding::ConstantFolding;
use rewriter::rewrite_expr;

pub mod constant_folding;
pub mod rewriter;
pub mod utils;

pub trait ExpressionFactory<'r, F: Field> {
    fn create<'a>(self, e: Expression<F>) -> ScopedExpression<'a, 'r, F>;

    fn create_ref<'a>(self, e: &'a Expression<F>) -> ScopedExpression<'a, 'r, F>;
}

impl<'r, T, F> ExpressionFactory<'r, F> for T
where
    T: ResolversProvider<F> + 'r,
    F: Field,
{
    fn create<'a>(self, e: Expression<F>) -> ScopedExpression<'a, 'r, F> {
        ScopedExpression::new(e, self)
    }

    fn create_ref<'a>(self, e: &'a Expression<F>) -> ScopedExpression<'a, 'r, F> {
        ScopedExpression::from_ref(e, self)
    }
}

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
    pub fn new<R>(expression: Expression<F>, resolvers: R) -> Self
    where
        R: ResolversProvider<F> + 'r,
    {
        Self {
            expression: Cow::Owned(expression),
            resolvers: boxed_resolver(resolvers),
        }
    }

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

    pub fn make_ctor<R>(resolvers: R) -> impl Fn(Expression<F>) -> Self + 'r
    where
        R: Clone + ResolversProvider<F> + 'r,
    {
        move |e| Self::new(e, resolvers.clone())
    }

    pub fn fold_constants(&self) -> Expression<F> {
        rewrite_expr(
            self.expression.as_ref(),
            &[&ConstantFolding::new(self.resolvers.as_ref())],
        )
    }

    pub(crate) fn selector_resolver(&self) -> &dyn SelectorResolver {
        self.resolvers.selector_resolver()
    }

    pub(crate) fn query_resolver(&self) -> &dyn QueryResolver<F> {
        self.resolvers.query_resolver()
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

impl<F> LowerableExpr for ScopedExpression<'_, '_, F>
where
    F: Field,
{
    type F = F;

    fn lower<L>(self, l: &L) -> Result<L::CellOutput>
    where
        L: ExprLowering<F = Self::F> + ?Sized,
    {
        l.lower_expr(self.expression.as_ref(), &*self.resolvers)
    }
}
