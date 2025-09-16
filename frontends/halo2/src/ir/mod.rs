//! Intermediate representation of circuits. Synthesized circuits get transformed into the structs
//! defined in this module and then the backend uses them to generate the final output.

use anyhow::Result;
use stmt::IRStmt;

use crate::{
    expressions::{ExpressionInRow, ScopedExpression},
    halo2::{PrimeField, RegionIndex},
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

/// Circuit that has not resolved its expressions yet and is still tied to the lifetime
/// of the [`CircuitSynthesis`] and the [`crate::driver::Driver`].
#[derive(Debug)]
pub struct UnresolvedIRCircuit<'ctx, 'syn, 'sco, F>
where
    F: PrimeField,
    'syn: 'sco,
    'ctx: 'sco + 'syn,
{
    ctx: &'ctx IRCtx,
    groups: Vec<GroupBody<ScopedExpression<'syn, 'sco, F>>>,
    regions_to_groups: Vec<usize>,
}

impl<'ctx, 'syn, 'sco, F> UnresolvedIRCircuit<'ctx, 'syn, 'sco, F>
where
    F: PrimeField,
    'syn: 'sco,
    'ctx: 'sco + 'syn,
{
    pub(crate) fn new(
        ctx: &'ctx IRCtx,
        groups: Vec<GroupBody<ScopedExpression<'syn, 'sco, F>>>,
        regions_to_groups: Vec<usize>,
    ) -> Self {
        Self {
            ctx,
            groups,
            regions_to_groups,
        }
    }

    /// Injects the IR into the specific regions
    pub fn inject_ir(
        &mut self,
        ir: &[(RegionIndex, IRStmt<ExpressionInRow<'syn, F>>)],
        syn: &'syn CircuitSynthesis<F>,
    ) {
        let regions = region_data(syn);
        for (index, stmt) in ir {
            let region = regions[index];
            let group_idx = self.regions_to_groups[**index];
            self.groups[group_idx].inject_ir(
                region,
                stmt,
                self.ctx.advice_io_of_group(group_idx),
                self.ctx.instance_io_of_group(group_idx),
                syn.fixed_query_resolver(),
            );
        }
    }

    /// Resolves the IR.
    pub fn resolve(self) -> anyhow::Result<ResolvedIRCircuit> {
        let mut groups = self
            .groups
            .into_iter()
            .map(|g| g.try_map(&IRAexpr::try_from))
            .collect::<Result<Vec<_>, _>>()?;
        for group in &mut groups {
            group.relativize_eq_constraints(self.ctx)?;
        }
        Ok(ResolvedIRCircuit {
            ctx: self.ctx.clone(),
            groups,
            //regions_to_groups: self.regions_to_groups,
        })
    }
}

/// Circuit that has resolved its expressions and is no longer tied to the lifetime of the
/// synthesis and is not parametrized on a prime field.
#[derive(Debug)]
pub struct ResolvedIRCircuit {
    ctx: IRCtx,
    groups: Vec<GroupBody<IRAexpr>>,
}

impl ResolvedIRCircuit {
    /// Returns a list of the groups inside the circuit.
    pub fn groups(&self) -> &[GroupBody<IRAexpr>] {
        &self.groups
    }

    /// Returns the context associated with this circuit.
    pub fn ctx(&self) -> &IRCtx {
        &self.ctx
    }

    /// Returns the main group.
    ///
    /// Panics if there isn't a main group.
    pub fn main(&self) -> &GroupBody<IRAexpr> {
        // Reverse the iterator because the main group is likely to be the last one.
        self.groups
            .iter()
            .rev()
            .find(|g| g.is_main())
            .expect("A main group is required")
    }
}
