use std::ptr::null;

use mlir_sys::{mlirIndexTypeGet, mlirIntegerAttrGet};
use rstest::rstest;

use crate::{
    llzkAttributeIsAFeltConstAttr, llzkFeltConstAttrGet, llzkFeltConstAttrGetFromParts,
    llzkFeltConstAttrGetFromString, llzkFeltTypeGet, llzkTypeIsAFeltType,
    mlirGetDialectHandle__llzk__felt__,
    sanity_tests::{context, str_ref, TestContext},
};

#[test]
fn test_mlir_get_dialect_handle_llzk_felt() {
    unsafe {
        mlirGetDialectHandle__llzk__felt__();
    }
}

#[rstest]
fn test_llzk_felt_const_attr_get(context: TestContext) {
    unsafe {
        let attr = llzkFeltConstAttrGet(context.ctx, 0);
        assert_ne!(attr.ptr, null());
    };
}

#[rstest]
fn test_llzk_felt_const_attr_get_from_str(context: TestContext) {
    unsafe {
        let attr = llzkFeltConstAttrGetFromString(context.ctx, 64, str_ref("123"));
        assert_ne!(attr.ptr, null());
    };
}

#[rstest]
fn test_llzk_felt_const_attr_get_from_parts(context: TestContext) {
    unsafe {
        let parts = [123, 0];
        let attr =
            llzkFeltConstAttrGetFromParts(context.ctx, 128, parts.as_ptr(), parts.len() as isize);
        assert_ne!(attr.ptr, null());
    };
}

#[rstest]
fn test_llzk_attribute_is_a_felt_const_attr_pass(context: TestContext) {
    unsafe {
        let attr = llzkFeltConstAttrGet(context.ctx, 0);
        assert!(llzkAttributeIsAFeltConstAttr(attr));
    };
}

#[rstest]
fn test_llzk_attribute_is_a_felt_const_attr_fail(context: TestContext) {
    unsafe {
        let attr = mlirIntegerAttrGet(mlirIndexTypeGet(context.ctx), 0);
        assert!(!llzkAttributeIsAFeltConstAttr(attr));
    };
}

#[rstest]
fn test_llzk_felt_type_get(context: TestContext) {
    unsafe {
        let r#type = llzkFeltTypeGet(context.ctx);
        assert_ne!(r#type.ptr, null());
    };
}

#[rstest]
fn test_llzk_type_is_a_felt_type_pass(context: TestContext) {
    unsafe {
        let r#type = llzkFeltTypeGet(context.ctx);
        assert!(llzkTypeIsAFeltType(r#type));
    };
}

#[rstest]
fn test_llzk_type_is_a_felt_type_fail(context: TestContext) {
    unsafe {
        let r#type = mlirIndexTypeGet(context.ctx);
        assert!(!llzkTypeIsAFeltType(r#type));
    };
}
