//! `poly` dialect.

pub mod ops;
pub mod r#type;
pub use ops::{applymap, read_const, unifiable_cast};
pub use ops::{is_applymap_op, is_read_const_op, is_unifiable_cast_op};

use llzk_sys::mlirGetDialectHandle__llzk__polymorphic__;
use melior::dialect::DialectHandle;

/// Returns a handle to the `poly` dialect.
pub fn handle() -> DialectHandle {
    unsafe { DialectHandle::from_raw(mlirGetDialectHandle__llzk__polymorphic__()) }
}
