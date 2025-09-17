//! Logic for generating IR from a synthesized circuit.

use std::collections::HashMap;

use crate::{
    expressions::ScopedExpression,
    gates::RewritePatternSet,
    halo2::{Field, PrimeField, RegionIndex},
    ir::{generate::patterns::load_patterns, groups::GroupBody, stmt::IRStmt, IRCtx},
    lookups::callbacks::LookupCallbacks,
    synthesis::{groups::Group, regions::RegionData, CircuitSynthesis},
    GateCallbacks,
};

pub(super) mod free_cells;
pub(crate) mod lookup;
mod patterns;

/// Generates an intermediate representation of the circuit from its synthesis.
pub fn generate_ir<'syn, 'ctx, 'cb, 'sco, F>(
    syn: &'syn CircuitSynthesis<F>,
    lookup_cb: &'cb dyn LookupCallbacks<F>,
    gate_cbs: &dyn GateCallbacks<F>,
    ir_ctx: &'ctx IRCtx,
) -> anyhow::Result<Vec<GroupBody<ScopedExpression<'syn, 'sco, F>>>>
where
    F: PrimeField,
    'syn: 'sco,
    'ctx: 'sco + 'syn,
    'cb: 'sco + 'syn,
{
    log::debug!("Circuit synthesis has {} gates", syn.gates().len());
    let patterns = load_patterns(gate_cbs);
    let regions_by_index = region_data(syn);
    let ctx = GroupIRCtx {
        regions_by_index,
        syn,
        patterns,
        lookup_cb,
    };

    log::debug!("Generating IR of region groups");

    let groups_ir = ctx
        .groups()
        .iter()
        .enumerate()
        .map(|(id, g)| {
            GroupBody::new(
                g,
                id,
                &ctx,
                ir_ctx.free_cells(id),
                ir_ctx.advice_io_of_group(id),
                ir_ctx.instance_io_of_group(id),
            )
        })
        .collect::<Result<Vec<_>, _>>()?;

    // Sanity check, only one group should be considered main.
    assert_eq!(
        groups_ir.iter().filter(|g| g.is_main()).count(),
        1,
        "Only one main group is allowed"
    );

    Ok(groups_ir)
}

/// Creates a map from region index to its data
#[inline]
pub(super) fn region_data<'s, F: Field>(syn: &'s CircuitSynthesis<F>) -> RegionByIndex<'s> {
    syn.groups()
        .iter()
        .flat_map(|g| g.regions())
        .map(|r| {
            r.index()
                .map(|i| (i, r))
                .unwrap_or_else(|| panic!("Region {r:?} does not have an index"))
        })
        .collect()
}

/// If the given statement is not empty prepends a comment
/// with contextual information.
#[inline]
fn prepend_comment<'a, F: Field>(
    stmt: IRStmt<ScopedExpression<'a, 'a, F>>,
    comment: impl FnOnce() -> IRStmt<ScopedExpression<'a, 'a, F>>,
) -> IRStmt<ScopedExpression<'a, 'a, F>> {
    if stmt.is_empty() {
        return stmt;
    }
    [comment(), stmt].into_iter().collect()
}

pub(super) type RegionByIndex<'s> = HashMap<RegionIndex, RegionData<'s>>;

/// Support data for creating group body IR structs
pub(super) struct GroupIRCtx<'cb, 's, F: Field> {
    regions_by_index: RegionByIndex<'s>,
    syn: &'s CircuitSynthesis<F>,
    patterns: RewritePatternSet<F>,
    lookup_cb: &'cb dyn LookupCallbacks<F>,
}

impl<'cb, 's, F: Field> GroupIRCtx<'cb, 's, F> {
    pub(super) fn groups(&self) -> &'s [Group] {
        self.syn.groups()
    }

    pub(super) fn regions_by_index(&self) -> &HashMap<RegionIndex, RegionData<'s>> {
        &self.regions_by_index
    }

    pub(super) fn syn(&self) -> &'s CircuitSynthesis<F> {
        self.syn
    }

    pub(super) fn patterns(&self) -> &RewritePatternSet<F> {
        &self.patterns
    }

    pub(super) fn lookup_cb(&self) -> &'cb dyn LookupCallbacks<F> {
        self.lookup_cb
    }
}
