//! Types and traits related to PLONK tables.
//!
//! Some types try to replicate the API of their namesakes in Halo2.

use std::ops::Deref;

use ff::Field;

use crate::{
    expressions::ExprBuilder,
    info_traits::CreateQuery,
    query::{Advice, Fixed, Instance},
};

/// Column type
pub trait ColumnType: std::fmt::Debug + Copy + Clone + PartialEq + Eq + std::hash::Hash {
    /// Constructs a polynomial representing a query to the cell.
    fn query_cell<F: Field, E: ExprBuilder<F>>(&self, index: usize, at: Rotation) -> E;
}

/// Erased column type.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, PartialOrd, Ord)]
pub enum Any {
    /// Fixed type.
    Fixed,
    /// Advice type.
    Advice,
    /// Instance type.
    Instance,
}

impl ColumnType for Any {
    fn query_cell<F: Field, E: ExprBuilder<F>>(&self, index: usize, at: Rotation) -> E {
        match self {
            Any::Fixed => Fixed.query_cell(index, at),
            Any::Advice => Advice.query_cell(index, at),
            Any::Instance => Instance.query_cell(index, at),
        }
    }
}

impl ColumnType for Fixed {
    fn query_cell<F: Field, E: ExprBuilder<F>>(&self, index: usize, at: Rotation) -> E {
        E::FixedQuery::query_expr(index, at)
    }
}

impl ColumnType for Advice {
    fn query_cell<F: Field, E: ExprBuilder<F>>(&self, index: usize, at: Rotation) -> E {
        E::AdviceQuery::query_expr(index, at)
    }
}

impl ColumnType for Instance {
    fn query_cell<F: Field, E: ExprBuilder<F>>(&self, index: usize, at: Rotation) -> E {
        E::InstanceQuery::query_expr(index, at)
    }
}

/// A column with a type.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Column<C: ColumnType> {
    index: usize,
    column_type: C,
}

impl<C: ColumnType> Column<C> {
    /// Creates a new column.
    pub fn new(index: usize, column_type: C) -> Self {
        Self { index, column_type }
    }

    /// Returns the index of a column.
    pub fn index(&self) -> usize {
        self.index
    }

    /// Returns the column type.
    pub fn column_type(&self) -> &C {
        &self.column_type
    }

    /// Creates an expression representing a query to a cell in this column.
    pub fn query_cell<F: Field, E: ExprBuilder<F>>(&self, at: Rotation) -> E {
        self.column_type.query_cell(self.index, at)
    }
}

impl From<Column<Fixed>> for Column<Any> {
    fn from(value: Column<Fixed>) -> Self {
        Self {
            index: value.index,
            column_type: Any::Fixed,
        }
    }
}

impl TryFrom<Column<Any>> for Column<Fixed> {
    type Error = anyhow::Error;

    fn try_from(value: Column<Any>) -> Result<Self, Self::Error> {
        match value.column_type {
            Any::Fixed => Ok(Self {
                index: value.index,
                column_type: Fixed,
            }),
            c => Err(anyhow::anyhow!("Expected Any::Fixed. Got {c:?}")),
        }
    }
}

impl From<Column<Advice>> for Column<Any> {
    fn from(value: Column<Advice>) -> Self {
        Self {
            index: value.index,
            column_type: Any::Advice,
        }
    }
}

impl TryFrom<Column<Any>> for Column<Advice> {
    type Error = anyhow::Error;

    fn try_from(value: Column<Any>) -> Result<Self, Self::Error> {
        match value.column_type {
            Any::Advice => Ok(Self {
                index: value.index,
                column_type: Advice,
            }),
            c => Err(anyhow::anyhow!("Expected Any::Advice. Got {c:?}")),
        }
    }
}

impl From<Column<Instance>> for Column<Any> {
    fn from(value: Column<Instance>) -> Self {
        Self {
            index: value.index,
            column_type: Any::Instance,
        }
    }
}

impl TryFrom<Column<Any>> for Column<Instance> {
    type Error = anyhow::Error;

    fn try_from(value: Column<Any>) -> Result<Self, Self::Error> {
        match value.column_type {
            Any::Instance => Ok(Self {
                index: value.index,
                column_type: Instance,
            }),
            c => Err(anyhow::anyhow!("Expected Any::Instance. Got {c:?}")),
        }
    }
}

/// Represents a cell in the table.
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub struct Cell {
    /// The index of the region this cell belongs to.
    pub region_index: RegionIndex,
    /// Offset relative to the region of the cell's row.
    pub row_offset: usize,
    /// The cell's column.
    pub column: Column<Any>,
}

/// Replacement type for Halo2's `Rotation` type.
pub type Rotation = i32;

/// Extension methods for [`Rotation`].
pub trait RotationExt {
    /// Returns the current row.
    fn cur() -> Self;

    /// Returns the next row.
    fn next() -> Self;

    /// Returns the previous row.
    fn prev() -> Self;
}

impl RotationExt for Rotation {
    fn cur() -> Self {
        0
    }

    fn next() -> Self {
        1
    }

    fn prev() -> Self {
        -1
    }
}

/// Replacement for Halo2's `RegionIndex` type.
#[derive(Eq, Hash, PartialEq, Debug, Copy, Clone)]
pub struct RegionIndex(usize);

impl Deref for RegionIndex {
    type Target = usize;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<usize> for RegionIndex {
    fn from(value: usize) -> Self {
        Self(value)
    }
}
