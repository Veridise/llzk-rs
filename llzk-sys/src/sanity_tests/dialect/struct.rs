use std::{
    ffi::CString,
    ptr::{null, null_mut},
};

use mlir_sys::{
    mlirAffineConstantExprGet, mlirAffineMapGet, mlirArrayAttrGet, mlirAttributeEqual,
    mlirFlatSymbolRefAttrGet, mlirIdentifierGet, mlirIndexTypeGet, mlirIntegerAttrGet,
    mlirLocationUnknownGet, mlirNamedAttributeGet, mlirOperationCreate, mlirOperationDestroy,
    mlirOperationGetContext, mlirOperationGetResult, mlirOperationStateAddAttributes,
    mlirOperationStateAddResults, mlirOperationStateGet, mlirStringRefCreateFromCString,
    MlirOperation,
};
use rstest::{fixture, rstest};
use std::alloc::{Layout, alloc, dealloc};

use crate::{
    MlirValueRange, llzkFieldDefOpGetHasPublicAttr, llzkFieldDefOpSetPublicAttr,
    llzkFieldReadOpBuild, llzkFieldReadOpBuildWithAffineMapDistance,
    llzkFieldReadOpBuildWithConstParamDistance, llzkFieldReadOpBuildWithLiteralDistance,
    llzkOperationIsAFieldDefOp, llzkOperationIsAStructDefOp, llzkStructDefOpGetBody,
    llzkStructDefOpGetBodyRegion, llzkStructDefOpGetComputeFuncOp,
    llzkStructDefOpGetConstrainFuncOp, llzkStructDefOpGetFieldDef, llzkStructDefOpGetFieldDefs,
    llzkStructDefOpGetFullyQualifiedName, llzkStructDefOpGetHasColumns,
    llzkStructDefOpGetHasParamName, llzkStructDefOpGetHeaderString,
    llzkStructDefOpGetIsMainComponent, llzkStructDefOpGetNumFieldDefs, llzkStructDefOpGetType,
    llzkStructDefOpGetTypeWithParams, llzkStructTypeGet, llzkStructTypeGetName,
    llzkStructTypeGetParams, llzkStructTypeGetWithArrayAttr, llzkStructTypeGetWithAttrs,
    llzkTypeIsAStructType, mlirGetDialectHandle__llzk__component__, mlirOpBuilderCreate,
    mlirOpBuilderDestroy,
    sanity_tests::{context, str_ref, TestContext},
    MlirValueRange,
};

#[test]
fn test_mlir_get_dialect_handle_llzk_component() {
    unsafe {
        mlirGetDialectHandle__llzk__component__();
    }
}

#[rstest]
fn test_llzk_struct_type_get(context: TestContext) {
    unsafe {
        let s = str_ref("T");
        let s = mlirFlatSymbolRefAttrGet(context.ctx, s);
        let t = llzkStructTypeGet(s);
        assert_ne!(t.ptr, null());
    }
}

#[rstest]
fn test_llzk_struct_type_get_with_array_attr(context: TestContext) {
    unsafe {
        let s = str_ref("T");
        let s = mlirFlatSymbolRefAttrGet(context.ctx, s);
        let attrs = [mlirFlatSymbolRefAttrGet(context.ctx, str_ref("A"))];
        let a = mlirArrayAttrGet(context.ctx, attrs.len() as isize, attrs.as_ptr());
        let t = llzkStructTypeGetWithArrayAttr(s, a);
        assert_ne!(t.ptr, null());
    }
}

#[rstest]
fn test_llzk_struct_type_get_with_attrs(context: TestContext) {
    unsafe {
        let s = str_ref("T");
        let s = mlirFlatSymbolRefAttrGet(context.ctx, s);
        let attrs = [mlirFlatSymbolRefAttrGet(context.ctx, str_ref("A"))];
        let t = llzkStructTypeGetWithAttrs(s, attrs.len() as isize, attrs.as_ptr());
        assert_ne!(t.ptr, null());
    }
}

#[rstest]
fn test_llzk_type_is_a_struct_type(context: TestContext) {
    unsafe {
        let s = str_ref("T");
        let s = mlirFlatSymbolRefAttrGet(context.ctx, s);
        let t = llzkStructTypeGet(s);
        assert_ne!(t.ptr, null());
        assert!(llzkTypeIsAStructType(t));
    }
}

#[rstest]
fn test_llzk_struct_type_get_name(context: TestContext) {
    unsafe {
        let s = str_ref("T");
        let s = mlirFlatSymbolRefAttrGet(context.ctx, s);
        let t = llzkStructTypeGet(s);
        assert_ne!(t.ptr, null());
        assert!(mlirAttributeEqual(s, llzkStructTypeGetName(t)));
    }
}

#[rstest]
fn test_llzk_struct_type_get_params(context: TestContext) {
    unsafe {
        let s = str_ref("T");
        let s = mlirFlatSymbolRefAttrGet(context.ctx, s);
        let attrs = [mlirFlatSymbolRefAttrGet(context.ctx, str_ref("A"))];
        let a = mlirArrayAttrGet(context.ctx, attrs.len() as isize, attrs.as_ptr());
        let t = llzkStructTypeGetWithArrayAttr(s, a);
        assert_ne!(t.ptr, null());
        assert!(mlirAttributeEqual(a, llzkStructTypeGetParams(t)));
    }
}

struct TestOp {
    #[allow(dead_code)]
    context: TestContext,
    op: MlirOperation,
}

impl Drop for TestOp {
    fn drop(&mut self) {
        unsafe { mlirOperationDestroy(self.op) }
    }
}

#[fixture]
fn test_op(context: TestContext) -> TestOp {
    unsafe {
        let ctx = context.ctx;
        let elt_type = mlirIndexTypeGet(ctx);
        let arith_constant_op_str = CString::new("arith.constant").unwrap();
        let value_str = CString::new("value").unwrap();
        let name = mlirStringRefCreateFromCString(arith_constant_op_str.as_ptr());
        let attr_name = mlirIdentifierGet(ctx, mlirStringRefCreateFromCString(value_str.as_ptr()));
        let location = mlirLocationUnknownGet(ctx);
        let results = [elt_type];
        let attr = mlirIntegerAttrGet(elt_type, 1);
        let attrs = [mlirNamedAttributeGet(attr_name, attr)];
        let mut op_state = mlirOperationStateGet(name, location);
        mlirOperationStateAddResults(&mut op_state, results.len() as isize, results.as_ptr());
        mlirOperationStateAddAttributes(&mut op_state, attrs.len() as isize, attrs.as_ptr());
        TestOp {
            context,
            op: mlirOperationCreate(&mut op_state),
        }
    }
}

#[rstest]
fn test_llzk_operation_is_a_struct_def_op(test_op: TestOp) {
    unsafe {
        assert!(!llzkOperationIsAStructDefOp(test_op.op));
    }
}

#[rstest]
fn test_llzk_struct_def_op_get_body_region(test_op: TestOp) {
    unsafe {
        if llzkOperationIsAStructDefOp(test_op.op) {
            llzkStructDefOpGetBodyRegion(test_op.op);
        }
    }
}

#[rstest]
fn test_llzk_struct_def_op_get_body(test_op: TestOp) {
    unsafe {
        if llzkOperationIsAStructDefOp(test_op.op) {
            llzkStructDefOpGetBody(test_op.op);
        }
    }
}

#[rstest]
fn test_llzk_struct_def_op_get_type(test_op: TestOp) {
    unsafe {
        if llzkOperationIsAStructDefOp(test_op.op) {
            llzkStructDefOpGetType(test_op.op);
        }
    }
}

#[rstest]
fn test_llzk_struct_def_op_get_type_with_params(test_op: TestOp) {
    unsafe {
        if llzkOperationIsAStructDefOp(test_op.op) {
            let attrs = mlirArrayAttrGet(mlirOperationGetContext(test_op.op), 0, null());
            llzkStructDefOpGetTypeWithParams(test_op.op, attrs);
        }
    }
}

#[rstest]
fn test_llzk_struct_def_op_get_field_def(test_op: TestOp) {
    unsafe {
        if llzkOperationIsAStructDefOp(test_op.op) {
            let name = str_ref("p");
            llzkStructDefOpGetFieldDef(test_op.op, name);
        }
    }
}

#[rstest]
fn test_llzk_struct_def_op_get_field_defs(test_op: TestOp) {
    unsafe {
        if llzkOperationIsAStructDefOp(test_op.op) {
            llzkStructDefOpGetFieldDefs(test_op.op, null_mut());
        }
    }
}

#[rstest]
fn test_llzk_struct_def_op_get_num_field_defs(test_op: TestOp) {
    unsafe {
        if llzkOperationIsAStructDefOp(test_op.op) {
            llzkStructDefOpGetNumFieldDefs(test_op.op);
        }
    }
}

#[rstest]
fn test_llzk_struct_def_op_get_has_columns(test_op: TestOp) {
    unsafe {
        if llzkOperationIsAStructDefOp(test_op.op) {
            llzkStructDefOpGetHasColumns(test_op.op);
        }
    }
}

#[rstest]
fn test_llzk_struct_def_op_get_compute_func_op(test_op: TestOp) {
    unsafe {
        if llzkOperationIsAStructDefOp(test_op.op) {
            llzkStructDefOpGetComputeFuncOp(test_op.op);
        }
    }
}

#[rstest]
fn test_llzk_struct_def_op_get_constrain_func_op(test_op: TestOp) {
    unsafe {
        if llzkOperationIsAStructDefOp(test_op.op) {
            llzkStructDefOpGetConstrainFuncOp(test_op.op);
        }
    }
}

#[rstest]
fn test_llzk_struct_def_op_get_header_string(test_op: TestOp) {
    unsafe {
        if llzkOperationIsAStructDefOp(test_op.op) {
            use core::ffi::c_char;
            extern "C" fn allocator(size: usize) -> *mut c_char {
                let layout = Layout::array::<c_char>(size).expect("failed to define string layout");
                unsafe { alloc(layout) as *mut c_char }
            }
            let mut size = 0;

            let str = llzkStructDefOpGetHeaderString(test_op.op, &mut size, Some(allocator));
            let layout =
                Layout::array::<c_char>(size as usize).expect("failed to define string layout");
            dealloc(str as *mut u8, layout);
        }
    }
}

#[rstest]
fn test_llzk_struct_def_op_get_has_param_name(test_op: TestOp) {
    unsafe {
        if llzkOperationIsAStructDefOp(test_op.op) {
            let name = str_ref("p");
            llzkStructDefOpGetHasParamName(test_op.op, name);
        }
    }
}

#[rstest]
fn test_llzk_struct_def_op_get_fully_qualified_name(test_op: TestOp) {
    unsafe {
        if llzkOperationIsAStructDefOp(test_op.op) {
            llzkStructDefOpGetFullyQualifiedName(test_op.op);
        }
    }
}

#[rstest]
fn test_llzk_struct_def_op_get_is_main_component(test_op: TestOp) {
    unsafe {
        if llzkOperationIsAStructDefOp(test_op.op) {
            llzkStructDefOpGetIsMainComponent(test_op.op);
        }
    }
}

#[rstest]
fn test_llzk_operation_is_a_field_def_op(test_op: TestOp) {
    unsafe {
        assert!(!llzkOperationIsAFieldDefOp(test_op.op));
    }
}

#[rstest]
fn test_llzk_field_def_op_get_has_public_attr(test_op: TestOp) {
    unsafe {
        if llzkOperationIsAFieldDefOp(test_op.op) {
            llzkFieldDefOpGetHasPublicAttr(test_op.op);
        }
    }
}

#[rstest]
fn test_llzk_field_def_op_set_public_attr(test_op: TestOp) {
    unsafe {
        if llzkOperationIsAFieldDefOp(test_op.op) {
            llzkFieldDefOpSetPublicAttr(test_op.op, true);
        }
    }
}

fn new_struct(context: &TestContext) -> MlirOperation {
    unsafe {
        let ctx = context.ctx;
        let struct_name = mlirFlatSymbolRefAttrGet(context.ctx, str_ref("S"));
        let arith_constant_op_str = CString::new("struct.new").unwrap();
        let name = mlirStringRefCreateFromCString(arith_constant_op_str.as_ptr());
        let location = mlirLocationUnknownGet(ctx);
        let result = llzkStructTypeGet(struct_name);
        let mut op_state = mlirOperationStateGet(name, location);
        mlirOperationStateAddResults(&mut op_state, 1, &result);
        mlirOperationCreate(&mut op_state)
    }
}

#[rstest]
fn test_llzk_field_read_op_build(context: TestContext) {
    unsafe {
        let builder = mlirOpBuilderCreate(context.ctx);
        let location = mlirLocationUnknownGet(context.ctx);
        let index_type = mlirIndexTypeGet(context.ctx);
        let r#struct = new_struct(&context);
        let struct_value = mlirOperationGetResult(r#struct, 0);
        let op = llzkFieldReadOpBuild(builder, location, index_type, struct_value, str_ref("f"));

        mlirOperationDestroy(op);
        mlirOperationDestroy(r#struct);
        mlirOpBuilderDestroy(builder);
    }
}

#[rstest]
fn test_llzk_field_read_op_build_with_affine_map_distance(context: TestContext) {
    unsafe {
        let builder = mlirOpBuilderCreate(context.ctx);
        let location = mlirLocationUnknownGet(context.ctx);
        let index_type = mlirIndexTypeGet(context.ctx);
        let r#struct = new_struct(&context);
        let struct_value = mlirOperationGetResult(r#struct, 0);

        let mut exprs = [mlirAffineConstantExprGet(context.ctx, 1)];
        let affine_map =
            mlirAffineMapGet(context.ctx, 0, 0, exprs.len() as isize, exprs.as_mut_ptr());
        let values = &[];
        let op = llzkFieldReadOpBuildWithAffineMapDistance(
            builder,
            location,
            index_type,
            struct_value,
            str_ref("f"),
            affine_map,
            MlirValueRange {
                values: values.as_ptr(),
                size: values.len() as isize,
            },
            0,
        );

        mlirOperationDestroy(op);
        mlirOperationDestroy(r#struct);
        mlirOpBuilderDestroy(builder);
    }
}

#[rstest]
fn test_llzk_field_read_op_builder_with_const_param_distance(context: TestContext) {
    unsafe {
        let builder = mlirOpBuilderCreate(context.ctx);
        let location = mlirLocationUnknownGet(context.ctx);
        let index_type = mlirIndexTypeGet(context.ctx);
        let r#struct = new_struct(&context);
        let struct_value = mlirOperationGetResult(r#struct, 0);

        let op = llzkFieldReadOpBuildWithConstParamDistance(
            builder,
            location,
            index_type,
            struct_value,
            str_ref("f"),
            str_ref("N"),
        );

        mlirOperationDestroy(op);
        mlirOperationDestroy(r#struct);
        mlirOpBuilderDestroy(builder);
    }
}

#[rstest]
fn test_llzk_field_read_op_build_with_literal_distance(context: TestContext) {
    unsafe {
        let builder = mlirOpBuilderCreate(context.ctx);
        let location = mlirLocationUnknownGet(context.ctx);
        let index_type = mlirIndexTypeGet(context.ctx);
        let r#struct = new_struct(&context);
        let struct_value = mlirOperationGetResult(r#struct, 0);

        let op = llzkFieldReadOpBuildWithLiteralDistance(
            builder,
            location,
            index_type,
            struct_value,
            str_ref("f"),
            1,
        );

        mlirOperationDestroy(op);
        mlirOperationDestroy(r#struct);
        mlirOpBuilderDestroy(builder);
    }
}
