use std::marker::PhantomData;

use anyhow::Result;

use crate::{
    backend::{
        func::FuncIO,
        lowering::{
            lowerable::{LowerableExpr, LowerableStmt},
            Lowering,
        },
    },
    ir::equivalency::EqvRelation,
};

pub struct AssumeDeterministic<T>(FuncIO, PhantomData<T>);

impl<T> AssumeDeterministic<T> {
    pub fn new(f: FuncIO) -> Self {
        Self(f, Default::default())
    }

    pub fn value(&self) -> FuncIO {
        self.0
    }
}

impl<T: LowerableExpr> LowerableStmt for AssumeDeterministic<T> {
    type F = T::F;

    fn lower<L>(self, l: &L) -> Result<()>
    where
        L: Lowering<F = Self::F> + ?Sized,
    {
        l.generate_assume_deterministic(self.0)
    }
}

impl<T: Clone> Clone for AssumeDeterministic<T> {
    fn clone(&self) -> Self {
        Self(self.0, Default::default())
    }
}

impl<T> PartialEq for AssumeDeterministic<T> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl<T: std::fmt::Debug> std::fmt::Debug for AssumeDeterministic<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "assume-deterministic {:?}", self.0)
    }
}

impl<L, R, E> EqvRelation<AssumeDeterministic<L>, AssumeDeterministic<R>> for E
where
    E: EqvRelation<FuncIO, FuncIO>,
{
    fn equivalent(lhs: &AssumeDeterministic<L>, rhs: &AssumeDeterministic<R>) -> bool {
        E::equivalent(&lhs.0, &rhs.0)
    }
}
