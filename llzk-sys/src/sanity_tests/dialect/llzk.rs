use std::ptr::null;

use rstest::rstest;

use crate::{
    llzkAttributeIsAPublicAttr, llzkPublicAttrGet, mlirGetDialectHandle__llzk__,
    sanity_tests::{TestContext, context},
};

#[test]
fn test_mlir_get_dialect_handle_llzk() {
    unsafe {
        mlirGetDialectHandle__llzk__();
    }
}

#[rstest]
fn test_llzk_public_attr_get(context: TestContext) {
    unsafe {
        let attr = llzkPublicAttrGet(context.ctx);
        assert_ne!(attr.ptr, null());
    };
}

#[rstest]
fn test_llzk_attribute_is_a_public_attr_pass(context: TestContext) {
    unsafe {
        let attr = llzkPublicAttrGet(context.ctx);
        assert!(llzkAttributeIsAPublicAttr(attr));
    };
}
