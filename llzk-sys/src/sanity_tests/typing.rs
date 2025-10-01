use std::{
    ffi::{CString, c_void},
    ptr::{null, null_mut},
};

use mlir_sys::{
    MlirOperation, MlirType, mlirArrayAttrGet, mlirAttributeEqual, mlirAttributeGetContext,
    mlirFlatSymbolRefAttrGet, mlirIndexTypeGet, mlirIntegerAttrGet, mlirIntegerTypeGet,
    mlirLocationUnknownGet, mlirStringRefCreateFromCString,
};
use rstest::{fixture, rstest};

use crate::{
    llzkArrayAttrTypeParamsUnify, llzkAssertValidAttrForParamOfType, llzkForceIntAttrType,
    llzkHasAffineMapAttr, llzkIsConcreteType, llzkIsMoreConcreteUnification, llzkIsSignalType,
    llzkIsValidArrayElemType, llzkIsValidArrayType, llzkIsValidColumnType,
    llzkIsValidConstReadType, llzkIsValidEmitEqType, llzkIsValidGlobalType, llzkIsValidType,
    llzkTypeParamsUnify, llzkTypesUnify,
    sanity_tests::{TestContext, context},
};

struct IndexType {
    #[allow(dead_code)]
    context: TestContext,
    t: MlirType,
}

#[fixture]
fn index_type(context: TestContext) -> IndexType {
    unsafe {
        let ctx = context.ctx;
        IndexType {
            context,
            t: mlirIndexTypeGet(ctx),
        }
    }
}

struct I16Type {
    #[allow(dead_code)]
    context: TestContext,
    t: MlirType,
}

#[fixture]
fn i16_type(context: TestContext) -> I16Type {
    unsafe {
        let ctx = context.ctx;
        I16Type {
            context,
            t: mlirIntegerTypeGet(ctx, 16),
        }
    }
}

#[rstest]
fn test_llzk_assert_valid_attr_for_param_of_type(index_type: IndexType) {
    unsafe {
        let int_attr = mlirIntegerAttrGet(index_type.t, 0);
        llzkAssertValidAttrForParamOfType(int_attr);
    }
}

#[rstest]
fn test_llzk_is_valid_type(index_type: IndexType) {
    unsafe {
        assert!(llzkIsValidType(index_type.t));
    }
}

#[rstest]
fn test_llzk_is_valid_column_type(index_type: IndexType) {
    unsafe {
        let null_op = MlirOperation { ptr: null_mut() };
        assert!(!llzkIsValidColumnType(index_type.t, null_op));
    }
}

#[rstest]
fn test_llzk_is_valid_emit_eq_type(index_type: IndexType) {
    unsafe {
        assert!(llzkIsValidEmitEqType(index_type.t));
    }
}

#[rstest]
fn test_llzk_is_valid_const_read_type(index_type: IndexType) {
    unsafe {
        assert!(llzkIsValidConstReadType(index_type.t));
    }
}

#[rstest]
fn test_llzk_is_valid_array_elem_type(index_type: IndexType) {
    unsafe {
        assert!(llzkIsValidArrayElemType(index_type.t));
    }
}

#[rstest]
fn test_llzk_is_valid_array_type(index_type: IndexType) {
    unsafe {
        assert!(!llzkIsValidArrayType(index_type.t));
    }
}

#[rstest]
fn test_llzk_is_concrete_type(index_type: IndexType) {
    unsafe {
        assert!(llzkIsConcreteType(index_type.t, true));
    }
}

#[rstest]
fn test_llzk_is_signal_type(index_type: IndexType) {
    unsafe {
        assert!(!llzkIsSignalType(index_type.t));
    }
}

#[rstest]
fn test_llzk_has_affine_map_attr(index_type: IndexType) {
    unsafe {
        assert!(!llzkHasAffineMapAttr(index_type.t));
    }
}

#[rstest]
fn test_llzk_type_params_unify_empty() {
    unsafe {
        let lhs = [];
        let rhs = [];
        assert!(llzkTypeParamsUnify(
            lhs.len() as isize,
            lhs.as_ptr(),
            rhs.len() as isize,
            rhs.as_ptr()
        ));
    }
}

#[rstest]
fn test_llzk_type_params_unify_pass(index_type: IndexType) {
    unsafe {
        let string = CString::new("N").unwrap();

        let string_ref = mlirStringRefCreateFromCString(string.as_ptr());

        let lhs = [mlirIntegerAttrGet(index_type.t, 0)];
        let rhs = [mlirFlatSymbolRefAttrGet(
            mlirAttributeGetContext(lhs[0]),
            string_ref,
        )];
        assert!(llzkTypeParamsUnify(
            lhs.len() as isize,
            lhs.as_ptr(),
            rhs.len() as isize,
            rhs.as_ptr()
        ));
    }
}

#[rstest]
fn test_llzk_type_params_unify_fail(index_type: IndexType) {
    unsafe {
        let lhs = [mlirIntegerAttrGet(index_type.t, 0)];
        let rhs = [mlirIntegerAttrGet(index_type.t, 1)];
        assert!(!llzkTypeParamsUnify(
            lhs.len() as isize,
            lhs.as_ptr(),
            rhs.len() as isize,
            rhs.as_ptr()
        ));
    }
}

#[rstest]
fn test_llzk_array_attr_type_params_unify_empty(context: TestContext) {
    unsafe {
        let lhs = [];
        let lhs = mlirArrayAttrGet(context.ctx, lhs.len() as isize, lhs.as_ptr());
        let rhs = [];
        let rhs = mlirArrayAttrGet(context.ctx, rhs.len() as isize, rhs.as_ptr());
        assert!(llzkArrayAttrTypeParamsUnify(lhs, rhs));
    }
}

#[rstest]
fn test_llzk_array_attr_type_params_unify_pass(index_type: IndexType) {
    unsafe {
        let string = CString::new("N").unwrap();

        let string_ref = mlirStringRefCreateFromCString(string.as_ptr());

        let lhs = [mlirIntegerAttrGet(index_type.t, 0)];
        let lhs = mlirArrayAttrGet(
            mlirAttributeGetContext(lhs[0]),
            lhs.len() as isize,
            lhs.as_ptr(),
        );
        let rhs = [mlirFlatSymbolRefAttrGet(
            mlirAttributeGetContext(lhs),
            string_ref,
        )];
        let rhs = mlirArrayAttrGet(
            mlirAttributeGetContext(lhs),
            rhs.len() as isize,
            rhs.as_ptr(),
        );
        assert!(llzkArrayAttrTypeParamsUnify(lhs, rhs));
    }
}

#[rstest]
fn test_llzk_array_attr_type_params_unify_fail(index_type: IndexType) {
    unsafe {
        let lhs = [mlirIntegerAttrGet(index_type.t, 0)];

        let lhs = mlirArrayAttrGet(
            mlirAttributeGetContext(lhs[0]),
            lhs.len() as isize,
            lhs.as_ptr(),
        );
        let rhs = [mlirIntegerAttrGet(index_type.t, 1)];

        let rhs = mlirArrayAttrGet(
            mlirAttributeGetContext(lhs),
            rhs.len() as isize,
            rhs.as_ptr(),
        );
        assert!(!llzkArrayAttrTypeParamsUnify(lhs, rhs));
    }
}

#[rstest]
fn test_llzk_types_unify(index_type: IndexType) {
    unsafe {
        assert!(llzkTypesUnify(index_type.t, index_type.t, 0, null()));
    }
}

#[rstest]
fn test_llzk_is_more_concrete_unification(index_type: IndexType) {
    unsafe {
        assert!(llzkIsMoreConcreteUnification(
            index_type.t,
            index_type.t,
            Some(test_callback1),
            null_mut()
        ));
    }
}

#[rstest]
fn test_llzk_force_int_attr_type(context: TestContext, i16_type: I16Type) {
    unsafe {
        let location = mlirLocationUnknownGet(context.ctx);
        let in_attr = mlirIntegerAttrGet(i16_type.t, 0);
        let out_attr = llzkForceIntAttrType(in_attr, location);
        assert!(!mlirAttributeEqual(in_attr, out_attr));
    }
}

#[rstest]
fn test_llzk_is_valid_global_type(index_type: IndexType) {
    unsafe {
        assert!(llzkIsValidGlobalType(index_type.t));
    }
}

unsafe extern "C" fn test_callback1(_: MlirType, _: MlirType, _: *mut c_void) -> bool {
    true
}
