use anyhow::Result;
use stmt::IRStmt;

use crate::backend::{
    codegen::Codegen,
    func::{ArgNo, FieldId, FuncIO},
    lowering::{Lowerable, Lowering, LoweringOutput},
};

pub mod lift;

pub use stmt::CmpOp as BinaryBoolOp;
pub mod expr;
pub mod stmt;

pub struct IRModule<T> {
    name: String,
    io: (usize, usize),
    body: IRStmt<T>,
}

impl<T> IRModule<T> {
    pub fn new<S>(name: S, inputs: usize, outputs: usize) -> Self
    where
        S: ToString,
    {
        Self::new_with_body(name, inputs, outputs, Default::default())
    }

    pub fn new_with_body<S>(name: S, inputs: usize, outputs: usize, body: IRStmt<T>) -> Self
    where
        S: ToString,
    {
        Self {
            name: name.to_string(),
            io: (inputs, outputs),
            body,
        }
    }

    pub fn new_with_stmts<S>(
        name: S,
        inputs: usize,
        outputs: usize,
        body: impl Iterator<Item = IRStmt<T>>,
    ) -> Self
    where
        S: ToString,
    {
        Self::new_with_body(name, inputs, outputs, IRStmt::seq(body))
    }

    pub fn inputs(&self) -> Vec<FuncIO> {
        (0..self.io.0).map(ArgNo::from).map(Into::into).collect()
    }

    pub fn outputs(&self) -> Vec<FuncIO> {
        (0..self.io.1).map(FieldId::from).map(Into::into).collect()
    }

    pub fn map<O>(self, f: &impl Fn(T) -> O) -> IRModule<O> {
        let body = self.body.map(f);
        IRModule {
            name: self.name,
            io: self.io,
            body,
        }
    }

    pub fn try_map<O>(self, f: &impl Fn(T) -> Result<O>) -> Result<IRModule<O>> {
        let body = self.body.try_map(f)?;
        Ok(IRModule {
            name: self.name,
            io: self.io,
            body,
        })
    }
}

impl<T: Lowerable> IRModule<T> {
    pub(crate) fn generate<'a>(self, codegen: &impl Codegen<'a, F = T::F>) -> anyhow::Result<()> {
        codegen.define_function_with_body(
            self.name.as_str(),
            self.io.0,
            self.io.1,
            |scope, _, _| Ok(vec![self.body]),
        )
    }
}
