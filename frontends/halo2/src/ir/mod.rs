use anyhow::Result;
use stmt::IRStmt;

use crate::halo2::{Advice, Instance};
use crate::ir::expr::IRAexpr;
use crate::ir::generate::{region_data, RegionByIndex};
use crate::synthesis::CircuitSynthesis;
use crate::CircuitIO;
use crate::{
    backend::func::{ArgNo, FieldId, FuncIO},
    expressions::{ExpressionInRow, ScopedExpression},
    halo2::RegionIndex,
    ir::groups::GroupBody,
};

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

mod ctx;
pub mod equivalency;
pub mod expr;
pub mod generate;
pub mod groups;
pub mod stmt;

pub use ctx::IRCtx;

// These structs are a WIP.

pub type UnresolvedIRCircuit<'s, F> = IRCircuit<ScopedExpression<'s, 's, F>>;

pub struct IRCircuit<T> {
    groups: Vec<GroupBody<T>>,
    regions_to_groups: Vec<usize>,
}

impl<'s, F: crate::halo2::PrimeField> UnresolvedIRCircuit<'s, F> {
    fn new(
        groups: Vec<GroupBody<ScopedExpression<'s, 's, F>>>,
        regions_to_groups: Vec<usize>,
    ) -> Self {
        Self {
            groups,
            regions_to_groups,
        }
    }

    /// Injects the IR into the specific regions
    pub fn inject_ir(
        &mut self,
        ir: &[(RegionIndex, IRStmt<ExpressionInRow<'s, F>>)],
        syn: &'s CircuitSynthesis<F>,
        ctx: &'s IRCtx,
    ) -> anyhow::Result<()> {
        let regions = region_data(syn)?;
        for (index, stmt) in ir {
            let region = regions[index];
            let group_idx = self.regions_to_groups[**index];
            self.groups[group_idx].inject_ir(
                region,
                stmt,
                ctx.advice_io_of_group(group_idx),
                ctx.instance_io_of_group(group_idx),
                syn.fixed_query_resolver(),
            );
        }
        Ok(())
    }

    /// Resolves the IR.
    pub fn resolve(self, ctx: &IRCtx) -> anyhow::Result<IRCircuit<IRAexpr>> {
        let mut groups = self
            .groups
            .into_iter()
            .map(|g| g.try_map(&IRAexpr::try_from))
            .collect::<Result<Vec<_>, _>>()?;
        for group in &mut groups {
            group.relativize_eq_constraints(ctx)?;
        }
        Ok(IRCircuit {
            groups,
            regions_to_groups: self.regions_to_groups,
        })
    }
}

impl<T> IRCircuit<T> {
    pub fn groups(&self) -> &[GroupBody<T>] {
        &self.groups
    }

    /// Returns the main group.
    ///
    /// Panics if there isn't a main group.
    pub fn main(&self) -> &GroupBody<T> {
        // Reverse the iterator because the main group is likely to be the last one.
        self.groups
            .iter()
            .rev()
            .find(|g| g.is_main())
            .expect("A main group is required")
    }
}

#[allow(dead_code)]
pub struct IRMainFunction<T> {
    advice_io: CircuitIO<Advice>,
    instance_io: CircuitIO<Instance>,
    body: IRStmt<T>,
    /// Set of regions the main function encompases
    regions: std::collections::HashSet<RegionIndex>,
}

impl<T> IRMainFunction<T> {
    #[allow(dead_code)]
    fn new(
        advice_io: CircuitIO<Advice>,
        instance_io: CircuitIO<Instance>,
        body: IRStmt<T>,
        regions: std::collections::HashSet<RegionIndex>,
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
    regions: std::collections::HashSet<RegionIndex>,
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
