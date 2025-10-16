#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![warn(rustdoc::broken_intra_doc_links)]
#![deny(missing_debug_implementations)]
#![warn(missing_docs)]

use mlir_sys::{
    MlirAffineExpr, MlirAffineMap, MlirAttribute, MlirBlock, MlirContext, MlirDialectHandle,
    MlirDialectRegistry, MlirLocation, MlirLogicalResult, MlirNamedAttribute, MlirOperation,
    MlirPass, MlirRegion, MlirStringRef, MlirType, MlirValue,
};

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

#[cfg(test)]
mod sanity_tests;
