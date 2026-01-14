//! `struct` dialect.

pub mod helpers;
mod ops;
mod r#type;

use llzk_sys::mlirGetDialectHandle__llzk__component__;
use melior::dialect::DialectHandle;
pub use ops::{
    FieldDefOp, FieldDefOpLike, FieldDefOpRef, StructDefOp, StructDefOpLike, StructDefOpMutLike,
    StructDefOpRef, def, field, new, readf, readf_with_offset, writef,
};
pub use ops::{is_struct_def, is_struct_field, is_struct_new, is_struct_readf, is_struct_writef};
pub use r#type::{StructType, is_struct_type};

/// Returns a handle to the `struct` dialect.
pub fn handle() -> DialectHandle {
    unsafe { DialectHandle::from_raw(mlirGetDialectHandle__llzk__component__()) }
}

/// Exports the common types of the struct dialect.
pub mod prelude {
    pub use super::ops::{
        FieldDefOp, FieldDefOpLike, FieldDefOpRef, FieldDefOpRefMut, StructDefOp, StructDefOpLike,
        StructDefOpMutLike, StructDefOpRef, StructDefOpRefMut,
    };
    pub use super::r#type::{StructType, is_struct_type};
}
