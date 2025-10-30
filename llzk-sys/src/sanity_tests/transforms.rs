use mlir_sys::{mlirPassManagerAddOwnedPass, mlirPassManagerCreate, mlirPassManagerDestroy};
use rstest::rstest;

use crate::{
    mlirCreateLLZKTransformationRedundantOperationEliminationPass,
    mlirCreateLLZKTransformationRedundantReadAndWriteEliminationPass,
    mlirCreateLLZKTransformationUnusedDeclarationEliminationPass,
    mlirRegisterLLZKTransformationPasses,
    mlirRegisterLLZKTransformationRedundantOperationEliminationPass,
    mlirRegisterLLZKTransformationRedundantReadAndWriteEliminationPass,
    mlirRegisterLLZKTransformationUnusedDeclarationEliminationPass,
    sanity_tests::{TestContext, context},
};

#[rstest]
fn test_mlir_register_transformation_passes_and_create(context: TestContext) {
    unsafe {
        mlirRegisterLLZKTransformationPasses();
        let manager = mlirPassManagerCreate(context.ctx);

        let pass1 = mlirCreateLLZKTransformationRedundantOperationEliminationPass();
        let pass2 = mlirCreateLLZKTransformationRedundantReadAndWriteEliminationPass();
        let pass3 = mlirCreateLLZKTransformationUnusedDeclarationEliminationPass();
        mlirPassManagerAddOwnedPass(manager, pass1);
        mlirPassManagerAddOwnedPass(manager, pass2);
        mlirPassManagerAddOwnedPass(manager, pass3);

        mlirPassManagerDestroy(manager);
    }
}

#[rstest]
fn test_mlir_register_redundant_operation_elimination_pass_and_create(context: TestContext) {
    unsafe {
        mlirRegisterLLZKTransformationRedundantOperationEliminationPass();
        let manager = mlirPassManagerCreate(context.ctx);

        let pass = mlirCreateLLZKTransformationRedundantOperationEliminationPass();
        mlirPassManagerAddOwnedPass(manager, pass);

        mlirPassManagerDestroy(manager);
    }
}
#[rstest]
fn test_mlir_register_redudant_read_and_write_elimination_pass_and_create(context: TestContext) {
    unsafe {
        mlirRegisterLLZKTransformationRedundantReadAndWriteEliminationPass();
        let manager = mlirPassManagerCreate(context.ctx);

        let pass = mlirCreateLLZKTransformationRedundantReadAndWriteEliminationPass();
        mlirPassManagerAddOwnedPass(manager, pass);

        mlirPassManagerDestroy(manager);
    }
}
#[rstest]
fn test_mlir_register_unused_declaration_elimination_pass_and_create(context: TestContext) {
    unsafe {
        mlirRegisterLLZKTransformationUnusedDeclarationEliminationPass();
        let manager = mlirPassManagerCreate(context.ctx);

        let pass = mlirCreateLLZKTransformationUnusedDeclarationEliminationPass();
        mlirPassManagerAddOwnedPass(manager, pass);

        mlirPassManagerDestroy(manager);
    }
}
