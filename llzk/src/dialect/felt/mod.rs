mod attrs;
mod ops;
mod r#type;

pub use attrs::{FeltConstAttribute, Radix};
use llzk_sys::mlirGetDialectHandle__llzk__felt__;
use melior::dialect::DialectHandle;
pub use ops::{add, constant, mul, neg, sub};
pub use r#type::FeltType;

pub fn handle() -> DialectHandle {
    unsafe { DialectHandle::from_raw(mlirGetDialectHandle__llzk__felt__()) }
}
