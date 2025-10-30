use crate::{
    llzkArrayTypeGet, llzkArrayTypeGetDim, llzkArrayTypeGetElementType, llzkArrayTypeGetNumDims,
    llzkArrayTypeGetWithNumericDims, llzkCreateArrayOpBuildWithMapOperands,
    llzkCreateArrayOpBuildWithMapOperandsAndDims, llzkCreateArrayOpBuildWithValues,
    llzkTypeIsAArrayType, mlirGetDialectHandle__llzk__array__, mlirOpBuilderCreate,
    mlirOpBuilderDestroy,
    sanity_tests::{TestContext, context, load_llzk_dialects},
};
use mlir_sys::{
    MlirContext, MlirOperation, MlirType, mlirAttributeEqual, mlirDenseI32ArrayGet,
    mlirIdentifierGet, mlirIndexTypeGet, mlirIntegerAttrGet, mlirLocationUnknownGet,
    mlirNamedAttributeGet, mlirOperationCreate, mlirOperationDestroy, mlirOperationGetResult,
    mlirOperationStateAddAttributes, mlirOperationStateEnableResultTypeInference,
    mlirOperationStateGet, mlirOperationVerify, mlirStringRefCreateFromCString, mlirTypeEqual,
};
use rstest::{fixture, rstest};
use std::{ffi::CString, ptr::null};

#[test]
fn test_mlir_get_dialect_handle_llzk_array() {
    unsafe {
        mlirGetDialectHandle__llzk__array__();
    }
}

#[rstest]
fn test_llzk_array_type_get(index_type: IndexType) {
    unsafe {
        let size = mlirIntegerAttrGet(index_type.t, 1);
        let dims = [size];
        let arr_type = llzkArrayTypeGet(index_type.t, dims.len() as isize, dims.as_ptr());
        assert_ne!(arr_type.ptr, null());
    }
}

#[rstest]
fn test_llzk_type_isa_array_type_pass(index_type: IndexType) {
    unsafe {
        let size = mlirIntegerAttrGet(index_type.t, 1);
        let dims = [size];
        let arr_type = llzkArrayTypeGet(index_type.t, dims.len() as isize, dims.as_ptr());
        assert_ne!(arr_type.ptr, null());
        assert!(llzkTypeIsAArrayType(arr_type));
    }
}

#[rstest]
fn test_llzk_type_isa_array_type_fail(index_type: IndexType) {
    unsafe {
        assert!(!llzkTypeIsAArrayType(index_type.t));
    }
}

#[rstest]
fn test_llzk_array_type_get_with_numeric_dims(index_type: IndexType) {
    unsafe {
        let dims = [1, 2];
        let arr_type =
            llzkArrayTypeGetWithNumericDims(index_type.t, dims.len() as isize, dims.as_ptr());
        assert_ne!(arr_type.ptr, null());
    }
}

#[rstest]
fn test_llzk_array_type_get_element_type(index_type: IndexType) {
    unsafe {
        let dims = [1, 2];
        let arr_type =
            llzkArrayTypeGetWithNumericDims(index_type.t, dims.len() as isize, dims.as_ptr());
        assert_ne!(arr_type.ptr, null());
        let elt_type = llzkArrayTypeGetElementType(arr_type);
        assert!(mlirTypeEqual(index_type.t, elt_type));
    }
}

#[rstest]
fn test_llzk_array_type_get_num_dims(index_type: IndexType) {
    unsafe {
        let dims = [1, 2];
        let arr_type =
            llzkArrayTypeGetWithNumericDims(index_type.t, dims.len() as isize, dims.as_ptr());
        assert_ne!(arr_type.ptr, null());
        let n_dims = llzkArrayTypeGetNumDims(arr_type);
        assert_eq!(n_dims, dims.len() as isize);
    }
}

#[rstest]
fn test_llzk_array_type_get_dim(index_type: IndexType) {
    unsafe {
        let dims = [1, 2];
        let arr_type =
            llzkArrayTypeGetWithNumericDims(index_type.t, dims.len() as isize, dims.as_ptr());
        assert_ne!(arr_type.ptr, null());
        let out_dim = llzkArrayTypeGetDim(arr_type, 0);
        let dim_as_attr = mlirIntegerAttrGet(index_type.t, dims[0]);
        assert!(mlirAttributeEqual(out_dim, dim_as_attr));
    }
}

#[rstest]
fn test_llzk_create_array_op_build_with_values(context: TestContext, #[values(&[1])] dims: &[i64]) {
    unsafe {
        use crate::mlirOpBuilderDestroy;

        let elt_type = mlirIndexTypeGet(context.ctx);
        let test_type = test_array(elt_type, &dims);
        let n_elements: i64 = dims.iter().product();
        let ops = create_n_ops(context.ctx, n_elements, elt_type);
        let values = ops
            .iter()
            .map(|op| mlirOperationGetResult(*op, 0))
            .collect::<Vec<_>>();
        let builder = mlirOpBuilderCreate(context.ctx);
        let location = mlirLocationUnknownGet(context.ctx);
        let create_array_op = llzkCreateArrayOpBuildWithValues(
            builder,
            location,
            test_type,
            values.len() as isize,
            values.as_ptr(),
        );
        for op in &ops {
            assert!(mlirOperationVerify(*op));
        }
        assert!(mlirOperationVerify(create_array_op));

        mlirOperationDestroy(create_array_op);
        for op in ops {
            mlirOperationDestroy(op);
        }
        mlirOpBuilderDestroy(builder);
    }
}

#[rstest]
fn test_llzk_create_array_op_build_with_map_operands(
    context: TestContext,
    #[values(&[1])] dims: &[i64],
) {
    load_llzk_dialects(&context);
    unsafe {
        use crate::mlirOpBuilderDestroy;

        let elt_type = mlirIndexTypeGet(context.ctx);
        let test_type = test_array(elt_type, &dims);

        let builder = mlirOpBuilderCreate(context.ctx);
        let location = mlirLocationUnknownGet(context.ctx);
        let dims_per_map = mlirDenseI32ArrayGet(context.ctx, 0, null());

        let op = llzkCreateArrayOpBuildWithMapOperands(
            builder,
            location,
            test_type,
            0,
            null(),
            dims_per_map,
        );

        assert!(mlirOperationVerify(op));
        mlirOperationDestroy(op);
        mlirOpBuilderDestroy(builder);
    }
}

#[rstest]
fn test_llzk_create_array_op_build_with_map_operands_and_dims(
    context: TestContext,
    #[values(&[1])] dims: &[i64],
) {
    load_llzk_dialects(&context);
    unsafe {
        use crate::mlirOpBuilderDestroy;

        let elt_type = mlirIndexTypeGet(context.ctx);
        let test_type = test_array(elt_type, &dims);

        let builder = mlirOpBuilderCreate(context.ctx);
        let location = mlirLocationUnknownGet(context.ctx);

        let op = llzkCreateArrayOpBuildWithMapOperandsAndDims(
            builder,
            location,
            test_type,
            0,
            null(),
            0,
            null(),
        );

        assert!(mlirOperationVerify(op));
        mlirOperationDestroy(op);

        mlirOpBuilderDestroy(builder);
    }
}

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

fn test_array(elt: MlirType, dims: &[i64]) -> MlirType {
    unsafe { llzkArrayTypeGetWithNumericDims(elt, dims.len() as isize, dims.as_ptr()) }
}

fn create_n_ops(ctx: MlirContext, n_ops: i64, elt_type: MlirType) -> Vec<MlirOperation> {
    unsafe {
        let arith_constant_op_str = CString::new("arith.constant").unwrap();
        let value_str = CString::new("value").unwrap();

        let name = mlirStringRefCreateFromCString(arith_constant_op_str.as_ptr());
        let attr_name = mlirIdentifierGet(ctx, mlirStringRefCreateFromCString(value_str.as_ptr()));
        let location = mlirLocationUnknownGet(ctx);
        (0..n_ops)
            .map(|n| {
                let attr = mlirNamedAttributeGet(attr_name, mlirIntegerAttrGet(elt_type, n));
                let mut op_state = mlirOperationStateGet(name, location);
                mlirOperationStateAddAttributes(&mut op_state, 1, &attr);
                mlirOperationStateEnableResultTypeInference(&mut op_state);

                let created_op = mlirOperationCreate(&mut op_state);

                created_op
            })
            .collect()
    }
}
