use std::ptr::null;

use mlir_sys::mlirIndexTypeGet;
use rstest::rstest;

use crate::{
    llzkStringTypeGet, llzkTypeIsAStringType, mlirGetDialectHandle__llzk__string__,
    sanity_tests::{TestContext, context},
};

#[test]
fn test_mlir_get_dialect_handle_llzk_string() {
    unsafe {
        mlirGetDialectHandle__llzk__string__();
    }
}

#[rstest]
fn test_llzk_string_type_get(context: TestContext) {
    unsafe {
        let r#type = llzkStringTypeGet(context.ctx);
        assert_ne!(r#type.ptr, null());
    };
}

#[rstest]
fn test_llzk_type_is_a_string_type_pass(context: TestContext) {
    unsafe {
        let r#type = llzkStringTypeGet(context.ctx);
        assert!(llzkTypeIsAStringType(r#type));
    };
}

#[rstest]
fn test_llzk_type_is_a_string_type_fail(context: TestContext) {
    unsafe {
        let r#type = mlirIndexTypeGet(context.ctx);
        assert!(!llzkTypeIsAStringType(r#type));
    };
}
