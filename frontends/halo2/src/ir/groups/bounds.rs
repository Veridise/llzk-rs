//! Structs for checking if cells are within the bounds of a group.

use ff::Field;

use crate::{
    ir::generate::RegionByIndex,
    synthesis::{
        constraint::EqConstraint,
        groups::{Group, GroupCell},
    },
};
use halo2_frontend_core::{
    query::Fixed,
    table::{Any, Column},
};
use std::{collections::HashSet, ops::Range};

type ColsAndRows<'a> = Vec<(&'a HashSet<Column<Any>>, Range<usize>)>;

fn cell_within_bounds(cols_and_rows: &ColsAndRows, col: Column<Any>, row: Option<usize>) -> bool {
    cols_and_rows.iter().any(|(columns, rows)| {
        // Check if the column is among the set of columns
        columns.contains(&col) &&
            // If given, check if the row is within range.
            row.map(|row| rows.contains(&row)).unwrap_or(true)
    })
}

/// Can check if a cell is within the bounds of a group.
#[derive(Debug)]
pub struct GroupBounds<'a> {
    cols_and_rows: ColsAndRows<'a>,
    foreign_io: HashSet<(Column<Any>, usize)>,
    io: HashSet<(Column<Any>, usize)>,
    children_output: HashSet<(Column<Any>, usize)>,
}

impl<'a> GroupBounds<'a> {
    /// Creates a new bound for the group.
    pub fn new(group: &'a Group, groups: &'a [Group], region_by_index: &RegionByIndex) -> Self {
        let mut region_indices = HashSet::new();
        let mut cols_and_rows = vec![];
        let mut foreign_io = HashSet::new();
        let mut io = HashSet::new();
        let mut children_output = HashSet::new();
        for region in group.regions() {
            region_indices.insert(*region.index().unwrap());
            cols_and_rows.push((region.columns(), region.rows()));
        }

        for cell in group.inputs().iter().chain(group.outputs()) {
            match cell {
                GroupCell::Assigned(cell) => {
                    // Copy constraints use absolute rows but the labels have relative
                    // rows.
                    if let Some(start) = region_by_index[&cell.region_index].start() {
                        let abs_row = cell.row_offset + start;
                        if region_indices.contains(&cell.region_index) {
                            io.insert((cell.column, abs_row));
                        } else {
                            foreign_io.insert((cell.column, abs_row));
                        }
                    }
                }
                GroupCell::InstanceIO((col, row)) => {
                    foreign_io.insert(((*col).into(), *row));
                }
                GroupCell::AdviceIO((col, row)) => {
                    foreign_io.insert(((*col).into(), *row));
                }
            }
        }
        for cell in group
            .children(groups)
            .into_iter()
            .flat_map(|(_, c)| c.outputs())
        {
            match cell {
                GroupCell::Assigned(cell) => {
                    if let Some(start) = region_by_index[&cell.region_index].start() {
                        let abs_row = cell.row_offset + start;
                        children_output.insert((cell.column, abs_row));
                    }
                }
                GroupCell::InstanceIO(_) => unreachable!(),
                GroupCell::AdviceIO(_) => unreachable!(),
            }
        }

        Self {
            cols_and_rows,
            foreign_io,
            io,
            children_output,
        }
    }

    /// Returns true if the cell is within bounds of the group.
    pub fn within_bounds(&self, col: &Column<Any>, row: &usize) -> bool {
        cell_within_bounds(&self.cols_and_rows, *col, Some(*row))
            || self.is_foreign_io(col, row)
            || self.is_children_output(col, row)
    }

    /// Returns true if the cell is an input or output that is not in the group's regions.
    pub fn is_foreign_io(&self, col: &Column<Any>, row: &usize) -> bool {
        self.foreign_io.contains(&(*col, *row))
    }

    /// Returns true if the cell is introduced by one of the children of the group.
    pub fn is_children_output(&self, col: &Column<Any>, row: &usize) -> bool {
        self.children_output.contains(&(*col, *row))
    }

    /// Returns true if the cell is an input or output that is in the group's regions.
    pub fn is_io(&self, col: &Column<Any>, row: &usize) -> bool {
        self.io.contains(&(*col, *row))
    }

    /// Returns true if the fixed cell is within bounds of the group.
    pub fn fixed_within_regions(&self, col: &Column<Fixed>) -> bool {
        cell_within_bounds(&self.cols_and_rows, (*col).into(), None)
    }

    fn check_cell(&self, col: &Column<Any>, row: &usize) -> Bound {
        if !self.within_bounds(col, row) {
            return Bound::Outside;
        }
        if self.is_foreign_io(col, row) {
            return Bound::ForeignIO;
        }
        if self.is_io(col, row) {
            return Bound::IO;
        }
        Bound::Within
    }

    /// Checks if the equality constraint against the bounds
    pub fn check_eq_constraint<F: Field>(
        &self,
        eq_constraint: &EqConstraint<F>,
    ) -> EqConstraintCheck {
        match eq_constraint {
            EqConstraint::AnyToAny(from, from_row, to, to_row) => EqConstraintCheck::AnyToAny(
                self.check_cell(from, from_row),
                (*from, *from_row),
                self.check_cell(to, to_row),
                (*to, *to_row),
            ),
            EqConstraint::FixedToConst(column, _, _) => {
                EqConstraintCheck::FixedToConst(if self.fixed_within_regions(column) {
                    Bound::Within
                } else {
                    Bound::Outside
                })
            }
        }
    }
}

/// Represents the different positions a cell can be wrt the bounds of a group.
#[derive(Debug)]
pub enum Bound {
    /// The cell is inside the group and is internal.
    Within,
    /// The cell is inside the group and marked as input/output.
    IO,
    /// The cell is not inside the group but was markes as input/output.
    ForeignIO,
    /// The cell is completely outside the group.
    Outside,
}

/// Result of checking a constraint against the bounds of a group.
#[derive(Debug)]
pub enum EqConstraintCheck {
    /// Result of checking `any <-> any` constraints.
    AnyToAny(Bound, (Column<Any>, usize), Bound, (Column<Any>, usize)),
    /// Result of checking `fixed <-> const` constraints.
    FixedToConst(Bound),
}
