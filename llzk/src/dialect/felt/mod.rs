//! `felt` dialect.

mod attrs;
mod ops;
mod r#type;

pub use attrs::FeltConstAttribute;
use llzk_sys::mlirGetDialectHandle__llzk__felt__;
use melior::dialect::DialectHandle;
pub use ops::{
    add, bit_and, bit_not, bit_or, bit_xor, constant, div, inv, r#mod, mul, neg, pow, shl, shr, sub,
};
pub use ops::{
    is_felt_add, is_felt_bit_and, is_felt_bit_not, is_felt_bit_or, is_felt_bit_xor, is_felt_const,
    is_felt_div, is_felt_inv, is_felt_mod, is_felt_mul, is_felt_neg, is_felt_pow, is_felt_shl,
    is_felt_shr, is_felt_sub,
};
pub use r#type::{FeltType, is_felt_type};

/// Returns a handle to the `felt` dialect.
pub fn handle() -> DialectHandle {
    unsafe { DialectHandle::from_raw(mlirGetDialectHandle__llzk__felt__()) }
}

/// Exports the common types of the felt dialect.
pub mod prelude {
    pub use super::attrs::FeltConstAttribute;
    pub use super::r#type::{FeltType, is_felt_type};
}
