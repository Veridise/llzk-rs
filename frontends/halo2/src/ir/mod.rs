//! Intermediate representation of circuits. Synthesized circuits get transformed into the structs
//! defined in this module and then the backend uses them to generate the final output.

use crate::{
    expressions::{ExpressionInRow, ScopedExpression},
    ir::{
        generate::region_data,
        groups::{GroupBody, relativize_eq_constraints},
    },
    synthesis::SynthesizedCircuit,
    temps::ExprOrTemp,
};
use anyhow::Result;
use ff::{Field, PrimeField};
use halo2_frontend_core::{expressions::EvaluableExpr, table::RegionIndex};
use haloumi_ir::{
    Felt, Prime,
    printer::{IRPrintable, IRPrinter},
    stmt::IRStmt,
    traits::{Canonicalize as _, ConstantFolding as _},
};
use haloumi_ir::{IRCircuit, expr::IRAexpr};
use std::fmt::Write;
//use haloumi_ir_base::felt::Felt;

//pub use haloumi_ir::{expr, stmt};
//pub use haloumi_ir_base::cmp::CmpOp;

mod ctx;
pub mod generate;
pub mod groups;
//pub mod printer;

pub use ctx::IRCtx;

/// Circuit that has not resolved its expressions yet and is still tied to the lifetime
/// of the [`SynthesizedCircuit`](crate::synthesis::SynthesizedCircuit) and the [`Driver`](crate::driver::Driver).
#[derive(Debug)]
pub struct UnresolvedIRCircuit<'ctx, 'syn, 'sco, F, E>(
    IRCircuit<ExprOrTemp<ScopedExpression<'syn, 'sco, F, E>>, (&'ctx IRCtx, Vec<usize>)>,
)
where
    E: Clone,
    F: Field;

//#[derive(Debug)]
//pub struct UnresolvedIRCircuit<'ctx, 'syn, 'sco, F, E>
//where
//    F: PrimeField,
//    'syn: 'sco,
//    'ctx: 'sco + 'syn,
//    E: Clone,
//{
//    ctx: &'ctx IRCtx,
//    groups: Vec<GroupBody<ExprOrTemp<ScopedExpression<'syn, 'sco, F, E>>>>,
//    regions_to_groups: Vec<usize>,
//}

impl<'ctx, 'syn, 'sco, F, E> UnresolvedIRCircuit<'ctx, 'syn, 'sco, F, E>
where
    F: PrimeField,
    'syn: 'sco,
    'ctx: 'sco + 'syn,
    E: Clone + std::fmt::Debug,
{
    pub(crate) fn new(
        ctx: &'ctx IRCtx,
        groups: Vec<GroupBody<ExprOrTemp<ScopedExpression<'syn, 'sco, F, E>>>>,
        regions_to_groups: Vec<usize>,
    ) -> Self {
        Self(IRCircuit::new(groups, (ctx, regions_to_groups)))
    }

    fn group(
        &mut self,
        index: usize,
    ) -> &mut GroupBody<ExprOrTemp<ScopedExpression<'syn, 'sco, F, E>>> {
        &mut self.0.body_mut()[index]
    }

    fn region_to_groups(&self, index: RegionIndex) -> usize {
        self.0.context().1[*index]
    }

    fn ctx(&self) -> &'ctx IRCtx {
        self.0.context().0
    }

    /// Injects the IR into the specific regions
    pub fn inject_ir<R>(
        &mut self,
        ir: impl IntoIterator<Item = (R, IRStmt<ExpressionInRow<'syn, E>>)>,
        syn: &'syn SynthesizedCircuit<F, E>,
    ) -> anyhow::Result<()>
    where
        R: Into<RegionIndex>,
    {
        let regions = region_data(syn);
        for (index, stmt) in ir {
            let index = index.into();
            let region = regions[&index];
            let group_idx = self.region_to_groups(index);
            let ctx = self.ctx();
            groups::inject_ir(
                self.group(group_idx),
                region,
                stmt,
                ctx.advice_io_of_group(group_idx),
                ctx.instance_io_of_group(group_idx),
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
        let ctx = self.ctx().clone();
        let mut groups = self
            .0
            .take_body()
            .into_iter()
            .map(|g| g.try_map(&mut IRAexpr::try_from))
            .collect::<Result<Vec<_>, _>>()?;
        for group in &mut groups {
            relativize_eq_constraints(group, &ctx)?;
        }
        Ok(ResolvedIRCircuit(IRCircuit::new(
            groups,
            ResolvedCtx(ctx, Prime::new::<F>()),
        )))
    }

    /// Validates the IR, returning errors if it failed.
    pub fn validate(&self) -> (Result<()>, Vec<String>) {
        let (res, errors) = self.0.validate();
        (
            res.map_err(|e| {
                anyhow::anyhow!(
                    "Validation of unresolved IR failed with {} errors",
                    e.error_count()
                )
            }),
            errors,
        )
    }
}

#[derive(Debug)]
struct ResolvedCtx(IRCtx, Prime);

/// Circuit that has resolved its expressions and is no longer tied to the lifetime of the
/// synthesis and is not parametrized on a prime field.
#[derive(Debug)]
pub struct ResolvedIRCircuit(IRCircuit<IRAexpr, ResolvedCtx>);
//    prime: Felt,
//    ctx: IRCtx,
//    groups: Vec<GroupBody<IRAexpr>>,
//}

impl ResolvedIRCircuit {
    /// Returns a list of the groups inside the circuit.
    pub fn groups(&self) -> &[GroupBody<IRAexpr>] {
        self.0.body()
    }

    /// Returns the context associated with this circuit.
    pub fn ctx(&self) -> &IRCtx {
        &self.0.context().0
    }

    /// Returns a printer of the circuit.
    pub fn display(&self) -> IRPrinter<'_> {
        self.0.display()
    }

    /// Returns the main group.
    ///
    /// Panics if there isn't a main group.
    pub fn main(&self) -> &GroupBody<IRAexpr> {
        self.0.main()
    }

    /// Returns the prime that defines the finite field the circuit uses.
    pub fn prime(&self) -> Prime {
        self.0.context().1
    }

    /// Folds the statements if the expressions are constant.
    ///
    /// If any of the statements fails to fold returns an error.
    pub fn constant_fold(&mut self) -> Result<()> {
        self.0.body_mut().constant_fold()?;
        Ok(())
    }

    /// Matches the statements against a series of known patterns and applies rewrites if able to.
    pub fn canonicalize(&mut self) {
        self.0.body_mut().canonicalize();
    }

    /// Validates the IR, returning errors if it failed.
    pub fn validate(&self) -> (Result<()>, Vec<String>) {
        let (res, errors) = self.0.validate();
        (
            res.map_err(|e| {
                anyhow::anyhow!(
                    "Validation of resolved IR failed with {} errors",
                    e.error_count()
                )
            }),
            errors,
        )
    }
}

impl IRPrintable for ResolvedCtx {
    fn fmt(
        &self,
        ctx: &mut haloumi_ir::printer::IRPrinterCtx<'_, '_>,
    ) -> haloumi_ir::printer::Result {
        ctx.list_nl("prime-number", |ctx| write!(ctx, "{}", self.1))
    }
}
