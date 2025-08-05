use std::{convert::identity, marker::PhantomData};

use anyhow::Result;

use crate::backend::{
    func::FuncIO,
    lowering::{Lowerable, Lowering, LoweringOutput},
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

impl<T: Lowerable> Lowerable for AssumeDeterministic<T> {
    type F = T::F;

    fn lower<L>(self, l: &L) -> Result<impl Into<LoweringOutput<L>>>
    where
        L: Lowering<F = Self::F> + ?Sized,
    {
        l.generate_assume_deterministic(self.0)
    }
}

impl<T: Clone> Clone for AssumeDeterministic<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone(), Default::default())
    }
}
