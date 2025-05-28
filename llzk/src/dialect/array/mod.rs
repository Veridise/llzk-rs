mod ops;
#[cfg(test)]
mod tests;
mod r#type;

use llzk_sys::mlirGetDialectHandle__llzk__array__;
use melior::dialect::DialectHandle;
pub use ops::{extract, insert, new, read, write, ArrayCtor};
pub use r#type::ArrayType;

pub fn handle() -> DialectHandle {
    unsafe { DialectHandle::from_raw(mlirGetDialectHandle__llzk__array__()) }
}
