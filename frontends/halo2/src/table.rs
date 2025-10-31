//! Types and traits related to PLONK tables.
//!
//! Some types try to replicate the API of their namesakes in Halo2.

use ff::Field;

use crate::{
    halo2::{Expression, Rotation},
    resolvers::{Advice, Fixed, Instance},
    synthesis::regions::RegionIndex,
};

/// Column type
pub trait ColumnType: std::fmt::Debug + Copy + Clone + PartialEq + Eq + std::hash::Hash {
    /// Constructs a polynomial representing a query to the cell.
    fn query_cell<F: Field>(&self, index: usize, at: Rotation) -> Expression<F>;
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
    fn query_cell<F: Field>(&self, index: usize, at: Rotation) -> Expression<F> {
        // Temporary implementation
        use halo2_proofs::plonk::ColumnType as _;
        match self {
            Any::Fixed => halo2_proofs::plonk::Any::Fixed,
            Any::Advice => halo2_proofs::plonk::Any::Advice(Default::default()),
            Any::Instance => halo2_proofs::plonk::Any::Instance,
        }
        .query_cell(index, halo2_proofs::poly::Rotation(at))
    }
}

/// Temporary implementation
impl From<halo2_proofs::plonk::Any> for Any {
    fn from(value: halo2_proofs::plonk::Any) -> Self {
        match value {
            halo2_proofs::plonk::Any::Advice(_) => Self::Advice,
            halo2_proofs::plonk::Any::Fixed => Self::Fixed,
            halo2_proofs::plonk::Any::Instance => Self::Instance,
        }
    }
}

/// Temporary implementation
impl From<halo2_proofs::plonk::Fixed> for Any {
    fn from(_: halo2_proofs::plonk::Fixed) -> Self {
        Self::Fixed
    }
}

/// Temporary implementation
impl From<halo2_proofs::plonk::Advice> for Any {
    fn from(_: halo2_proofs::plonk::Advice) -> Self {
        Self::Advice
    }
}

/// Temporary implementation
impl From<halo2_proofs::plonk::Instance> for Any {
    fn from(_: halo2_proofs::plonk::Instance) -> Self {
        Self::Instance
    }
}

impl ColumnType for Fixed {
    fn query_cell<F: Field>(&self, index: usize, at: Rotation) -> Expression<F> {
        // Temporary implementation
        use halo2_proofs::plonk::ColumnType as _;
        halo2_proofs::plonk::Fixed.query_cell(index, halo2_proofs::poly::Rotation(at))
    }
}

/// Temporary implementation
impl From<halo2_proofs::plonk::Fixed> for Fixed {
    fn from(_: halo2_proofs::plonk::Fixed) -> Self {
        Self
    }
}

impl ColumnType for Advice {
    fn query_cell<F: Field>(&self, index: usize, at: Rotation) -> Expression<F> {
        // Temporary implementation
        use halo2_proofs::plonk::ColumnType as _;
        halo2_proofs::plonk::Advice::default().query_cell(index, halo2_proofs::poly::Rotation(at))
    }
}

/// Temporary implementation
impl From<halo2_proofs::plonk::Advice> for Advice {
    fn from(_: halo2_proofs::plonk::Advice) -> Self {
        Self
    }
}

impl ColumnType for Instance {
    fn query_cell<F: Field>(&self, index: usize, at: Rotation) -> Expression<F> {
        // Temporary implementation
        use halo2_proofs::plonk::ColumnType as _;
        halo2_proofs::plonk::Instance.query_cell(index, halo2_proofs::poly::Rotation(at))
    }
}

/// Temporary implementation
impl From<halo2_proofs::plonk::Instance> for Instance {
    fn from(_: halo2_proofs::plonk::Instance) -> Self {
        Self
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
    pub fn query_cell<F: Field>(&self, at: Rotation) -> Expression<F> {
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

/// Temporary implementation
impl<FC: halo2_proofs::plonk::ColumnType + Into<TC>, TC: ColumnType>
    From<halo2_proofs::plonk::Column<FC>> for Column<TC>
{
    fn from(value: halo2_proofs::plonk::Column<FC>) -> Self {
        Self {
            index: value.index(),
            column_type: (*value.column_type()).into(),
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
