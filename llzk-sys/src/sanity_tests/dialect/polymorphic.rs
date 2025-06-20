use std::ptr::{null, null_mut};

use mlir_sys::{
    mlirAffineConstantExprGet, mlirAffineMapAttrGet, mlirAffineMapEqual, mlirAffineMapGet,
    mlirAttributeEqual, mlirFlatSymbolRefAttrGet, mlirLocationUnknownGet, mlirOperationDestroy,
    mlirOperationVerify, mlirStringAttrGet, mlirStringRefEqual, MlirValue,
};
use rstest::rstest;

use crate::{
    llzkApplyMapOpBuild, llzkApplyMapOpBuildWithAffineExpr, llzkApplyMapOpBuildWithAffineMap,
    llzkApplyMapOpGetAffineMap, llzkApplyMapOpGetDimOperands, llzkApplyMapOpGetNumDimOperands,
    llzkApplyMapOpGetNumSymbolOperands, llzkApplyMapOpGetSymbolOperands,
    llzkOperationIsAApplyMapOp, llzkTypeIsATypeVarType, llzkTypeVarTypeGet,
    llzkTypeVarTypeGetFromAttr, llzkTypeVarTypeGetName, llzkTypeVarTypeGetNameRef,
    mlirGetDialectHandle__llzk__polymorphic__, mlirOpBuilderCreate,
    sanity_tests::{context, str_ref, TestContext},
    MlirValueRange,
};

#[test]
fn test_mlir_get_dialect_handle_llzk_polymorphic() {
    unsafe {
        mlirGetDialectHandle__llzk__polymorphic__();
    }
}

#[rstest]
fn test_llzk_type_var_type_get(context: TestContext) {
    unsafe {
        let t = llzkTypeVarTypeGet(context.ctx, str_ref("T"));
        assert_ne!(t.ptr, null());
    }
}

#[rstest]
fn test_llzk_type_is_a_type_var_type(context: TestContext) {
    unsafe {
        let t = llzkTypeVarTypeGet(context.ctx, str_ref("T"));
        assert!(llzkTypeIsATypeVarType(t));
    }
}

#[rstest]
fn test_llzk_type_var_type_get_from_attr(context: TestContext) {
    unsafe {
        let s = mlirStringAttrGet(context.ctx, str_ref("T"));
        let t = llzkTypeVarTypeGetFromAttr(context.ctx, s);
        assert_ne!(t.ptr, null());
    }
}

#[rstest]
fn test_llzk_type_var_type_get_name_ref(context: TestContext) {
    unsafe {
        let s = str_ref("T");
        let t = llzkTypeVarTypeGet(context.ctx, s);
        assert_ne!(t.ptr, null());
        assert!(mlirStringRefEqual(s, llzkTypeVarTypeGetNameRef(t)));
    }
}

#[rstest]
fn test_llzk_type_var_type_get_name(context: TestContext) {
    unsafe {
        let s = str_ref("T");
        let t = llzkTypeVarTypeGet(context.ctx, s);
        let s = mlirFlatSymbolRefAttrGet(context.ctx, s);
        assert_ne!(t.ptr, null());
        assert!(mlirAttributeEqual(s, llzkTypeVarTypeGetName(t)));
    }
}

#[rstest]
fn test_llzk_apply_map_op_build(context: TestContext) {
    unsafe {
        let builder = mlirOpBuilderCreate(context.ctx);
        let location = mlirLocationUnknownGet(context.ctx);
        let mut exprs = [mlirAffineConstantExprGet(context.ctx, 1)];
        let affine_map =
            mlirAffineMapGet(context.ctx, 0, 0, exprs.len() as isize, exprs.as_mut_ptr());
        let affine_map = mlirAffineMapAttrGet(affine_map);
        let op = llzkApplyMapOpBuild(
            builder,
            location,
            affine_map,
            MlirValueRange {
                values: null(),
                size: 0,
            },
        );
        assert_ne!(op.ptr, null_mut());
        assert!(mlirOperationVerify(op));
        mlirOperationDestroy(op);
    }
}

#[rstest]
fn test_llzk_apply_map_op_build_with_affine_map(context: TestContext) {
    unsafe {
        let builder = mlirOpBuilderCreate(context.ctx);
        let location = mlirLocationUnknownGet(context.ctx);
        let mut exprs = [mlirAffineConstantExprGet(context.ctx, 1)];
        let affine_map =
            mlirAffineMapGet(context.ctx, 0, 0, exprs.len() as isize, exprs.as_mut_ptr());
        let op = llzkApplyMapOpBuildWithAffineMap(
            builder,
            location,
            affine_map,
            MlirValueRange {
                values: null(),
                size: 0,
            },
        );
        assert_ne!(op.ptr, null_mut());
        assert!(mlirOperationVerify(op));
        mlirOperationDestroy(op);
    }
}

#[rstest]
fn test_llzk_apply_map_op_build_with_affine_expr(context: TestContext) {
    unsafe {
        let builder = mlirOpBuilderCreate(context.ctx);
        let location = mlirLocationUnknownGet(context.ctx);
        let expr = mlirAffineConstantExprGet(context.ctx, 1);
        let op = llzkApplyMapOpBuildWithAffineExpr(
            builder,
            location,
            expr,
            MlirValueRange {
                values: null(),
                size: 0,
            },
        );
        assert_ne!(op.ptr, null_mut());
        assert!(mlirOperationVerify(op));
        mlirOperationDestroy(op);
    }
}

#[rstest]
fn test_llzk_op_is_a_apply_map_op(context: TestContext) {
    unsafe {
        let builder = mlirOpBuilderCreate(context.ctx);
        let location = mlirLocationUnknownGet(context.ctx);
        let expr = mlirAffineConstantExprGet(context.ctx, 1);
        let op = llzkApplyMapOpBuildWithAffineExpr(
            builder,
            location,
            expr,
            MlirValueRange {
                values: null(),
                size: 0,
            },
        );
        assert_ne!(op.ptr, null_mut());
        assert!(mlirOperationVerify(op));
        assert!(llzkOperationIsAApplyMapOp(op));
        mlirOperationDestroy(op);
    }
}

#[rstest]
fn test_llzk_apply_map_op_get_affine_map(context: TestContext) {
    unsafe {
        let builder = mlirOpBuilderCreate(context.ctx);
        let location = mlirLocationUnknownGet(context.ctx);
        let mut exprs = [mlirAffineConstantExprGet(context.ctx, 1)];
        let affine_map =
            mlirAffineMapGet(context.ctx, 0, 0, exprs.len() as isize, exprs.as_mut_ptr());
        let op = llzkApplyMapOpBuildWithAffineMap(
            builder,
            location,
            affine_map,
            MlirValueRange {
                values: null(),
                size: 0,
            },
        );
        assert_ne!(op.ptr, null_mut());
        assert!(mlirOperationVerify(op));
        let out_affine_map = llzkApplyMapOpGetAffineMap(op);
        assert!(mlirAffineMapEqual(affine_map, out_affine_map));
        mlirOperationDestroy(op);
    }
}

fn boxed_value_range(size: isize) -> Box<[MlirValue]> {
    vec![MlirValue { ptr: null() }; size as usize].into_boxed_slice()
}

#[rstest]
fn test_llzk_apply_map_op_get_dim_operands(context: TestContext) {
    unsafe {
        let builder = mlirOpBuilderCreate(context.ctx);
        let location = mlirLocationUnknownGet(context.ctx);
        let mut exprs = [mlirAffineConstantExprGet(context.ctx, 1)];
        let affine_map =
            mlirAffineMapGet(context.ctx, 0, 0, exprs.len() as isize, exprs.as_mut_ptr());
        let op = llzkApplyMapOpBuildWithAffineMap(
            builder,
            location,
            affine_map,
            MlirValueRange {
                values: null(),
                size: 0,
            },
        );
        assert_ne!(op.ptr, null_mut());
        assert!(mlirOperationVerify(op));
        let n_dims = llzkApplyMapOpGetNumDimOperands(op);
        let mut dims = boxed_value_range(n_dims);
        llzkApplyMapOpGetDimOperands(op, dims.as_mut_ptr());
        assert_eq!(dims.len(), 0);
        mlirOperationDestroy(op);
    }
}

#[rstest]
fn test_llzk_apply_map_op_get_symbol_operands(context: TestContext) {
    unsafe {
        let builder = mlirOpBuilderCreate(context.ctx);
        let location = mlirLocationUnknownGet(context.ctx);
        let mut exprs = [mlirAffineConstantExprGet(context.ctx, 1)];
        let affine_map =
            mlirAffineMapGet(context.ctx, 0, 0, exprs.len() as isize, exprs.as_mut_ptr());
        let op = llzkApplyMapOpBuildWithAffineMap(
            builder,
            location,
            affine_map,
            MlirValueRange {
                values: null(),
                size: 0,
            },
        );
        assert_ne!(op.ptr, null_mut());
        assert!(mlirOperationVerify(op));
        let n_syms = llzkApplyMapOpGetNumSymbolOperands(op);
        let mut syms = boxed_value_range(n_syms);
        llzkApplyMapOpGetSymbolOperands(op, syms.as_mut_ptr());
        assert_eq!(syms.len(), 0);
        mlirOperationDestroy(op);
    }
}
