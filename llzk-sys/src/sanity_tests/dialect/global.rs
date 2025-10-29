use std::ptr::null_mut;

use mlir_sys::{
    MlirAttribute, MlirContext, MlirNamedAttribute, MlirOperation, MlirType,
    mlirAttributeGetContext, mlirFlatSymbolRefAttrGet, mlirIdentifierGet, mlirIndexTypeGet,
    mlirIntegerAttrGet, mlirLocationUnknownGet, mlirNamedAttributeGet, mlirOperationCreate,
    mlirOperationDestroy, mlirOperationStateAddAttributes, mlirOperationStateGet, mlirTypeAttrGet,
    mlirUnitAttrGet,
};
use rstest::rstest;

use crate::{
    llzkGlobalDefOpGetIsConstant, llzkOperationIsAGlobalDefOp,
    mlirGetDialectHandle__llzk__global__,
    sanity_tests::{TestContext, context, str_ref},
};

#[test]
fn test_mlir_get_dialect_handle_llzk_global() {
    unsafe {
        mlirGetDialectHandle__llzk__global__();
    }
}

fn named_attr(s: &'static str, attr: MlirAttribute) -> MlirNamedAttribute {
    unsafe {
        mlirNamedAttributeGet(
            mlirIdentifierGet(mlirAttributeGetContext(attr), str_ref(s)),
            attr,
        )
    }
}

fn create_global_def_op(
    ctx: MlirContext,
    sym_name: &'static str,
    constant: bool,
    r#type: MlirType,
    initial_value: Option<MlirAttribute>,
) -> MlirOperation {
    unsafe {
        let sym_name = mlirFlatSymbolRefAttrGet(ctx, str_ref(sym_name));
        let mut attrs = vec![
            named_attr("sym_name", sym_name),
            named_attr("type", mlirTypeAttrGet(r#type)),
        ];
        if constant {
            attrs.push(named_attr("constant", mlirUnitAttrGet(ctx)));
        }
        if let Some(value) = initial_value {
            attrs.push(named_attr("initial_value", value));
        }
        let name = str_ref("global.def");
        let mut state = mlirOperationStateGet(name, mlirLocationUnknownGet(ctx));
        mlirOperationStateAddAttributes(&mut state, attrs.len() as isize, attrs.as_ptr());

        mlirOperationCreate(&mut state)
    }
}

#[rstest]
fn test_llzk_operation_is_a_global_def_op(context: TestContext) {
    unsafe {
        let op = create_global_def_op(context.ctx, "G", false, mlirIndexTypeGet(context.ctx), None);
        assert_ne!(op.ptr, null_mut());
        assert!(llzkOperationIsAGlobalDefOp(op));
        mlirOperationDestroy(op);
    }
}

#[rstest]
fn test_llzk_global_def_op_get_is_constant_1(context: TestContext) {
    unsafe {
        let op = create_global_def_op(context.ctx, "G", false, mlirIndexTypeGet(context.ctx), None);
        assert_ne!(op.ptr, null_mut());
        assert!(!llzkGlobalDefOpGetIsConstant(op));
        mlirOperationDestroy(op);
    }
}

#[rstest]
fn test_llzk_global_def_op_get_is_constant_2(context: TestContext) {
    unsafe {
        let op = create_global_def_op(
            context.ctx,
            "G",
            true,
            mlirIndexTypeGet(context.ctx),
            Some(mlirIntegerAttrGet(mlirIndexTypeGet(context.ctx), 1)),
        );
        assert_ne!(op.ptr, null_mut());
        assert!(llzkGlobalDefOpGetIsConstant(op));
        mlirOperationDestroy(op);
    }
}
