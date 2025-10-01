use std::{ffi::c_void, ptr::null_mut};

use mlir_sys::{MlirBlock, MlirOperation, MlirRegion};
use rstest::rstest;

use crate::{
    MlirOpBuilderInsertPoint, mlirOpBuilderCreate, mlirOpBuilderCreateWithListener,
    mlirOpBuilderDestroy, mlirOpBuilderListenerCreate, mlirOpBuilderListenerDestroy,
    sanity_tests::{TestContext, context},
};

#[rstest]
fn test_mlir_op_builder_create(context: TestContext) {
    unsafe {
        let builder = mlirOpBuilderCreate(context.ctx);
        mlirOpBuilderDestroy(builder);
    }
}

#[rstest]
fn test_mlir_op_builder_create_with_listener(context: TestContext) {
    unsafe {
        let listener =
            mlirOpBuilderListenerCreate(Some(test_callback1), Some(test_callback2), null_mut());
        let builder = mlirOpBuilderCreateWithListener(context.ctx, listener);

        mlirOpBuilderDestroy(builder);
        mlirOpBuilderListenerDestroy(listener);
    }
}

#[rstest]
fn test_mlir_op_builder_listener_create() {
    unsafe {
        let listener =
            mlirOpBuilderListenerCreate(Some(test_callback1), Some(test_callback2), null_mut());
        mlirOpBuilderListenerDestroy(listener);
    }
}

unsafe extern "C" fn test_callback1(_: MlirOperation, _: MlirOpBuilderInsertPoint, _: *mut c_void) {
}
unsafe extern "C" fn test_callback2(_: MlirBlock, _: MlirRegion, _: MlirBlock, _: *mut c_void) {}
