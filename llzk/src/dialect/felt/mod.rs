//! `felt` dialect.

mod attrs;
mod ops;
mod r#type;

pub use attrs::{FeltConstAttribute /*, Radix*/};
use llzk_sys::mlirGetDialectHandle__llzk__felt__;
use melior::dialect::DialectHandle;
pub use ops::{
    add, bit_and, bit_not, bit_or, bit_xor, constant, div, inv, r#mod, mul, neg, shl, shr, sub,
};
pub use r#type::FeltType;

/// Returns a handle to the `felt` dialect.
pub fn handle() -> DialectHandle {
    unsafe { DialectHandle::from_raw(mlirGetDialectHandle__llzk__felt__()) }
}

/// Exports the common types of the felt dialect.
pub mod prelude {
    pub use super::attrs::FeltConstAttribute;
    pub use super::r#type::FeltType;
}
