use crate::{
    halo2::{Any, Column, Field},
    ir::{
        generate::RegionByIndex,
        groups::bounds::{Bound, EqConstraintCheck, GroupBounds},
    },
    synthesis::{
        constraint::EqConstraintGraph,
        groups::{Group, GroupCell},
    },
};
use std::collections::VecDeque;

/// List of free cells that need to be binded in a group.
#[derive(Debug, Clone)]
pub struct FreeCells {
    pub inputs: Vec<GroupCell>,
    pub callsites: Vec<Vec<GroupCell>>,
}

/// Check for cells in the equality constraints that are not bounded by IO and are not part of
/// the regions in the group.
/// These are added as part of the inputs of the group and callsites to the group are updated.
/// Then the groups are updated recursivelly.
///
/// Returns a the list of updated input arguments for each callsite and the list of cells that need
/// to be added to the group's inputs.
pub fn lift_free_cells_to_inputs<F: Field>(
    groups: &[Group],
    region_by_index: &RegionByIndex,
    constraints: &EqConstraintGraph<F>,
) -> Vec<FreeCells> {
    let mut result: Vec<_> = groups
        .iter()
        .enumerate()
        .map(|(id, g)| {
            let fc = find_free_cells(g, groups, region_by_index, constraints)
                .into_iter()
                .filter_map(GroupCell::from_tuple)
                .collect::<Vec<_>>();
            if !fc.is_empty() {
                log::debug!(
                    "For group {} \"{}\" we found {} free cells",
                    id,
                    g.name(),
                    fc.len()
                );
            }
            FreeCells {
                inputs: fc,
                callsites: vec![vec![]; g.children_count()],
            }
        })
        .collect();

    // Prime the worklist with the groups that already have elements in `inputs`.
    let mut worklist: VecDeque<_> = result
        .iter()
        .enumerate()
        .filter_map(|(n, r)| {
            if r.inputs.is_empty() {
                return None;
            }
            Some(n)
        })
        .collect();

    log::debug!("Initial worklist: {worklist:?}");
    while !worklist.is_empty() {
        let callee_idx = worklist.pop_front().unwrap();
        log::debug!("Working with group {callee_idx}");

        for (caller_idx, caller) in groups.iter().enumerate() {
            // Extend the callsite to adapt to the new number of inputs.
            let inputs = result[callee_idx].inputs.clone();
            {
                let callsite = match caller.has_child(callee_idx) {
                    Some(callsite) => &mut result[caller_idx].callsites[callsite],
                    None => {
                        continue;
                    }
                };
                log::debug!(
                    "Group {caller_idx} \"{}\" calls {callee_idx} \"{}\"",
                    groups[caller_idx].name(),
                    groups[callee_idx].name()
                );
                callsite.extend(inputs.clone());
            }
            // Check if by extending the callsite we would have new fresh variables
            // and add the caller to the worklist
            let bounds = GroupBounds::new(caller, groups, region_by_index);
            let out_of_bounds: Vec<_> = inputs
                .into_iter()
                .filter(|c| !bounds.within_bounds(&c.col(), &c.row()))
                .collect();
            log::debug!("Out of bounds cells: {:?}", out_of_bounds);
            if out_of_bounds.is_empty() {
                log::debug!("Empty");
                continue;
            }
            result[caller_idx].inputs.extend(out_of_bounds);
            log::debug!("Modified");
            worklist.push_back(caller_idx);
        }
        log::debug!("Worklist after iteration: {worklist:?}");
    }

    result
}

/// Searches for cells in constraints that are not within the bounds of the group but the other
/// side of the equality is.
fn find_free_cells<F: Field>(
    group: &Group,
    groups: &[Group],
    region_by_index: &RegionByIndex,
    constraints: &EqConstraintGraph<F>,
) -> Vec<(Column<Any>, usize)> {
    let bounds = GroupBounds::new(group, groups, region_by_index);

    log::debug!("Search for free cells in '{}' constraints", group.name());
    log::debug!("  Inputs: {:?}", group.inputs());
    log::debug!("  Outputs: {:?}", group.outputs());
    log::debug!("  Bounds: {bounds:?}");
    constraints
        .edges()
        .into_iter()
        .filter_map(|c| match bounds.check_eq_constraint(&c) {
            EqConstraintCheck::AnyToAny(left, (lcol, lrow), right, (rcol, rrow)) => {
                match (left, right) {
                    (Bound::Within, Bound::Outside) => Some((rcol, rrow)),
                    (Bound::Outside, Bound::Within) => Some((lcol, lrow)),
                    //(Bound::IO, Bound::Outside) => Some((rcol, rrow)),
                    //(Bound::Outside, Bound::IO) => Some((lcol, lrow)),
                    _ => None,
                }
            }
            EqConstraintCheck::FixedToConst(_) => None,
        })
        .inspect(|cell| log::debug!("Found free cell: {cell:?}"))
        .collect()
}
