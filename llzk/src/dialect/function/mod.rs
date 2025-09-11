mod ops;

use llzk_sys::mlirGetDialectHandle__llzk__function__;
use melior::dialect::DialectHandle;
pub use ops::{
    call, def, r#return, CallOp, CallOpLike, CallOpRef, FuncDefOp, FuncDefOpLike, FuncDefOpRef,
};

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
