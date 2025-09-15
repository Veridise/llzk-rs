//! Intermediate representation of circuits. Synthesized circuits get transformed into the structs
//! defined in this module and then the backend uses them to generate the final output.

use anyhow::Result;
use stmt::IRStmt;

use crate::{
    expressions::{ExpressionInRow, ScopedExpression},
    halo2::RegionIndex,
    ir::{expr::IRAexpr, generate::region_data, groups::GroupBody},
    synthesis::CircuitSynthesis,
};

/// Comparison operators between arithmetic expressions.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum CmpOp {
    /// Equality
    Eq,
    /// Less than
    Lt,
    /// Less of equal than
    Le,
    /// Greater than
    Gt,
    /// Greater or equal than
    Ge,
    /// Not equal
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

mod ctx;
pub mod equivalency;
pub mod expr;
pub mod generate;
pub mod groups;
pub mod stmt;

pub use ctx::IRCtx;

// These structs are a WIP.

/// Alias for a circuit that has not resolved its expressions yet and is still tied to the lifetime
/// of the [`CircuitSynthesis`].
pub type UnresolvedIRCircuit<'s, F> = IRCircuit<ScopedExpression<'s, 's, F>>;

/// Represents the whole circuit.
#[derive(Debug)]
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
    /// Returns a list of the groups inside the circuit.
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
