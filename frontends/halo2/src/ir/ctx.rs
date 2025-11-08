use std::collections::{HashMap, HashSet};
use std::ops::Range;

use ff::Field;

use crate::CircuitIO;
use crate::io::IOCell;
use crate::ir::generate::region_data;
use crate::synthesis::SynthesizedCircuit;
use crate::synthesis::groups::GroupCell;
use crate::synthesis::regions::RegionData;
use halo2_frontend_core::query::{Advice, Instance};
use halo2_frontend_core::table::{Any, Column, RegionIndex};

/// Contains information related to the IR of a circuit. Is used by the driver to lower the
/// circuit.
#[derive(Debug, Clone)]
pub struct IRCtx {
    groups_advice_io: HashMap<usize, crate::io::AdviceIO>,
    groups_instance_io: HashMap<usize, crate::io::InstanceIO>,
    advice_cells: HashMap<RegionIndex, AdviceCells>,
}

impl IRCtx {
    pub(crate) fn new<F: Field, E>(syn: &SynthesizedCircuit<F, E>) -> Self {
        let mut groups_advice_io: HashMap<usize, crate::io::AdviceIO> = Default::default();
        let mut groups_instance_io: HashMap<usize, crate::io::InstanceIO> = Default::default();

        let regions = syn.groups().region_starts();
        for (idx, group) in syn.groups().iter().enumerate() {
            let advice_io = mk_advice_io(group.inputs(), group.outputs(), &regions);
            let instance_io = mk_instance_io(group.inputs(), group.outputs(), &regions);

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
        }
    }

    pub(crate) fn advice_io_of_group(&self, idx: usize) -> &crate::io::AdviceIO {
        &self.groups_advice_io[&idx]
    }

    pub(crate) fn instance_io_of_group(&self, idx: usize) -> &crate::io::InstanceIO {
        &self.groups_instance_io[&idx]
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
                .filter(|c| matches!(c.column_type(), Any::Advice))
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

/// Constructs a CircuitIO of advice cells.
fn mk_advice_io(
    inputs: &[GroupCell],
    outputs: &[GroupCell],
    regions: &HashMap<RegionIndex, usize>,
) -> crate::io::AdviceIO {
    let filter_fn = |input: &GroupCell| -> Option<IOCell<Advice>> {
        match input {
            GroupCell::Assigned(cell) => match cell.column.column_type() {
                Any::Advice => {
                    let row = cell.row_offset + regions[&cell.region_index.into()];
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
                    let row = cell.row_offset + regions[&cell.region_index.into()];
                    Some((cell.column.try_into().unwrap(), row))
                }
                Any::Advice => None,
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
