use std::{convert::identity, marker::PhantomData};

use anyhow::Result;

use crate::backend::{
    func::FuncIO,
    lowering::{Lowerable, Lowering, LoweringOutput},
};

pub struct Assert<T>(T);

impl<T> Assert<T> {
    pub fn new(cond: T) -> Self {
        Self(cond)
    }

    pub fn map<O>(self, f: &impl Fn(T) -> O) -> Assert<O> {
        Assert::new(f(self.0))
    }
    pub fn try_map<O>(self, f: &impl Fn(T) -> Result<O>) -> Result<Assert<O>> {
        f(self.0).map(Assert::new)
    }
}

impl<T: Lowerable> Lowerable for Assert<T> {
    type F = T::F;

    fn lower<L>(self, l: &L) -> Result<impl Into<LoweringOutput<L>>>
    where
        L: Lowering<F = Self::F> + ?Sized,
    {
        l.lower_value(self.0).and_then(|v| l.generate_assert(&v))
    }
}

impl<T: Clone> Clone for Assert<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}
