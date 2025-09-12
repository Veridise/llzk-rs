use anyhow::Result;
use stmt::IRStmt;

use crate::backend::{
    codegen::Codegen,
    func::{ArgNo, FieldId, FuncIO},
    lowering::lowerable::LowerableExpr,
};
use crate::CircuitIO;

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum CmpOp {
    Eq,
    Lt,
    Le,
    Gt,
    Ge,
    Ne,
}

impl std::fmt::Display for CmpOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                CmpOp::Eq => "==",
                CmpOp::Lt => "<",
                CmpOp::Le => "<=",
                CmpOp::Gt => ">",
                CmpOp::Ge => ">=",
                CmpOp::Ne => "!=",
            }
        )
    }
}

#[cfg(feature = "lift-field-operations")]
pub mod lift;

pub mod equivalency;
pub mod expr;
pub mod generate;
pub mod stmt;

// These structs are a WIP.

#[allow(dead_code)]
pub struct IRCircuit<T> {
    main: IRMainFunction<T>,
    functions: Vec<IRFunction<T>>,
}

#[allow(dead_code)]
pub struct IRMainFunction<T> {
    advice_io: CircuitIO<crate::halo2::Advice>,
    instance_io: CircuitIO<crate::halo2::Instance>,
    body: IRStmt<T>,
    /// Set of regions the main function encompases
    regions: std::collections::HashSet<crate::halo2::RegionIndex>,
}

impl<T> IRMainFunction<T> {
    #[allow(dead_code)]
    fn new(
        advice_io: CircuitIO<crate::halo2::Advice>,
        instance_io: CircuitIO<crate::halo2::Instance>,
        body: IRStmt<T>,
        regions: std::collections::HashSet<crate::halo2::RegionIndex>,
    ) -> Self {
        Self {
            advice_io,
            instance_io,
            body,
            regions,
        }
    }
}

/// For compatibility with the lookup callbacks.
pub type IRModule<T> = IRFunction<T>;

pub struct IRFunction<T> {
    name: String,
    io: (usize, usize),
    body: IRStmt<T>,
    /// Set of regions the function encompases
    regions: std::collections::HashSet<crate::halo2::RegionIndex>,
}

impl<T> IRFunction<T> {
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
            regions: Default::default(),
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

    pub fn map<O>(self, f: &impl Fn(T) -> O) -> IRFunction<O> {
        let body = self.body.map(f);
        IRFunction {
            name: self.name,
            io: self.io,
            body,
            regions: self.regions,
        }
    }

    pub fn try_map<O>(self, f: &impl Fn(T) -> Result<O>) -> Result<IRFunction<O>> {
        let body = self.body.try_map(f)?;
        Ok(IRFunction {
            name: self.name,
            io: self.io,
            body,
            regions: self.regions,
        })
    }
}

impl<T: LowerableExpr> IRFunction<T> {
    #[allow(dead_code)]
    pub(crate) fn generate<'a: 's, 's>(
        self,
        codegen: &impl Codegen<'a, 's, F = T::F>,
    ) -> anyhow::Result<()> {
        codegen.define_function_with_body(self.name.as_str(), self.io.0, self.io.1, |_, _, _| {
            Ok([self.body])
        })
    }
}
