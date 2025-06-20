use crate::{
    llzkCallOpBuild, llzkCallOpBuildToCallee, llzkCallOpBuildToCalleeWithMapOperands,
    llzkCallOpBuildToCalleeWithMapOperandsAndDims, llzkCallOpBuildWithMapOperands,
    llzkCallOpBuildWithMapOperandsAndDims, llzkCallOpGetCalleeIsCompute,
    llzkCallOpGetCalleeIsConstrain, llzkCallOpGetCalleeIsStructCompute,
    llzkCallOpGetCalleeIsStructConstrain, llzkCallOpGetCalleeType,
    llzkCallOpGetSingleResultTypeOfCompute, llzkFuncDefOpCreateWithAttrsAndArgAttrs,
    llzkFuncDefOpGetFullyQualifiedName, llzkFuncDefOpGetHasAllowConstraintAttr,
    llzkFuncDefOpGetHasAllowWitnessAttr, llzkFuncDefOpGetHasArgIsPub, llzkFuncDefOpGetIsInStruct,
    llzkFuncDefOpGetIsStructCompute, llzkFuncDefOpGetIsStructConstrain,
    llzkFuncDefOpGetNameIsCompute, llzkFuncDefOpGetNameIsConstrain,
    llzkFuncDefOpGetSingleResultTypeOfCompute, llzkFuncDefOpSetAllowConstraintAttr,
    llzkFuncDefOpSetAllowWitnessAttr, llzkOperationIsACallOp, llzkOperationIsAFuncDefOp,
    mlirGetDialectHandle__llzk__function__, mlirOpBuilderCreate,
    sanity_tests::{context, str_ref, TestContext},
};
use mlir_sys::{
    mlirDenseI32ArrayGet, mlirDictionaryAttrGet, mlirFlatSymbolRefAttrGet,
    mlirFunctionTypeGet, mlirIndexTypeGet, mlirLocationUnknownGet, mlirOperationDestroy,
    mlirOperationGetContext, mlirOperationVerify, mlirStringRefCreateFromCString, mlirTypeEqual,
    MlirAttribute, MlirContext, MlirNamedAttribute, MlirOperation, MlirType,
};
use rstest::{fixture, rstest};
use std::{
    ffi::CString,
    ptr::{null, null_mut},
};

#[test]
fn test_mlir_get_dialect_handle_llzk_function() {
    unsafe {
        mlirGetDialectHandle__llzk__function__();
    }
}

fn create_func_type(ctx: MlirContext, ins: &[MlirType], outs: &[MlirType]) -> MlirType {
    unsafe {
        mlirFunctionTypeGet(
            ctx,
            ins.len() as isize,
            ins.as_ptr(),
            outs.len() as isize,
            outs.as_ptr(),
        )
    }
}

fn create_func_def_op(
    ctx: MlirContext,
    name: &str,
    r#type: MlirType,
    attrs: &[MlirNamedAttribute],
    arg_attrs: &[MlirAttribute],
) -> MlirOperation {
    unsafe {
        let location = mlirLocationUnknownGet(ctx);
        let name = CString::new(name).unwrap();
        let name = mlirStringRefCreateFromCString(name.as_ptr());
        llzkFuncDefOpCreateWithAttrsAndArgAttrs(
            location,
            name,
            r#type,
            attrs.len() as isize,
            attrs.as_ptr(),
            arg_attrs.len() as isize,
            arg_attrs.as_ptr(),
        )
    }
}

fn empty_arg_attrs<const N: usize>(ctx: MlirContext, _: &[MlirType; N]) -> [MlirAttribute; N] {
    std::array::from_fn(|_| unsafe { mlirDictionaryAttrGet(ctx, 0, null()) })
}

#[rstest]
fn test_llzk_func_def_op_create_with_attrs_and_arg_attrs(context: TestContext) {
    unsafe {
        let in_types = [mlirIndexTypeGet(context.ctx)];
        let in_attrs = empty_arg_attrs(context.ctx, &in_types);
        //let in_attrs = [mlirDictionaryAttrGet(context.ctx, 0, null())];
        let op = create_func_def_op(
            context.ctx,
            "foo",
            create_func_type(context.ctx, &in_types, &[]),
            &[],
            &in_attrs,
        );
        mlirOperationDestroy(op);
    }
}

struct TestFuncDefOp {
    #[allow(dead_code)]
    context: TestContext,
    pub op: MlirOperation,
    pub in_types: Vec<MlirType>,
    pub out_types: Vec<MlirType>,
    pub name: &'static str,
}

impl Drop for TestFuncDefOp {
    fn drop(&mut self) {
        unsafe { mlirOperationDestroy(self.op) }
    }
}

#[fixture]
fn test_function(context: TestContext) -> TestFuncDefOp {
    let in_types = [unsafe { mlirIndexTypeGet(context.ctx) }, unsafe {
        mlirIndexTypeGet(context.ctx)
    }];
    let in_attrs = empty_arg_attrs(context.ctx, &in_types);
    let out_types = [unsafe { mlirIndexTypeGet(context.ctx) }];
    let name = "foo";
    let ctx = context.ctx;
    TestFuncDefOp {
        context,
        in_types: in_types.to_vec(),
        out_types: out_types.to_vec(),
        name,
        op: create_func_def_op(
            ctx,
            name,
            create_func_type(ctx, &in_types, &out_types),
            &[],
            &in_attrs,
        ),
    }
}

#[fixture]
fn test_function0(context: TestContext) -> TestFuncDefOp {
    let in_types = [];
    let out_types = [unsafe { mlirIndexTypeGet(context.ctx) }];
    let name = "bar";
    let ctx = context.ctx;
    TestFuncDefOp {
        context,
        in_types: in_types.to_vec(),
        out_types: out_types.to_vec(),
        name,
        op: create_func_def_op(
            ctx,
            name,
            create_func_type(ctx, &in_types, &out_types),
            &[],
            &[],
        ),
    }
}

#[rstest]
fn test_llzk_operation_is_a_func_def_op(test_function: TestFuncDefOp) {
    unsafe {
        assert!(llzkOperationIsAFuncDefOp(test_function.op));
    }
}

#[rstest]
fn test_llzk_func_def_op_get_has_allow_constraint_attr(test_function: TestFuncDefOp) {
    unsafe {
        assert!(!llzkFuncDefOpGetHasAllowConstraintAttr(test_function.op));
    }
}

#[rstest]
fn test_llzk_func_def_op_set_allow_constraint_attr(test_function: TestFuncDefOp) {
    unsafe {
        assert!(!llzkFuncDefOpGetHasAllowConstraintAttr(test_function.op));
        llzkFuncDefOpSetAllowConstraintAttr(test_function.op, true);
        assert!(llzkFuncDefOpGetHasAllowConstraintAttr(test_function.op));
        llzkFuncDefOpSetAllowConstraintAttr(test_function.op, false);
        assert!(!llzkFuncDefOpGetHasAllowConstraintAttr(test_function.op));
    }
}

#[rstest]
fn test_llzk_func_def_op_get_has_allow_witness_attr(test_function: TestFuncDefOp) {
    unsafe {
        assert!(!llzkFuncDefOpGetHasAllowWitnessAttr(test_function.op));
    }
}

#[rstest]
fn test_llzk_func_def_op_set_allow_witness_attr(test_function: TestFuncDefOp) {
    unsafe {
        assert!(!llzkFuncDefOpGetHasAllowWitnessAttr(test_function.op));
        llzkFuncDefOpSetAllowWitnessAttr(test_function.op, true);
        assert!(llzkFuncDefOpGetHasAllowWitnessAttr(test_function.op));
        llzkFuncDefOpSetAllowWitnessAttr(test_function.op, false);
        assert!(!llzkFuncDefOpGetHasAllowWitnessAttr(test_function.op));
    }
}

#[rstest]
fn test_llzk_func_def_op_get_has_arg_is_pub(test_function: TestFuncDefOp) {
    unsafe { assert!(!llzkFuncDefOpGetHasArgIsPub(test_function.op, 0)) }
}

#[rstest]
fn test_llzk_func_def_op_get_fully_qualified_name(test_function: TestFuncDefOp) {
    unsafe {
        // Because the func is not included in a module or struct calling this method will result
        // in an error. To avoid this while still having a test that links against the function we
        // only "call" the method on a condition that is actually impossible but the compiler
        // cannot see that.
        if test_function.op.ptr == null_mut() {
            llzkFuncDefOpGetFullyQualifiedName(test_function.op);
        }
    }
}

macro_rules! false_pred_test {
    ($test:ident, $func:ident) => {
        #[rstest]
        fn $test(test_function: TestFuncDefOp) {
            unsafe {
                assert!(!$func(test_function.op));
            }
        }
    };
}

false_pred_test!(
    test_llzk_func_def_op_get_name_is_compute,
    llzkFuncDefOpGetNameIsCompute
);
false_pred_test!(
    test_llzk_func_def_op_get_name_is_constrain,
    llzkFuncDefOpGetNameIsConstrain
);
false_pred_test!(
    test_llzk_func_def_op_get_is_in_struct,
    llzkFuncDefOpGetIsInStruct
);
false_pred_test!(
    test_llzk_func_def_op_get_is_struct_compute,
    llzkFuncDefOpGetIsStructCompute
);
false_pred_test!(
    test_llzk_func_def_op_get_is_struct_constrain,
    llzkFuncDefOpGetIsStructConstrain
);

#[rstest]
fn test_llzk_func_def_op_get_single_result_type_of_compute(test_function: TestFuncDefOp) {
    unsafe {
        // We want to link the function to make sure it has been implemented but we don't want to
        // call it because the precondition checks will fail with the test function.
        if llzkFuncDefOpGetIsStructCompute(test_function.op) {
            llzkFuncDefOpGetSingleResultTypeOfCompute(test_function.op);
        }
    }
}

#[rstest]
fn test_llzk_call_op_build(test_function0: TestFuncDefOp) {
    unsafe {
        let ctx = mlirOperationGetContext(test_function0.op);
        let builder = mlirOpBuilderCreate(ctx);
        let location = mlirLocationUnknownGet(ctx);
        let callee_name = str_ref(test_function0.name);
        let callee_name = mlirFlatSymbolRefAttrGet(ctx, callee_name);
        let call = llzkCallOpBuild(
            builder,
            location,
            test_function0.out_types.len() as isize,
            test_function0.out_types.as_ptr(),
            callee_name,
            0,
            null(),
        );
        assert!(mlirOperationVerify(call));
        mlirOperationDestroy(call);
    }
}

#[rstest]
fn test_llzk_call_op_build_to_callee(test_function0: TestFuncDefOp) {
    unsafe {
        let ctx = mlirOperationGetContext(test_function0.op);
        let builder = mlirOpBuilderCreate(ctx);
        let location = mlirLocationUnknownGet(ctx);
        let call = llzkCallOpBuildToCallee(builder, location, test_function0.op, 0, null());
        assert!(mlirOperationVerify(call));
        mlirOperationDestroy(call);
    }
}

#[rstest]
fn llzk_call_op_build_with_map_operands(test_function0: TestFuncDefOp) {
    unsafe {
        let ctx = mlirOperationGetContext(test_function0.op);
        let builder = mlirOpBuilderCreate(ctx);
        let location = mlirLocationUnknownGet(ctx);
        let callee_name = str_ref(test_function0.name);
        let callee_name = mlirFlatSymbolRefAttrGet(ctx, callee_name);
        let dims_per_map = mlirDenseI32ArrayGet(ctx, 0, null());
        let call = llzkCallOpBuildWithMapOperands(
            builder,
            location,
            test_function0.out_types.len() as isize,
            test_function0.out_types.as_ptr(),
            callee_name,
            0,
            null(),
            dims_per_map,
            0,
            null(),
        );
        assert!(mlirOperationVerify(call));
        mlirOperationDestroy(call);
    }
}

#[rstest]
fn llzk_call_op_build_with_map_operands_and_dims(test_function0: TestFuncDefOp) {
    unsafe {
        let ctx = mlirOperationGetContext(test_function0.op);
        let builder = mlirOpBuilderCreate(ctx);
        let location = mlirLocationUnknownGet(ctx);
        let callee_name = str_ref(test_function0.name);
        let callee_name = mlirFlatSymbolRefAttrGet(ctx, callee_name);
        let call = llzkCallOpBuildWithMapOperandsAndDims(
            builder,
            location,
            test_function0.out_types.len() as isize,
            test_function0.out_types.as_ptr(),
            callee_name,
            0,
            null(),
            0,
            null(),
            0,
            null(),
        );
        assert!(mlirOperationVerify(call));
        mlirOperationDestroy(call);
    }
}

#[rstest]
fn llzk_call_op_build_to_callee_with_map_operands(test_function0: TestFuncDefOp) {
    unsafe {
        let ctx = mlirOperationGetContext(test_function0.op);
        let builder = mlirOpBuilderCreate(ctx);
        let location = mlirLocationUnknownGet(ctx);
        let dims_per_map = mlirDenseI32ArrayGet(ctx, 0, null());
        let call = llzkCallOpBuildToCalleeWithMapOperands(
            builder,
            location,
            test_function0.op,
            0,
            null(),
            dims_per_map,
            0,
            null(),
        );
        assert!(mlirOperationVerify(call));
        mlirOperationDestroy(call);
    }
}

#[rstest]
fn llzk_call_op_build_to_callee_with_map_operands_and_dims(test_function0: TestFuncDefOp) {
    unsafe {
        let ctx = mlirOperationGetContext(test_function0.op);
        let builder = mlirOpBuilderCreate(ctx);
        let location = mlirLocationUnknownGet(ctx);
        let call = llzkCallOpBuildToCalleeWithMapOperandsAndDims(
            builder,
            location,
            test_function0.op,
            0,
            null(),
            0,
            null(),
            0,
            null(),
        );
        assert!(mlirOperationVerify(call));
        mlirOperationDestroy(call);
    }
}

macro_rules! call_pred_test {
    ($test:ident, $func:ident, $expected:expr) => {
        #[rstest]
        fn $test(test_function0: TestFuncDefOp) {
            unsafe {
                let ctx = mlirOperationGetContext(test_function0.op);
                let builder = mlirOpBuilderCreate(ctx);
                let location = mlirLocationUnknownGet(ctx);
                let call = llzkCallOpBuildToCallee(builder, location, test_function0.op, 0, null());

                assert_eq!($func(call), $expected);
                mlirOperationDestroy(call);
            }
        }
    };
}

call_pred_test!(
    test_llzk_operation_is_a_call_op,
    llzkOperationIsACallOp,
    true
);

#[rstest]
fn test_llzk_call_op_get_callee_type(test_function0: TestFuncDefOp) {
    unsafe {
        let ctx = mlirOperationGetContext(test_function0.op);
        let builder = mlirOpBuilderCreate(ctx);
        let location = mlirLocationUnknownGet(ctx);
        let call = llzkCallOpBuildToCallee(builder, location, test_function0.op, 0, null());

        let func_type = create_func_type(ctx, &test_function0.in_types, &test_function0.out_types);
        let out_type = llzkCallOpGetCalleeType(call);
        assert!(mlirTypeEqual(func_type, out_type));

        mlirOperationDestroy(call);
    }
}

call_pred_test!(
    test_llzk_call_op_get_callee_is_compute,
    llzkCallOpGetCalleeIsCompute,
    false
);
call_pred_test!(
    test_llzk_call_op_get_callee_is_constrain,
    llzkCallOpGetCalleeIsConstrain,
    false
);
call_pred_test!(
    test_llzk_call_op_get_callee_is_struct_compute,
    llzkCallOpGetCalleeIsStructCompute,
    false
);
call_pred_test!(
    test_llzk_call_op_get_callee_is_struct_constrain,
    llzkCallOpGetCalleeIsStructConstrain,
    false
);

#[rstest]
fn test_llzk_call_op_get_single_result_type_of_compute(test_function0: TestFuncDefOp) {
    unsafe {
        let ctx = mlirOperationGetContext(test_function0.op);
        let builder = mlirOpBuilderCreate(ctx);
        let location = mlirLocationUnknownGet(ctx);
        let call = llzkCallOpBuildToCallee(builder, location, test_function0.op, 0, null());

        // We want to link the function to make sure it has been implemented but we don't want to
        // call it because the precondition checks will fail with the test function.
        if llzkCallOpGetCalleeIsStructCompute(call) {
            llzkCallOpGetSingleResultTypeOfCompute(call);
        }

        mlirOperationDestroy(call);
    }
}
