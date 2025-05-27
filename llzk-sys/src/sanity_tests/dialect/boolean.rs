use crate::{
    llzkAttributeIsAFeltCmpPredicateAttr, llzkFeltCmpPredicateAttrGet,
    mlirGetDialectHandle__llzk__boolean__,
    sanity_tests::{context, TestContext},
    LlzkCmp,
};
use mlir_sys::mlirUnitAttrGet;
use rstest::rstest;
use std::ptr::null;

#[test]
fn test_mlir_get_dialect_handle_llzk_boolean() {
    unsafe {
        mlirGetDialectHandle__llzk__boolean__();
    }
}

#[rstest]
fn test_llzk_felt_cmp_predicate_attr_get(
    context: TestContext,
    #[values(
        crate::LlzkCmp_LlzkCmp_EQ,
        crate::LlzkCmp_LlzkCmp_NE,
        crate::LlzkCmp_LlzkCmp_LT,
        crate::LlzkCmp_LlzkCmp_LE,
        crate::LlzkCmp_LlzkCmp_GT,
        crate::LlzkCmp_LlzkCmp_GE
    )]
    cmp: LlzkCmp,
) {
    unsafe {
        let attr = llzkFeltCmpPredicateAttrGet(context.ctx, cmp);
        assert_ne!(attr.ptr, null());
    }
}

#[rstest]
fn test_llzk_attribute_is_a_felt_cmp_predicate_attr_pass(context: TestContext) {
    unsafe {
        let attr = llzkFeltCmpPredicateAttrGet(context.ctx, 0);
        assert!(llzkAttributeIsAFeltCmpPredicateAttr(attr));
    }
}

#[rstest]
fn test_llzk_attribute_is_a_felt_cmp_predicate_attr_fail(context: TestContext) {
    unsafe {
        let attr = mlirUnitAttrGet(context.ctx);
        assert!(!llzkAttributeIsAFeltCmpPredicateAttr(attr));
    }
}
