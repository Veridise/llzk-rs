//use anyhow::Result;

use crate::{expr::IRAexpr, traits::ConstantFolding};
use eqv::{EqvRelation, equiv};
use haloumi_ir_base::{
    SymbolicEqv,
    //equivalency::{EqvRelation, SymbolicEqv},
    felt::Felt,
    func::FuncIO,
};
use haloumi_lowering::{
    Lowering,
    lowerable::{LowerableExpr, LowerableStmt},
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

    pub fn try_map<O, E>(self, f: &impl Fn(T) -> Result<O, E>) -> Result<Call<O>, E> {
        Ok(Call::new(
            self.callee,
            self.inputs
                .into_iter()
                .map(f)
                .collect::<Result<Vec<_>, _>>()?,
            self.outputs,
        ))
    }

    pub fn try_map_inplace<E>(&mut self, f: &impl Fn(&mut T) -> Result<(), E>) -> Result<(), E> {
        for i in &mut self.inputs {
            f(i)?;
        }
        Ok(())
    }

    pub fn callee(&self) -> &str {
        &self.callee
    }

    pub fn inputs(&self) -> &[T] {
        &self.inputs
    }

    pub fn outputs(&self) -> &[FuncIO] {
        &self.outputs
    }

    pub fn inputs_mut(&mut self) -> &mut Vec<T> {
        &mut self.inputs
    }

    pub fn outputs_mut(&mut self) -> &mut Vec<FuncIO> {
        &mut self.outputs
    }

    /// Folds the statements if the expressions are constant.
    pub fn constant_fold(&mut self, prime: T::F) -> Result<(), T::Error>
    where
        T: ConstantFolding,
    {
        for i in &mut self.inputs {
            i.constant_fold(prime)?;
        }
        Ok(())
    }
}

impl<I: LowerableExpr> LowerableStmt for Call<I> {
    fn lower<L>(self, l: &L) -> haloumi_lowering::Result<()>
    where
        L: Lowering + ?Sized,
    {
        let inputs = self
            .inputs
            .into_iter()
            .map(|i| i.lower(l))
            .collect::<Result<Vec<_>, _>>()?;
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

impl<L, R> EqvRelation<Call<L>, Call<R>> for SymbolicEqv
where
    SymbolicEqv: EqvRelation<L, R> + EqvRelation<FuncIO, FuncIO>,
{
    /// A call statement is equivalent to another if their input and outputs are equivalent and
    /// point to the same callee.
    fn equivalent(lhs: &Call<L>, rhs: &Call<R>) -> bool {
        lhs.callee == rhs.callee
            && equiv! { SymbolicEqv | &lhs.inputs, &rhs.inputs }
            && equiv! { SymbolicEqv | &lhs.outputs, &rhs.outputs }
        //&& <SymbolicEqv as EqvRelation<Vec<L>, Vec<R>>>::equivalent(&lhs.inputs, &rhs.inputs)
        //&& <SymbolicEqv as EqvRelation<Vec<FuncIO>, Vec<FuncIO>>>::equivalent(
        //    &lhs.outputs,
        //    &rhs.outputs,
        //)
    }
}
