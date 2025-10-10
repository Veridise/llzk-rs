use std::collections::{HashMap, HashSet};
use std::ops::Range;

use crate::halo2::{Advice, Any, Column, Instance};
use crate::halo2::{Field, RegionIndex};
use crate::io::IOCell;
use crate::ir::generate::free_cells::{lift_free_cells_to_inputs, FreeCells};
use crate::ir::generate::region_data;
use crate::synthesis::groups::{Group, GroupCell};
use crate::synthesis::regions::RegionData;
use crate::synthesis::CircuitSynthesis;
use crate::CircuitIO;

/// Contains information related to the IR of a circuit. Is used by the driver to lower the
/// circuit.
#[derive(Debug, Clone)]
pub struct IRCtx {
    groups_advice_io: HashMap<usize, crate::io::AdviceIO>,
    groups_instance_io: HashMap<usize, crate::io::InstanceIO>,
    advice_cells: HashMap<RegionIndex, AdviceCells>,
    free_cells: Vec<FreeCells>,
}

impl IRCtx {
    pub(crate) fn new<F: Field>(syn: &CircuitSynthesis<F>, lift_free_cells: bool) -> Self {
        let regions_by_index = region_data(syn);
        let free_cells = if lift_free_cells {
            lift_free_cells_to_inputs(syn.groups(), &regions_by_index, syn.constraints())
        } else {
            syn.groups().iter().map(FreeCells::empty).collect()
        };

        let mut groups_advice_io: HashMap<usize, crate::io::AdviceIO> = Default::default();
        let mut groups_instance_io: HashMap<usize, crate::io::InstanceIO> = Default::default();

        let regions = syn.groups().region_starts();
        for (idx, group) in syn.groups().iter().enumerate() {
            let mut advice_io = mk_advice_io(group.inputs(), group.outputs(), &regions);
            let mut instance_io = mk_instance_io(group.inputs(), group.outputs(), &regions);
            update_io(&mut advice_io, &mut instance_io, group, &free_cells[idx]);

            groups_advice_io.insert(idx, advice_io);
            groups_instance_io.insert(idx, instance_io);
        }

        Self {
            groups_instance_io,
            groups_advice_io,
            advice_cells: region_data(syn)
                .into_iter()
                .map(|(k, r)| (k, AdviceCells::new(r)))
                .collect(),
            free_cells,
        }
    }

    pub(crate) fn advice_io_of_group(&self, idx: usize) -> &crate::io::AdviceIO {
        &self.groups_advice_io[&idx]
    }

    pub(crate) fn instance_io_of_group(&self, idx: usize) -> &crate::io::InstanceIO {
        &self.groups_instance_io[&idx]
    }

    pub(crate) fn free_cells(&self, idx: usize) -> &FreeCells {
        &self.free_cells[idx]
    }

    pub(crate) fn advice_cells(&self) -> &HashMap<RegionIndex, AdviceCells> {
        &self.advice_cells
    }
}

/// Contains information about the advice cells in a region.
#[derive(Debug, Clone)]
pub(crate) struct AdviceCells {
    columns: HashSet<Column<Any>>,
    rows: Range<usize>,
    start: Option<usize>,
}

impl AdviceCells {
    pub fn new(region: RegionData) -> Self {
        Self {
            columns: region
                .columns()
                .iter()
                .filter(|c| matches!(c.column_type(), Any::Advice(_)))
                .copied()
                .collect(),
            rows: region.rows(),
            start: region.start(),
        }
    }

    /// Returns true if the region contains the given advice cell.
    pub fn contains_advice_cell(&self, col: usize, row: usize) -> bool {
        let in_col_set = self.columns.iter().any(|c| c.index() == col);
        let in_row_range = self.rows.contains(&row);
        in_col_set && in_row_range
    }

    /// Returns the start of the region.
    pub fn start(&self) -> Option<usize> {
        self.start
    }
}

/// If the group has free cells that need to be bounded and is not the top level group
/// makes a copy of its IO and adds the cells as inputs.
fn update_io(
    advice_io: &mut crate::io::AdviceIO,
    instance_io: &mut crate::io::InstanceIO,
    group: &Group,
    free_cells: &FreeCells,
) {
    // Do not update the IO if it's main.
    if group.is_top_level() {
        return;
    }

    for cell in &free_cells.inputs {
        match cell {
            GroupCell::InstanceIO(cell) => instance_io.add_input(*cell),
            GroupCell::AdviceIO(cell) => advice_io.add_input(*cell),
            GroupCell::Assigned(_) => unreachable!(),
        }
    }
}

/// Constructs a CircuitIO of advice cells.
fn mk_advice_io(
    inputs: &[GroupCell],
    outputs: &[GroupCell],
    regions: &HashMap<RegionIndex, usize>,
) -> crate::io::AdviceIO {
    let filter_fn = |input: &GroupCell| -> Option<IOCell<Advice>> {
        match input {
            GroupCell::Assigned(cell) => match cell.column.column_type() {
                Any::Advice(_) => {
                    let row = cell.row_offset + regions[&cell.region_index];
                    Some((cell.column.try_into().unwrap(), row))
                }
                Any::Instance => None,
                Any::Fixed => unreachable!(),
            },
            GroupCell::InstanceIO(_) => None,
            GroupCell::AdviceIO(cell) => Some(*cell),
        }
    };
    CircuitIO::new_from_iocells(
        inputs.iter().filter_map(filter_fn),
        outputs.iter().filter_map(filter_fn),
    )
}

/// Constructs a CircuitIO of instance cells.
fn mk_instance_io(
    inputs: &[GroupCell],
    outputs: &[GroupCell],
    regions: &HashMap<RegionIndex, usize>,
) -> crate::io::InstanceIO {
    let filter_fn = |input: &GroupCell| -> Option<IOCell<Instance>> {
        match input {
            GroupCell::Assigned(cell) => match cell.column.column_type() {
                Any::Instance => {
                    let row = cell.row_offset + regions[&cell.region_index];
                    Some((cell.column.try_into().unwrap(), row))
                }
                Any::Advice(_) => None,
                Any::Fixed => unreachable!(),
            },
            GroupCell::InstanceIO(cell) => Some(*cell),
            GroupCell::AdviceIO(_) => None,
        }
    };
    CircuitIO::new_from_iocells(
        inputs.iter().filter_map(filter_fn),
        outputs.iter().filter_map(filter_fn),
    )
}
