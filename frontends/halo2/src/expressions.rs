use std::borrow::Cow;

use crate::backend::resolvers::{
    boxed_resolver, ResolversProvider,
};

use crate::backend::lowering::{Lowerable, Lowering, LoweringOutput};
use crate::halo2::{Expression, Field};
use anyhow::Result;

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

    pub fn make_ctor<R>(resolvers: R) -> impl Fn(Expression<F>) -> Self + 'r
    where
        R: Clone + ResolversProvider<F> + 'r,
    {
        move |e| Self::new(e, resolvers.clone())
    }
}

impl<F> Lowerable for ScopedExpression<'_, '_, F>
where
    F: Field,
{
    type F = F;

    fn lower<L>(self, l: &L) -> Result<impl Into<LoweringOutput<L>>>
    where
        L: Lowering<F = Self::F> + ?Sized,
    {
        l.lower_expr(self.expression.as_ref(), &*self.resolvers)
    }
}
