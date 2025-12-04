//! Generated bindings of LLZK's C API.
//!
//! Follows a similar model to `mlir-sys` and integrates with that crate.

#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![warn(rustdoc::broken_intra_doc_links)]
#![deny(missing_debug_implementations)]
// This lint should never set to `deny` since the functions here depend on code written in the llzk-lib repository.
// It's set to warn as a reminder for the CAPI maintainers to add missing documentation.
#![warn(missing_docs)]

use mlir_sys::{
    MlirAffineExpr, MlirAffineMap, MlirAttribute, MlirBlock, MlirContext, MlirDialectHandle,
    MlirDialectRegistry, MlirLocation, MlirLogicalResult, MlirNamedAttribute, MlirOperation,
    MlirPass, MlirRegion, MlirStringRef, MlirType, MlirValue,
};

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

use std::{ffi::CStr, sync::LazyLock};

/// Macro to create safe Rust string constants for C FFI string constants.
macro_rules! c_str_constant {
    ($(#[$meta:meta])* $const_name:ident, $c_const:ident) => {
        $(#[$meta])*
        pub static $const_name: LazyLock<&'static str> = LazyLock::new(|| {
            unsafe { CStr::from_ptr($c_const) }
                .to_str()
                .expect(concat!(stringify!($c_const), " is valid UTF-8"))
        });
    };
}

c_str_constant!(
    /// Symbol name for the witness generation function within a component.
    FUNC_NAME_COMPUTE,
    LLZK_FUNC_NAME_COMPUTE
);

c_str_constant!(
    /// Symbol name for the constraint generation function within a component.
    FUNC_NAME_CONSTRAIN,
    LLZK_FUNC_NAME_CONSTRAIN
);

c_str_constant!(
    /// Symbol name for the struct/component representing a signal.
    COMPONENT_NAME_SIGNAL,
    LLZK_COMPONENT_NAME_SIGNAL
);

c_str_constant!(
    /// Symbol name for the main entry point struct/component.
    COMPONENT_NAME_MAIN,
    LLZK_COMPONENT_NAME_MAIN
);

c_str_constant!(
    /// Name of the attribute on the top-level ModuleOp that specifies the IR language name.
    LANG_ATTR_NAME,
    LLZK_LANG_ATTR_NAME
);

#[cfg(test)]
mod sanity_tests;
