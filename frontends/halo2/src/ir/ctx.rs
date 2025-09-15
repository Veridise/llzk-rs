use std::collections::HashMap;

use crate::halo2::{Advice, Any, Instance};
use crate::halo2::{Field, RegionIndex};
use crate::io::IOCell;
use crate::ir::generate::free_cells::{lift_free_cells_to_inputs, FreeCells};
use crate::ir::generate::{region_data, RegionByIndex};
use crate::synthesis::groups::{Group, GroupCell};
use crate::synthesis::CircuitSynthesis;
use crate::CircuitIO;

/// Contains information related to the IR of a circuit. Is used by the driver to lower the
/// circuit.
pub struct IRCtx<'s> {
    groups_advice_io: HashMap<usize, crate::io::AdviceIO>,
    groups_instance_io: HashMap<usize, crate::io::InstanceIO>,
    regions_by_index: RegionByIndex<'s>,
    free_cells: Vec<FreeCells>,
}

impl<'s> IRCtx<'s> {
    pub(crate) fn new<F: Field>(syn: &'s CircuitSynthesis<F>) -> anyhow::Result<Self> {
        let regions_by_index = region_data(syn)?;
        let free_cells =
            lift_free_cells_to_inputs(syn.groups(), &regions_by_index, syn.constraints())?;

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

        Ok(Self {
            groups_instance_io,
            groups_advice_io,
            regions_by_index: region_data(syn)?,
            free_cells,
        })
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

    pub(crate) fn regions_by_index(&self) -> &RegionByIndex<'s> {
        &self.regions_by_index
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
                _ => None,
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
                _ => None,
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
