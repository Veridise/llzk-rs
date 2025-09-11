use crate::{
    backend::{
        codegen::{
            strats::{
                groups::body::{organize_groups_by_key, GroupBody},
                load_patterns,
            },
            Codegen, CodegenStrategy,
        },
        resolvers::ResolversProvider,
    },
    gates::RewritePatternSet,
    halo2::{Field, RegionIndex},
    ir::equivalency::{EqvRelation, SymbolicEqv},
    lookups::callbacks::LookupCallbacks,
    synthesis::{
        groups::Group,
        regions::{RegionData, RegionRow, Row},
        CircuitSynthesis,
    },
    utils, GateCallbacks,
};
use anyhow::Result;
use std::collections::{HashMap, HashSet};

mod body;
mod bounds;
mod callsite;
mod free_cells;

/// Code generation strategy that write the code of each group in a separate function.
#[derive(Default)]
pub struct GroupConstraintsStrat {}

impl CodegenStrategy for GroupConstraintsStrat {
    fn codegen<'c: 'st, 's, 'st, C>(
        &self,
        codegen: &C,
        syn: &'s CircuitSynthesis<C::F>,
        lookup_cb: &dyn LookupCallbacks<C::F>,
        gate_cbs: &dyn GateCallbacks<C::F>,
    ) -> Result<()>
    where
        C: Codegen<'c, 'st>,
        Row<'s, 's, C::F>: ResolversProvider<C::F> + 's,
        RegionRow<'s, 's, 's, C::F>: ResolversProvider<C::F> + 's,
    {
        log::debug!("Circuit synthesis has {} gates", syn.gates().len());
        let patterns = load_patterns(gate_cbs);
        let regions = region_data(syn)?;
        let ctx = GroupIRCtx {
            groups: syn.groups(),
            regions_by_index: &regions,
            syn,
            patterns: &patterns,
            lookup_cb,
        };

        log::debug!("Generating step 1 IR of region groups");

        let free_cells = free_cells::lift_free_cells_to_inputs(syn.groups(), ctx)?;

        let mut groups_ir = ctx
            .groups
            .iter()
            .enumerate()
            .map(|(id, g)| GroupBody::new(g, id, ctx, &free_cells[id]))
            .collect::<Result<Vec<_>, _>>()?;

        // Sanity check, only one group should be considered main.
        assert_eq!(
            groups_ir.iter().filter(|g| g.is_main()).count(),
            1,
            "Only one main group is allowed"
        );
        // Select leaders and generate the final names.
        // If the group was renamed its index will contain Some(_).
        let (leaders, updated_calldata) = select_leaders(&groups_ir);

        log::debug!("Leaders for the non-main groups: {leaders:?}");
        // Build the final list of IR and invoke codegen
        groups_ir.retain_mut(|g| {
            // Keep a group if its main or is in the leaders list.
            let keep = g.is_main() || leaders.contains(&g.id());
            if keep {
                // If we are keeping it update the names if necessary.
                update_names(g, &updated_calldata)
            }
            keep
        });

        // Create a function per group.
        for group in groups_ir {
            log::debug!("Group {group:#?}");
            if group.is_main() {
                log::debug!("Generating main body");
                codegen.within_main(syn, move |_| Ok([group]))?;
            } else {
                log::debug!("Generating body of function {}", group.name());
                let name = group.name().to_owned();
                codegen.define_function_with_body(
                    &name,
                    group.io().0,
                    group.io().1,
                    |_, _, _| Ok([group]),
                )?;
            }
        }
        Ok(())
    }
}

/// Creates a map from region index to its data
fn region_data<'s, F: Field>(
    syn: &'s CircuitSynthesis<F>,
) -> anyhow::Result<HashMap<RegionIndex, RegionData<'s>>> {
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

/// Find the leaders of each equivalence class in the groups and annotate the required renames
fn select_leaders<F: Field>(groups_ir: &[GroupBody<F>]) -> (Vec<usize>, Vec<Option<String>>) {
    // Separate the groups by their key.
    let groups_by_key = organize_groups_by_key(groups_ir);
    log::debug!("Groups: {groups_by_key:?}");
    let mut leaders = vec![];
    // For each group annotate its new name if it needs to be renamed.
    let mut updated_calldata: Vec<Option<String>> = vec![None; groups_ir.len()];
    // Avoids duplicating names
    let mut used_names: HashSet<String> = HashSet::default();
    // For each set of groups with the same key we create equivalence classes and select a
    // leader for each class.
    let mut eqv_class = disjoint::DisjointSet::new();
    // Keeps track of the inserted elements in the equivalence class.
    let eqv_class_ids: Vec<_> = (0..groups_ir.len())
        .map(|_| eqv_class.add_singleton())
        .collect();
    for groups in groups_by_key.values() {
        // Find the equivalence classes.
        for (i, j) in utils::product(groups.as_slice(), groups.as_slice()) {
            if *i == *j {
                continue;
            }
            let lhs = &groups_ir[*i];
            let rhs = &groups_ir[*j];
            if SymbolicEqv::equivalent(lhs, rhs) {
                eqv_class.join(eqv_class_ids[*i], eqv_class_ids[*j]);
            }
        }
    }

    // Flip the mapping between ids to recover them.
    let eqv_class_ids: HashMap<_, _> = eqv_class_ids
        .into_iter()
        .enumerate()
        .map(|(n, id)| (id, n))
        .collect();

    // For each group eqv class select a leader and annotate what groups need to be updated.
    for (n, set) in eqv_class.sets().into_iter().enumerate() {
        debug_assert!(!set.is_empty());
        let set: Vec<_> = set.into_iter().map(|id| eqv_class_ids[&id]).collect();
        // We arbitrarily chose the leader to be the first element.
        leaders.push(set[0]);
        let leader = groups_ir.get(set[0]).unwrap();
        let name = fresh_group_name(leader.name(), &mut used_names, n);
        for update in &set {
            updated_calldata[*update] = Some(name.clone());
        }
    }

    (leaders, updated_calldata)
}

/// Updates the name of the group and the names of the functions each callsite references.
fn update_names<F: Field>(group: &mut GroupBody<F>, updated_calldata: &[Option<String>]) {
    if let Some(name) = &updated_calldata[group.id()] {
        *group.name_mut() = name.clone();
    }
    for callsite in group.callsites_mut() {
        if let Some(name) = &updated_calldata[callsite.callee_id()] {
            callsite.set_name(name.clone());
        }
    }
}

/// Finds a version of the given name that is fresh.
fn fresh_group_name(name: &str, used_names: &mut HashSet<String>, n: usize) -> String {
    // Create a lazy iterator with the input name and every rename and then consume it until we get
    // a valid name.
    let name = std::iter::chain([name.to_owned()], (n..).map(|n| format!("{name}{n}")))
        .find_map(|name| {
            if used_names.contains(&name) {
                return None;
            }
            Some(name)
        })
        .unwrap();
    used_names.insert(name.clone());
    name
}

/// Support data for creating group body IR structs
#[derive(Copy, Clone)]
struct GroupIRCtx<'g, 's, F: Field> {
    groups: &'g [Group],
    regions_by_index: &'g HashMap<RegionIndex, RegionData<'s>>,
    syn: &'s CircuitSynthesis<F>,
    patterns: &'g RewritePatternSet<F>,
    lookup_cb: &'g dyn LookupCallbacks<F>,
}
