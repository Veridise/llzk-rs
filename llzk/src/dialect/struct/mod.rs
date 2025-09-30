pub mod helpers;
mod ops;
mod r#type;

use llzk_sys::mlirGetDialectHandle__llzk__component__;
use melior::dialect::DialectHandle;
pub use ops::{
    def, field, new, readf, readf_with_offset, writef, FieldDefOp, FieldDefOpLike, FieldDefOpRef,
    StructDefOp, StructDefOpLike, StructDefOpRef,
};
pub use r#type::StructType;

pub fn handle() -> DialectHandle {
    unsafe { DialectHandle::from_raw(mlirGetDialectHandle__llzk__component__()) }
}

/// Exports the common types of the struct dialect.
pub mod prelude {
    pub use super::ops::{
        FieldDefOp, FieldDefOpLike, FieldDefOpRef, FieldDefOpRefMut, StructDefOp, StructDefOpLike,
        StructDefOpRef, StructDefOpRefMut,
    };
    pub use super::r#type::StructType;
}
