use std::{convert::identity, marker::PhantomData};

use anyhow::Result;

use crate::backend::{
    func::FuncIO,
    lowering::{Lowerable, Lowering, LoweringOutput},
};

pub struct Call<I> {
    callee: String,
    inputs: Vec<I>,
    outputs: Vec<FuncIO>,
}

impl<T> Call<T> {
    pub fn new(
        callee: impl AsRef<str>,
        inputs: impl IntoIterator<Item = T>,
        outputs: impl IntoIterator<Item = FuncIO>,
    ) -> Self {
        Self {
            callee: callee.as_ref().to_owned(),
            inputs: inputs.into_iter().collect(),
            outputs: outputs.into_iter().collect(),
        }
    }
    pub fn map<O>(self, f: &impl Fn(T) -> O) -> Call<O> {
        Call::new(self.callee, self.inputs.into_iter().map(f), self.outputs)
    }
    pub fn try_map<O>(self, f: &impl Fn(T) -> Result<O>) -> Result<Call<O>> {
        Ok(Call::new(
            self.callee,
            self.inputs.into_iter().map(f).collect::<Result<Vec<_>>>()?,
            self.outputs,
        ))
    }
}

impl<I: Lowerable> Lowerable for Call<I> {
    type F = I::F;

    fn lower<L>(self, l: &L) -> Result<impl Into<LoweringOutput<L>>>
    where
        L: Lowering<F = Self::F> + ?Sized,
    {
        let inputs = self
            .inputs
            .into_iter()
            .map(|i| l.lower_value(i))
            .collect::<Result<Vec<_>>>()?;
        l.generate_call(self.callee.as_str(), &inputs, &self.outputs)
    }
}

impl<T: Clone> Clone for Call<T> {
    fn clone(&self) -> Self {
        Self {
            callee: self.callee.clone(),
            inputs: self.inputs.clone(),
            outputs: self.outputs.clone(),
        }
    }
}

impl<T: PartialEq> PartialEq for Call<T> {
    fn eq(&self, other: &Self) -> bool {
        self.callee == other.callee && self.inputs == other.inputs && self.outputs == other.outputs
    }
}

impl<T: std::fmt::Debug> std::fmt::Debug for Call<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "call '{}'({:?}) -> ({:?})",
            self.callee, self.inputs, self.outputs
        )
    }
}
