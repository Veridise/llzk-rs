use anyhow::Result;

use crate::{
    backend::lowering::{
        lowerable::{LowerableExpr, LowerableStmt},
        Lowering,
    },
    ir::{equivalency::EqvRelation, expr::IRBexpr},
};

pub struct Assert<T>(IRBexpr<T>);

impl<T> Assert<T> {
    pub fn new(cond: IRBexpr<T>) -> Self {
        Self(cond)
    }

    pub fn map<O>(self, f: &impl Fn(T) -> O) -> Assert<O> {
        Assert::new(self.0.map(f))
    }
    pub fn try_map<O>(self, f: &impl Fn(T) -> Result<O>) -> Result<Assert<O>> {
        self.0.try_map(f).map(Assert::new)
    }
}

impl<T: LowerableExpr> LowerableStmt for Assert<T> {
    type F = T::F;

    fn lower<L>(self, l: &L) -> Result<()>
    where
        L: Lowering<F = Self::F> + ?Sized,
    {
        l.generate_assert(&self.0.lower(l)?)
    }
}

impl<T: Clone> Clone for Assert<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T: PartialEq> PartialEq for Assert<T> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl<T: std::fmt::Debug> std::fmt::Debug for Assert<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "assert ")?;
        if f.alternate() {
            write!(f, "{:#?}", self.0)
        } else {
            write!(f, "{:?}", self.0)
        }
    }
}

impl<L, R, E> EqvRelation<Assert<L>, Assert<R>> for E
where
    E: EqvRelation<L, R>,
{
    fn equivalent(lhs: &Assert<L>, rhs: &Assert<R>) -> bool {
        <E as EqvRelation<IRBexpr<L>, IRBexpr<R>>>::equivalent(&lhs.0, &rhs.0)
    }
}
