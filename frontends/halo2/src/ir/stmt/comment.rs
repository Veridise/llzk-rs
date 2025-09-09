use std::marker::PhantomData;

use anyhow::Result;

use crate::{
    backend::lowering::{
        lowerable::{LowerableExpr, LowerableStmt},
        Lowering,
    },
    ir::equivalency::EqvRelation,
};

pub struct Comment<T>(String, PhantomData<T>);

impl<T> Comment<T> {
    pub fn new(s: impl AsRef<str>) -> Self {
        Self(s.as_ref().to_owned(), Default::default())
    }

    pub fn value(&self) -> &str {
        self.0.as_str()
    }
}

impl<T: LowerableExpr> LowerableStmt for Comment<T> {
    type F = T::F;

    fn lower<L>(self, l: &L) -> Result<()>
    where
        L: Lowering<F = Self::F> + ?Sized,
    {
        l.generate_comment(self.0)
    }
}

impl<T: Clone> Clone for Comment<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone(), Default::default())
    }
}

impl<T> PartialEq for Comment<T> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl<T: std::fmt::Debug> std::fmt::Debug for Comment<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "comment '{}'", self.0)
    }
}

impl<L, R, E> EqvRelation<Comment<L>, Comment<R>> for E
where
    E: EqvRelation<L, R>,
{
    /// Comments are always equivalent
    fn equivalent(_lhs: &Comment<L>, _rhs: &Comment<R>) -> bool {
        true
    }
}
