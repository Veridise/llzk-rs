use std::{fmt, ops::Deref};

use crate::{
    ir::equivalency::{EqvRelation, SymbolicEqv},
    temps::Temp,
};

/// Argument number of a function
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
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

impl fmt::Debug for ArgNo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "arg{}", self.0)
    }
}

/// An identifier that Backend::FuncOutput will use to identify an output field in the function.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
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

impl Deref for FieldId {
    type Target = usize;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl fmt::Display for FieldId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl fmt::Debug for FieldId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "field{}", self.0)
    }
}

/// Used for comparing cell's offsets.
#[derive(Copy, Clone, Eq, PartialEq)]
enum Offset {
    Rel(usize),
    Abs(usize),
}

/// A reference to a cell in the circuit.
#[derive(Clone, Copy, Eq, PartialOrd, Ord)]
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
        log::debug!("Creating relative reference (col: {col}, base: {base}, offset: {offset})");
        Self {
            col,
            base: Some(base),
            offset,
        }
    }

    /// Returns the absolute row the cell points to.
    pub fn row(&self) -> usize {
        self.base.unwrap_or_default() + self.offset
    }

    /// Returns the offset of the cell.
    fn offset(&self) -> Offset {
        match self.base {
            Some(_) => Offset::Rel(self.offset),
            None => Offset::Abs(self.offset),
        }
    }

    pub fn col(&self) -> usize {
        self.col
    }

    /// Returns true if is an absolute reference.
    pub fn is_absolute(&self) -> bool {
        self.base.is_none()
    }

    /// If the reference is absolute converts the reference into a relative reference wrt the base.
    ///
    /// The base has to be less or equal than the absolute row number.
    ///
    /// If the reference is relative return None.
    pub fn relativize(&self, base: usize) -> Option<Self> {
        match self.offset() {
            Offset::Abs(offset) => {
                if base > offset {
                    return None;
                }
                let offset = offset - base;
                Some(Self::relative(self.col, base, offset))
            }
            Offset::Rel(_) => None,
        }
    }
}

impl std::fmt::Debug for CellRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.base {
            Some(base) => write!(f, "[{}, {base}(+{})]", self.col, self.offset),
            None => write!(f, "[{}, {}]", self.col, self.offset),
        }
    }
}

impl std::fmt::Display for CellRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.base {
            Some(base) => write!(f, "[{},{base}(+{})]", self.col, self.offset),
            None => write!(f, "[{},{}]", self.col, self.offset),
        }
    }
}

impl PartialEq for CellRef {
    fn eq(&self, other: &Self) -> bool {
        self.col() == other.col() && self.row() == other.row()
    }
}

impl std::hash::Hash for CellRef {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.col().hash(state);
        self.row().hash(state);
    }
}

impl EqvRelation<CellRef> for SymbolicEqv {
    /// Two cell refs are equivalent if they point to the same absolute cell or the point to the
    /// same relative cell regardless of their base.
    fn equivalent(lhs: &CellRef, rhs: &CellRef) -> bool {
        lhs.col() == rhs.col()
            && (
                // Either they point to the same cell
                lhs.row() == rhs.row() ||
        // Or they relativelly point to the same cell
        lhs.offset() == rhs.offset()
            )
    }
}

#[derive(Clone, Copy, Hash, Eq, PartialEq, PartialOrd, Ord)]
pub enum FuncIO {
    /// Points to the n-th input argument
    Arg(ArgNo),
    /// Points to the n-th output.
    Field(FieldId),
    /// Points to an advice cell.
    Advice(CellRef),
    /// Points to a fixed cell.
    Fixed(CellRef),
    /// lookup id, column, row, idx, region_idx
    TableLookup(u64, usize, usize, usize, usize),
    /// call output: (call #, output #)
    CallOutput(usize, usize),
    /// Temporary value
    Temp(Temp),
    /// Challenge argument (index, phase, n-th arg)
    Challenge(usize, u8, ArgNo),
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
    /// Advice and fixed cells: equivalent if they point to the same cell relative to their base.
    /// Table lookups: equivalent if they point to the same column and row.
    /// Call outputs: equivalent if they have the same output number.
    fn equivalent(lhs: &FuncIO, rhs: &FuncIO) -> bool {
        match (lhs, rhs) {
            (FuncIO::Arg(lhs), FuncIO::Arg(rhs)) => lhs == rhs,
            (FuncIO::Field(lhs), FuncIO::Field(rhs)) => lhs == rhs,
            (FuncIO::Advice(lhs), FuncIO::Advice(rhs)) => Self::equivalent(lhs, rhs),
            (FuncIO::Fixed(lhs), FuncIO::Fixed(rhs)) => Self::equivalent(lhs, rhs),
            (
                FuncIO::TableLookup(_, col0, row0, _, _),
                FuncIO::TableLookup(_, col1, row1, _, _),
            ) => col0 == col1 && row0 == row1,
            (FuncIO::CallOutput(_, o0), FuncIO::CallOutput(_, o1)) => o0 == o1,
            (FuncIO::Temp(lhs), FuncIO::Temp(rhs)) => lhs == rhs,
            (
                FuncIO::Challenge(lhs_index, lhs_phase, _),
                FuncIO::Challenge(rhs_index, rhs_phase, _),
            ) => lhs_index == rhs_index && lhs_phase == rhs_phase,
            _ => false,
        }
    }
}

impl std::fmt::Debug for FuncIO {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Arg(arg) => write!(f, "{arg:?}"),
            Self::Field(field) => write!(f, "{field:?}"),
            Self::Advice(c) => write!(f, "adv{c:?}"),
            Self::Fixed(c) => write!(f, "fix{c:?}"),
            Self::TableLookup(id, col, row, idx, region_idx) => {
                write!(f, "lookup{id}[{col},{row}]@({idx},{region_idx})")
            }
            Self::CallOutput(call, out) => write!(f, "call{call}->{out}"),
            Self::Temp(id) => write!(f, "t{}", **id),
            Self::Challenge(index, phase, _) => write!(f, "chall{index}@{phase}"),
        }
    }
}

impl std::fmt::Display for FuncIO {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Arg(arg) => write!(f, "{arg}"),
            Self::Field(field) => write!(f, "{field}"),
            Self::Advice(c) => write!(f, "adv{c}"),
            Self::Fixed(c) => write!(f, "fix{c}"),
            Self::TableLookup(id, col, row, idx, region_idx) => {
                write!(f, "lookup{id}[{col},{row}]@({idx},{region_idx})")
            }
            Self::CallOutput(call, out) => write!(f, "call{call}->{out}"),
            Self::Temp(id) => write!(f, "t{}", **id),
            Self::Challenge(index, phase, _) => write!(f, "chall{index}@{phase}"),
        }
    }
}

impl From<Temp> for FuncIO {
    fn from(value: Temp) -> Self {
        Self::Temp(value)
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

#[cfg(test)]
mod tests {
    use std::hash::{DefaultHasher, Hash as _, Hasher as _};

    use log::LevelFilter;
    use quickcheck_macros::quickcheck;
    use simplelog::{Config, TestLogger};

    use super::*;

    /// Tests that a relative reference and an absolute cell that point to the same cell must be
    /// equal and any that do not are not equal.
    #[quickcheck]
    fn same_absolute_and_relative_equal(col: usize, base: usize, offset: usize) -> bool {
        let _ = TestLogger::init(LevelFilter::Debug, Config::default());
        // Ignore tests where there's overflow
        if let None = base.checked_add(offset) {
            return true;
        }
        CellRef::absolute(col, base + offset) == CellRef::relative(col, base, offset)
    }

    /// Tests that a relative reference and an absolute cell that point to the same row in different
    /// columns are not equal.
    #[quickcheck]
    fn diff_col_absolute_and_relative_not_equal(col: usize, base: usize, offset: usize) -> bool {
        let _ = TestLogger::init(LevelFilter::Debug, Config::default());
        // Ignore tests where there's overflow
        if let None = base.checked_add(offset) {
            return true;
        }
        if let None = col.checked_add(1) {
            return true;
        }
        CellRef::absolute(col, base + offset) != CellRef::relative(col + 1, base, offset)
    }

    /// Tests that a relative reference and an absolute cell that point to the same column in different
    /// rows are not equal.
    #[quickcheck]
    fn diff_row_absolute_and_relative_not_equal_1(col: usize, base: usize, offset: usize) -> bool {
        let _ = TestLogger::init(LevelFilter::Debug, Config::default());
        // Ignore tests where there's overflow
        if let None = base.checked_add(offset) {
            return true;
        }
        if let None = (base + offset).checked_add(1) {
            return true;
        }
        CellRef::absolute(col, base + offset) != CellRef::relative(col, base + 1, offset)
    }

    /// Tests that a relative reference and an absolute cell that point to the same column in different
    /// rows are not equal.
    #[quickcheck]
    fn diff_row_absolute_and_relative_not_equal_2(col: usize, base: usize, offset: usize) -> bool {
        let _ = TestLogger::init(LevelFilter::Debug, Config::default());
        // Ignore tests where there's overflow
        if let None = base.checked_add(offset) {
            return true;
        }
        if let None = (base + offset).checked_add(1) {
            return true;
        }
        CellRef::absolute(col, base + offset) != CellRef::relative(col, base, offset + 1)
    }

    fn hash(cell: CellRef) -> u64 {
        let mut h = DefaultHasher::new();
        cell.hash(&mut h);
        h.finish()
    }

    /// Tests that a relative reference and an absolute cell that point to the same cell must be
    /// equal and any that do not are not equal.
    #[quickcheck]
    fn same_absolute_and_relative_equal_hash(col: usize, base: usize, offset: usize) -> bool {
        let _ = TestLogger::init(LevelFilter::Debug, Config::default());
        // Ignore tests where there's overflow
        if let None = base.checked_add(offset) {
            return true;
        }
        hash(CellRef::absolute(col, base + offset)) == hash(CellRef::relative(col, base, offset))
    }

    /// Tests that a relative reference and an absolute cell that point to the same row in different
    /// columns are not equal.
    #[quickcheck]
    fn diff_col_absolute_and_relative_not_equal_hash(
        col: usize,
        base: usize,
        offset: usize,
    ) -> bool {
        let _ = TestLogger::init(LevelFilter::Debug, Config::default());
        // Ignore tests where there's overflow
        if let None = base.checked_add(offset) {
            return true;
        }
        if let None = col.checked_add(1) {
            return true;
        }
        hash(CellRef::absolute(col, base + offset))
            != hash(CellRef::relative(col + 1, base, offset))
    }

    /// Tests that a relative reference and an absolute cell that point to the same column in different
    /// rows are not equal.
    #[quickcheck]
    fn diff_row_absolute_and_relative_not_equal_1_hash(
        col: usize,
        base: usize,
        offset: usize,
    ) -> bool {
        let _ = TestLogger::init(LevelFilter::Debug, Config::default());
        // Ignore tests where there's overflow
        if let None = base.checked_add(offset) {
            return true;
        }
        if let None = base.checked_add(1) {
            return true;
        }
        if let None = base
            .checked_add(1)
            .and_then(|base| base.checked_add(offset))
        {
            return true;
        }
        hash(CellRef::absolute(col, base + offset))
            != hash(CellRef::relative(col, base + 1, offset))
    }

    /// Tests that a relative reference and an absolute cell that point to the same column in different
    /// rows are not equal.
    #[quickcheck]
    fn diff_row_absolute_and_relative_not_equal_2_hash(
        col: usize,
        base: usize,
        offset: usize,
    ) -> bool {
        let _ = TestLogger::init(LevelFilter::Debug, Config::default());
        // Ignore tests where there's overflow
        if let None = base.checked_add(offset) {
            return true;
        }
        if let None = base
            .checked_add(offset)
            .and_then(|base| base.checked_add(1))
        {
            return true;
        }
        if let None = offset.checked_add(1) {
            return true;
        }
        hash(CellRef::absolute(col, base + offset))
            != hash(CellRef::relative(col, base, offset + 1))
    }

    /// Tests that two relative references with the same column and offset are symbolically equivalent.
    #[quickcheck]
    fn same_relative_sym_eqv(col: usize, base: usize, offset: usize) -> bool {
        let _ = TestLogger::init(LevelFilter::Debug, Config::default());
        // Ignore tests where there's overflow
        if let None = base.checked_add(offset) {
            return true;
        }
        if let None = base.checked_add(1) {
            return true;
        }
        if let None = (base + offset).checked_add(1) {
            return true;
        }
        SymbolicEqv::equivalent(
            &CellRef::relative(col, base + 1, offset),
            &CellRef::relative(col, base, offset),
        )
    }
}
