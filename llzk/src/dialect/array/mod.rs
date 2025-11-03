//! `array` dialect.

mod ops;
#[cfg(test)]
mod tests;
mod r#type;

use llzk_sys::mlirGetDialectHandle__llzk__array__;
use melior::dialect::DialectHandle;
pub use ops::{ArrayCtor, extract, insert, new, read, write};
pub use r#type::ArrayType;

/// Returns a handle to the `array` dialect.
pub fn handle() -> DialectHandle {
    unsafe { DialectHandle::from_raw(mlirGetDialectHandle__llzk__array__()) }
}

/// Exports the common types of the array dialect.
pub mod prelude {
    pub use super::r#type::ArrayType;
}
