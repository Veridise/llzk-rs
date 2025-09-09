use std::{fmt, ops::Deref};

use crate::ir::equivalency::{EqvRelation, SymbolicEqv};

/// Argument number of a function
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ArgNo(usize);

impl From<usize> for ArgNo {
    fn from(value: usize) -> Self {
        Self(value)
    }
}

impl Deref for ArgNo {
    type Target = usize;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl ArgNo {
    pub fn offset_by(self, offset: usize) -> Self {
        Self(self.0 + offset)
    }
}

impl fmt::Display for ArgNo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// An identifier that Backend::FuncOutput will use to identify an output field in the function.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FieldId(usize);

impl From<usize> for FieldId {
    fn from(value: usize) -> Self {
        Self(value)
    }
}

impl FieldId {
    pub fn offset_by(self, offset: usize) -> Self {
        Self(self.0 + offset)
    }
}

impl fmt::Display for FieldId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A reference to a cell in the circuit.
#[derive(Clone, Copy, Hash, Eq, PartialEq, Debug)]
pub struct CellRef {
    col: usize,
    base: Option<usize>,
    offset: usize,
}

impl CellRef {
    pub fn absolute(col: usize, row: usize) -> Self {
        Self {
            col,
            base: None,
            offset: row,
        }
    }

    pub fn relative(col: usize, base: usize, offset: usize) -> Self {
        Self {
            col,
            base: Some(base),
            offset,
        }
    }

    /// Resolves the row this reference points to.
    pub fn row(&self) -> usize {
        self.base.unwrap_or_default() + self.offset
    }

    pub fn col(&self) -> usize {
        self.col
    }
}

#[derive(Clone, Copy, Hash, Eq, PartialEq, Debug)]
pub enum FuncIO {
    /// Points to the n-th input argument
    Arg(ArgNo),
    /// Points to the n-th output.
    Field(FieldId),
    /// Points to an advice cell.
    Advice(CellRef),
    /// Points to a fixed cell.
    Fixed(CellRef),
    // lookup id, column, row, idx, region_idx
    TableLookup(u64, usize, usize, usize, usize),
    // call output: (call #, output #)
    CallOutput(usize, usize),
}

impl FuncIO {
    pub fn advice_abs(col: usize, row: usize) -> Self {
        Self::Advice(CellRef::absolute(col, row))
    }

    pub fn advice_rel(col: usize, base: usize, offset: usize) -> Self {
        Self::Advice(CellRef::relative(col, base, offset))
    }

    pub fn fixed_abs(col: usize, row: usize) -> Self {
        Self::Fixed(CellRef::absolute(col, row))
    }

    pub fn fixed_rel(col: usize, base: usize, offset: usize) -> Self {
        Self::Fixed(CellRef::relative(col, base, offset))
    }
}

impl EqvRelation<FuncIO> for SymbolicEqv {
    /// Two FuncIOs are symbolically equivalent if they refer to the data regardless of how is
    /// pointed to.
    ///
    /// Arguments and fields:  equivalent if they refer to the same offset.
    /// Advice and fixed cells: equivalent if they point to the same cell.
    /// Table lookups: equivalent if they point to the same column and row.
    /// Call outputs: equivalent if they have the same output number.
    fn equivalent(lhs: &FuncIO, rhs: &FuncIO) -> bool {
        match (lhs, rhs) {
            (FuncIO::Arg(lhs), FuncIO::Arg(rhs)) => lhs == rhs,
            (FuncIO::Field(lhs), FuncIO::Field(rhs)) => lhs == rhs,
            (FuncIO::Advice(lhs), FuncIO::Advice(rhs)) => {
                lhs.col() == rhs.col() && lhs.row() == rhs.row()
            }
            (FuncIO::Fixed(lhs), FuncIO::Fixed(rhs)) => {
                lhs.col() == rhs.col() && lhs.row() == rhs.row()
            }
            (
                FuncIO::TableLookup(_, col0, row0, _, _),
                FuncIO::TableLookup(_, col1, row1, _, _),
            ) => col0 == col1 && row0 == row1,
            (FuncIO::CallOutput(_, o0), FuncIO::CallOutput(_, o1)) => o0 == o1,
            _ => false,
        }
    }
}

impl From<ArgNo> for FuncIO {
    fn from(value: ArgNo) -> Self {
        Self::Arg(value)
    }
}

impl From<FieldId> for FuncIO {
    fn from(value: FieldId) -> Self {
        Self::Field(value)
    }
}

//impl From<(usize, usize)> for FuncIO {
//    fn from(value: (usize, usize)) -> Self {
//        Self::Advice(value.0, value.1)
//    }
//}
