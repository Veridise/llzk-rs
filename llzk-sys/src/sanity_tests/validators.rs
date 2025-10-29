use mlir_sys::{mlirPassManagerAddOwnedPass, mlirPassManagerCreate, mlirPassManagerDestroy};
use rstest::rstest;

use crate::{
    mlirCreateLLZKValidationFieldWriteValidatorPass,
    mlirRegisterLLZKValidationFieldWriteValidatorPass, mlirRegisterLLZKValidationPasses,
    sanity_tests::{TestContext, context},
};

#[rstest]
fn test_mlir_register_validation_passes_and_create(context: TestContext) {
    unsafe {
        let manager = mlirPassManagerCreate(context.ctx);

        let pass = mlirCreateLLZKValidationFieldWriteValidatorPass();
        mlirPassManagerAddOwnedPass(manager, pass);

        mlirPassManagerDestroy(manager);
    }
}

#[rstest]
fn test_mlir_register_validation_field_write_validator_pass_and_create(context: TestContext) {
    unsafe {
        let manager = mlirPassManagerCreate(context.ctx);

        let pass = mlirCreateLLZKValidationFieldWriteValidatorPass();
        mlirPassManagerAddOwnedPass(manager, pass);

        mlirPassManagerDestroy(manager);
    }
}
