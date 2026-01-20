//! `function` dialect.

mod ops;

use llzk_sys::mlirGetDialectHandle__llzk__function__;
use melior::dialect::DialectHandle;
pub use ops::{
    CallOp, CallOpLike, CallOpRef, FuncDefOp, FuncDefOpLike, FuncDefOpMutLike, FuncDefOpRef,
};
pub use ops::{call, def, r#return};
pub use ops::{is_func_call, is_func_def, is_func_return};

/// Returns a handle to the `function` dialect.
pub fn handle() -> DialectHandle {
    unsafe { DialectHandle::from_raw(mlirGetDialectHandle__llzk__function__()) }
}

/// Exports the common types of the func dialect.
pub mod prelude {
    pub use super::ops::{
        CallOp, CallOpLike, CallOpRef, CallOpRefMut, FuncDefOp, FuncDefOpLike, FuncDefOpRef,
        FuncDefOpRefMut,
    };
}
