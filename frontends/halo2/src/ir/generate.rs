//! Logic for generating IR from a synthesized circuit.

use std::collections::HashMap;

use crate::{
    expressions::ScopedExpression,
    gates::{DefaultGateCallbacks, RewritePatternSet},
    halo2::{Field, PrimeField, RegionIndex},
    ir::{generate::patterns::load_patterns, groups::GroupBody, IRCtx},
    lookups::callbacks::{DefaultLookupCallbacks, LookupCallbacks},
    synthesis::{groups::Group, regions::RegionData, CircuitSynthesis},
    GateCallbacks,
};

pub(super) mod free_cells;
mod patterns;

/// Configuration parameters for IR generation.
pub struct IRGenParams<'lc, 'gc, F: Field> {
    debug_comments: bool,
    lookup_cb: Option<&'lc dyn LookupCallbacks<F>>,
    gate_cb: Option<&'gc dyn GateCallbacks<F>>,
}

impl<'lc, 'gc, F: Field> IRGenParams<'lc, 'gc, F> {
    fn new() -> Self {
        Self {
            debug_comments: false,
            lookup_cb: None,
            gate_cb: None,
        }
    }

    /// Returns wether debug comments are enabled or not.
    pub fn debug_comments(&self) -> bool {
        self.debug_comments
    }
}

impl<F: Field> Default for IRGenParams<'_, '_, F> {
    fn default() -> Self {
        Self::new()
    }
}

impl<F: Field> std::fmt::Debug for IRGenParams<'_, '_, F> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IRGenParams")
            .field("debug_comments", &self.debug_comments)
            .field(
                "lookup_cb",
                if self.lookup_cb.is_some() {
                    &"set"
                } else {
                    &"default"
                },
            )
            .field(
                "gate_cb",
                if self.gate_cb.is_some() {
                    &"set"
                } else {
                    &"default"
                },
            )
            .finish()
    }
}

/// Builder for creating [`IRGenParams`].
#[derive(Debug, Default)]
pub struct IRGenParamsBuilder<'lc, 'gc, F: Field>(IRGenParams<'lc, 'gc, F>);

impl<'lc, 'gc, F: Field> IRGenParamsBuilder<'lc, 'gc, F> {
    /// Creates a new builder.
    pub fn new() -> Self {
        Self(IRGenParams::new())
    }

    /// Enables debug comments.
    pub fn with_debug_comments(&mut self) -> &mut Self {
        self.0.debug_comments = true;
        self
    }

    /// Disables debug comments.
    pub fn without_debug_comments(&mut self) -> &mut Self {
        self.0.debug_comments = false;
        self
    }

    /// Sets the lookup callbacks.
    pub fn lookup_callbacks(&mut self, lc: &'lc dyn LookupCallbacks<F>) -> &mut Self {
        self.0.lookup_cb = Some(lc);
        self
    }

    /// Unsets the lookup callbacks.
    pub fn no_lookup_callbacks(&mut self) -> &mut Self {
        self.0.lookup_cb = None;
        self
    }

    /// Sets the gate callbacks.
    pub fn gate_callbacks(&mut self, gc: &'gc dyn GateCallbacks<F>) -> &mut Self {
        self.0.gate_cb = Some(gc);
        self
    }

    /// Unsets the gate callbacks.
    pub fn no_gate_callbacks(&mut self) -> &mut Self {
        self.0.gate_cb = None;
        self
    }

    /// Creates the params.
    pub fn build(&mut self) -> IRGenParams<'lc, 'gc, F> {
        std::mem::take(&mut self.0)
    }
}

/// Generates an intermediate representation of the circuit from its synthesis.
pub fn generate_ir<'syn, 'ctx, 'cb, 'sco, F>(
    syn: &'syn CircuitSynthesis<F>,
    params: IRGenParams<'cb, '_, F>,
    //lookup_cb: &'cb dyn LookupCallbacks<F>,
    //gate_cbs: &dyn GateCallbacks<F>,
    ir_ctx: &'ctx IRCtx,
    //generate_debug_comments: bool,
) -> anyhow::Result<Vec<GroupBody<ScopedExpression<'syn, 'sco, F>>>>
where
    F: PrimeField,
    'syn: 'sco,
    'ctx: 'sco + 'syn,
    'cb: 'sco + 'syn,
{
    log::debug!("Circuit synthesis has {} gates", syn.gates().len());
    let patterns = load_patterns(params.gate_cb.unwrap_or(&DefaultGateCallbacks));
    let regions_by_index = region_data(syn);
    let ctx = GroupIRCtx {
        regions_by_index,
        syn,
        patterns,
        params,
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

pub(super) type RegionByIndex<'s> = HashMap<RegionIndex, RegionData<'s>>;

/// Support data for creating group body IR structs
pub(super) struct GroupIRCtx<'lc, 'gc, 'syn, F: Field> {
    regions_by_index: RegionByIndex<'syn>,
    syn: &'syn CircuitSynthesis<F>,
    patterns: RewritePatternSet<F>,
    params: IRGenParams<'lc, 'gc, F>,
}

impl<'lc, 'gc, 'syn, F: Field> GroupIRCtx<'lc, 'gc, 'syn, F> {
    pub(super) fn groups(&self) -> &'syn [Group] {
        self.syn.groups()
    }

    pub(super) fn regions_by_index(&self) -> &HashMap<RegionIndex, RegionData<'syn>> {
        &self.regions_by_index
    }

    pub(super) fn syn(&self) -> &'syn CircuitSynthesis<F> {
        self.syn
    }

    pub(super) fn patterns(&self) -> &RewritePatternSet<F> {
        &self.patterns
    }

    pub(super) fn lookup_cb(&self) -> &'lc dyn LookupCallbacks<F> {
        self.params.lookup_cb.unwrap_or(&DefaultLookupCallbacks)
    }

    pub(super) fn generate_debug_comments(&self) -> bool {
        self.params.debug_comments
    }
}
