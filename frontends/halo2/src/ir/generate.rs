use std::collections::HashMap;

use crate::{
    expressions::ScopedExpression,
    gates::RewritePatternSet,
    halo2::{Field, PrimeField, RegionIndex},
    ir::{generate::patterns::load_patterns, groups::GroupBody, IRCircuit, IRCtx},
    lookups::callbacks::LookupCallbacks,
    synthesis::{groups::Group, regions::RegionData, CircuitSynthesis},
    GateCallbacks,
};

pub(super) mod free_cells;
pub(crate) mod groups;
pub(crate) mod inline;
mod patterns;

/// Generates an intermediate representation of the circuit from its synthesis.
pub fn generate_ir<'s, 'c: 's, F: PrimeField>(
    syn: &'s CircuitSynthesis<F>,
    lookup_cb: &dyn LookupCallbacks<F>,
    gate_cbs: &dyn GateCallbacks<F>,
    ir_ctx: &'c IRCtx<'s>,
) -> anyhow::Result<IRCircuit<ScopedExpression<'s, 's, F>>> {
    log::debug!("Circuit synthesis has {} gates", syn.gates().len());
    let patterns = load_patterns(gate_cbs);
    let regions_by_index = region_data(syn)?;
    let ctx = GroupIRCtx {
        regions_by_index,
        syn,
        patterns,
        lookup_cb,
    };

    log::debug!("Generating IR of region groups");

    let enumerated_groups = ctx.groups().iter().enumerate().collect::<Vec<_>>();
    let mut regions_to_groups = vec![];

    for (idx, group) in &enumerated_groups {
        for region in group.regions() {
            regions_to_groups.push((region.index().unwrap(), *idx));
        }
    }
    regions_to_groups.sort_by_key(|(ri, _)| **ri);
    debug_assert!(regions_to_groups
        .iter()
        .enumerate()
        .all(|(n, (ri, _))| n == **ri));
    let groups_ir = enumerated_groups
        .into_iter()
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

    Ok(IRCircuit::new(
        groups_ir,
        regions_to_groups
            .into_iter()
            .map(|(_, gidx)| gidx)
            .collect(),
    ))
}

/// Creates a map from region index to its data
#[inline]
pub(super) fn region_data<'s, F: Field>(
    syn: &'s CircuitSynthesis<F>,
) -> anyhow::Result<RegionByIndex<'s>> {
    syn.groups()
        .iter()
        .flat_map(|g| g.regions())
        .map(|r| {
            r.index()
                .map(|i| (i, r))
                .ok_or_else(|| anyhow::anyhow!("Region {r:?} does not have an index"))
        })
        .collect()
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
