use std::marker::PhantomData;

use llzk_sys::MlirValueRange;
use melior::ir::Value;

pub struct ValueRange<'c, 'a, 'b> {
    raw: MlirValueRange,
    _context: PhantomData<&'a [Value<'c, 'b>]>,
}

impl ValueRange<'_, '_, '_> {
    pub fn to_raw(&self) -> MlirValueRange {
        self.raw
    }
}
