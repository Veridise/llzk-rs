mod ops;

use llzk_sys::mlirGetDialectHandle__llzk__function__;
use melior::dialect::DialectHandle;
pub use ops::{
    CallOp, CallOpLike, CallOpRef, FuncDefOp, FuncDefOpLike, FuncDefOpMutLike, FuncDefOpRef, call,
    def, r#return,
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
