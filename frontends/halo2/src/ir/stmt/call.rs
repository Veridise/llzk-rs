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

    pub fn map_into<O>(&self, f: &impl Fn(&T) -> O) -> Call<O> {
        Call::new(
            self.callee.clone(),
            self.inputs.iter().map(f),
            self.outputs.clone(),
        )
    }

    pub fn try_map<O>(self, f: &impl Fn(T) -> Result<O>) -> Result<Call<O>> {
        Ok(Call::new(
            self.callee,
            self.inputs.into_iter().map(f).collect::<Result<Vec<_>>>()?,
            self.outputs,
        ))
    }

    pub fn try_map_inplace(&mut self, f: &impl Fn(&mut T) -> Result<()>) -> Result<()> {
        for i in &mut self.inputs {
            f(i)?;
        }
        Ok(())
    }
}

impl<I: LowerableExpr> LowerableStmt for Call<I> {
    fn lower<L>(self, l: &L) -> Result<()>
    where
        L: Lowering + ?Sized,
    {
        let inputs = self
            .inputs
            .into_iter()
            .map(|i| i.lower(l))
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
        if f.alternate() {
            write!(
                f,
                "call '{}'({:#?}) -> ({:#?})",
                self.callee, self.inputs, self.outputs
            )
        } else {
            write!(
                f,
                "call '{}'({:?}) -> ({:?})",
                self.callee, self.inputs, self.outputs
            )
        }
    }
}

impl<L, R, E> EqvRelation<Call<L>, Call<R>> for E
where
    E: EqvRelation<L, R> + EqvRelation<FuncIO, FuncIO>,
{
    /// A call statement is equivalent to another if their input and outputs are equivalent and
    /// point to the same callee.
    fn equivalent(lhs: &Call<L>, rhs: &Call<R>) -> bool {
        lhs.callee == rhs.callee
            && <E as EqvRelation<Vec<L>, Vec<R>>>::equivalent(&lhs.inputs, &rhs.inputs)
            && <E as EqvRelation<Vec<FuncIO>, Vec<FuncIO>>>::equivalent(&lhs.outputs, &rhs.outputs)
    }
}
