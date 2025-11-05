//! Types for working with `ValueRange`s.

use std::marker::PhantomData;

use llzk_sys::MlirValueRange;
use melior::ir::Value;

/// Wrapper around a MLIR `ValueRange`, a non-owned iterator of MLIR values.
#[derive(Debug)]
pub struct ValueRange<'c, 'a, 'b> {
    raw: MlirValueRange,
    _context: PhantomData<&'a [Value<'c, 'b>]>,
}

impl ValueRange<'_, '_, '_> {
    /// Returns the raw representation of the value range.
    pub fn to_raw(&self) -> MlirValueRange {
        self.raw
    }
}
