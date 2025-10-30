//! Intermediate representation of circuits. Synthesized circuits get transformed into the structs
//! defined in this module and then the backend uses them to generate the final output.

use anyhow::Result;
use stmt::IRStmt;

use crate::{
    expressions::{EvaluableExpr, ExpressionInRow, ScopedExpression},
    halo2::{PrimeField, RegionIndex},
    ir::{
        expr::{Felt, IRAexpr},
        generate::region_data,
        groups::GroupBody,
        printer::IRPrinter,
    },
    synthesis::SynthesizedCircuit,
    temps::ExprOrTemp,
};

/// Comparison operators between arithmetic expressions.
#[derive(Copy, Clone, PartialEq, Eq, Debug, PartialOrd, Ord, Hash)]
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

mod canon;
mod ctx;
pub mod equivalency;
pub mod expr;
pub mod generate;
pub mod groups;
pub mod printer;
pub mod stmt;

pub use ctx::IRCtx;

/// Circuit that has not resolved its expressions yet and is still tied to the lifetime
/// of the [`CircuitSynthesis`] and the [`crate::driver::Driver`].
#[derive(Debug)]
pub struct UnresolvedIRCircuit<'ctx, 'syn, 'sco, F, E: Clone>
where
    F: PrimeField,
    'syn: 'sco,
    'ctx: 'sco + 'syn,
{
    ctx: &'ctx IRCtx,
    groups: Vec<GroupBody<ExprOrTemp<ScopedExpression<'syn, 'sco, F, E>>>>,
    regions_to_groups: Vec<usize>,
}

impl<'ctx, 'syn, 'sco, F, E: Clone> UnresolvedIRCircuit<'ctx, 'syn, 'sco, F, E>
where
    F: PrimeField,
    'syn: 'sco,
    'ctx: 'sco + 'syn,
{
    pub(crate) fn new(
        ctx: &'ctx IRCtx,
        groups: Vec<GroupBody<ExprOrTemp<ScopedExpression<'syn, 'sco, F, E>>>>,
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
        ir: impl IntoIterator<Item = (RegionIndex, IRStmt<ExpressionInRow<'syn, E>>)>,
        syn: &'syn SynthesizedCircuit<F, E>,
    ) -> anyhow::Result<()> {
        let regions = region_data(syn);
        for (index, stmt) in ir {
            let region = regions[&index];
            let group_idx = self.regions_to_groups[*index];
            self.groups[group_idx].inject_ir(
                region,
                stmt,
                self.ctx.advice_io_of_group(group_idx),
                self.ctx.instance_io_of_group(group_idx),
                syn.fixed_query_resolver(),
            )?;
        }
        Ok(())
    }

    /// Resolves the IR.
    pub fn resolve(self) -> anyhow::Result<ResolvedIRCircuit>
    where
        E: EvaluableExpr<F>,
    {
        let mut groups = self
            .groups
            .into_iter()
            .map(|g| g.try_map(&IRAexpr::try_from))
            .collect::<Result<Vec<_>, _>>()?;
        for group in &mut groups {
            group.relativize_eq_constraints(self.ctx)?;
        }
        Ok(ResolvedIRCircuit {
            prime: Felt::prime::<F>(),
            ctx: self.ctx.clone(),
            groups,
        })
    }

    /// Validates the IR, returning errors if it failed.
    pub fn validate(&self) -> (Result<()>, Vec<String>) {
        let mut errors = vec![];

        for group in &self.groups {
            let (status, group_errors) = group.validate(&self.groups);
            if status.is_err() {
                for err in group_errors {
                    errors.push(format!("Error in group \"{}\": {err}", group.name()));
                }
            }
        }

        (
            if errors.is_empty() {
                Ok(())
            } else {
                Err(anyhow::anyhow!(
                    "Validation of unresolved IR failed with {} errors",
                    errors.len()
                ))
            },
            errors,
        )
    }
}

/// Circuit that has resolved its expressions and is no longer tied to the lifetime of the
/// synthesis and is not parametrized on a prime field.
#[derive(Debug)]
pub struct ResolvedIRCircuit {
    prime: Felt,
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

    /// Returns a printer of the circuit.
    pub fn display<'a>(&'a self) -> IRPrinter<'a> {
        IRPrinter::from_circuit(self)
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

    /// Returns the prime that defines the finite field the circuit uses.
    pub fn prime(&self) -> Felt {
        self.prime
    }

    /// Folds the statements if the expressions are constant.
    ///
    /// If any of the statements fails to fold returns an error.
    pub fn constant_fold(&mut self) -> Result<()> {
        let prime = self.prime();
        self.groups
            .iter_mut()
            .try_for_each(|g| g.constant_fold(prime))
    }

    /// Matches the statements against a series of known patterns and applies rewrites if able to.
    pub fn canonicalize(&mut self) {
        for group in &mut self.groups {
            group.canonicalize();
        }
    }

    /// Validates the IR, returning errors if it failed.
    pub fn validate(&self) -> (Result<()>, Vec<String>) {
        let mut errors = vec![];

        for group in &self.groups {
            let (status, group_errors) = group.validate(&self.groups);
            if status.is_err() {
                for err in group_errors {
                    errors.push(format!("Error in group \"{}\": {err}", group.name()));
                }
            }
        }

        (
            if errors.is_empty() {
                Ok(())
            } else {
                Err(anyhow::anyhow!(
                    "Validation of resolved IR failed with {} errors",
                    errors.len()
                ))
            },
            errors,
        )
    }
}
