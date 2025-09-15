use crate::{
    halo2::{Any, Column, Field, Fixed},
    ir::RegionByIndex,
    synthesis::{
        constraint::EqConstraint,
        groups::{Group, GroupCell},
    },
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
}

impl<'a> GroupBounds<'a> {
    pub fn new(group: &'a Group, regions_by_index: &RegionByIndex) -> Self {
        Self::new_with_extra(group, regions_by_index, None)
    }

    pub fn new_with_extra(
        group: &'a Group,
        region_by_index: &RegionByIndex,
        extra_inputs: Option<&[GroupCell]>,
    ) -> Self {
        let region_indices: HashSet<_> = group
            .regions()
            .iter()
            .map(|r| *r.index().unwrap())
            .collect();
        let cols_and_rows = group
            .regions()
            .iter()
            .map(|r| (r.columns(), r.rows()))
            .collect();
        let foreign_io: HashSet<_> = std::iter::chain(group.inputs(), group.outputs())
            .chain(extra_inputs.iter().flat_map(|i| *i))
            .filter_map(|i| {
                match i {
                    GroupCell::Assigned(cell) => {
                        if !region_indices.contains(&cell.region_index) {
                            // Copy constraints use absolute rows but the labels have relative
                            // rows.
                            let abs_row =
                                cell.row_offset + region_by_index[&cell.region_index].start()?;
                            Some((cell.column, abs_row))
                        } else {
                            None
                        }
                    }
                    GroupCell::InstanceIO((col, row)) => Some(((*col).into(), *row)),
                    GroupCell::AdviceIO((col, row)) => Some(((*col).into(), *row)),
                }
            })
            .collect();
        let io: HashSet<_> = std::iter::chain(group.inputs(), group.outputs())
            .chain(extra_inputs.iter().flat_map(|i| *i))
            .filter_map(|i| {
                match i {
                    GroupCell::Assigned(cell) => {
                        if region_indices.contains(&cell.region_index) {
                            // Copy constraints use absolute rows but the labels have relative
                            // rows.
                            let abs_row =
                                cell.row_offset + region_by_index[&cell.region_index].start()?;
                            Some((cell.column, abs_row))
                        } else {
                            None
                        }
                    }
                    _ => None,
                }
            })
            .collect();

        Self {
            cols_and_rows,
            foreign_io,
            io,
        }
    }

    pub fn within_bounds(&self, col: &Column<Any>, row: &usize) -> bool {
        cell_within_bounds(&self.cols_and_rows, *col, Some(*row)) || self.is_foreign_io(col, row)
    }

    /// Returns true if the cell is an input or output that is not in the group's regions.
    pub fn is_foreign_io(&self, col: &Column<Any>, row: &usize) -> bool {
        self.foreign_io.contains(&(*col, *row))
    }

    /// Returns true if the cell is an input or output that is in the group's regions.
    pub fn is_io(&self, col: &Column<Any>, row: &usize) -> bool {
        self.io.contains(&(*col, *row))
    }

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

pub enum Bound {
    Within,
    IO,
    ForeignIO,
    Outside,
}

/// Result of checking a constraint against the bounds of a group.
pub enum EqConstraintCheck {
    AnyToAny(Bound, (Column<Any>, usize), Bound, (Column<Any>, usize)),
    FixedToConst(Bound),
}
