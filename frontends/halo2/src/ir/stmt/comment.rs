use std::{convert::identity, marker::PhantomData};

use anyhow::Result;

use crate::backend::{
    func::FuncIO,
    lowering::{Lowerable, Lowering, LoweringOutput},
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

impl<T: Lowerable> Lowerable for Comment<T> {
    type F = T::F;

    fn lower<L>(self, l: &L) -> Result<impl Into<LoweringOutput<L>>>
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
