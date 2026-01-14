use mlir_sys::{mlirPassManagerAddOwnedPass, mlirPassManagerCreate, mlirPassManagerDestroy};
use rstest::rstest;

use crate::{
    mlirCreateLLZKTransformationRedundantOperationEliminationPass,
    mlirCreateLLZKTransformationRedundantReadAndWriteEliminationPass,
    mlirCreateLLZKTransformationUnusedDeclarationEliminationPass,
    mlirRegisterLLZKTransformationRedundantOperationEliminationPass,
    mlirRegisterLLZKTransformationRedundantReadAndWriteEliminationPass,
    mlirRegisterLLZKTransformationUnusedDeclarationEliminationPass,
    sanity_tests::{TestContext, context},
};

#[cfg(test)]
#[allow(unused_variables)]
mod tests {
    use super::*;
    use rstest::fixture;

    #[fixture]
    fn register_passes() {
        static ONCE: std::sync::Once = std::sync::Once::new();
        ONCE.call_once(|| unsafe {
            mlirRegisterLLZKTransformationRedundantOperationEliminationPass();
            mlirRegisterLLZKTransformationRedundantReadAndWriteEliminationPass();
            mlirRegisterLLZKTransformationUnusedDeclarationEliminationPass();
        });
    }

    #[rstest]
    fn test_mlir_register_transformation_passes_and_create(
        register_passes: (),
        context: TestContext,
    ) {
        unsafe {
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
    fn test_mlir_register_redundant_operation_elimination_pass_and_create(
        register_passes: (),
        context: TestContext,
    ) {
        unsafe {
            let manager = mlirPassManagerCreate(context.ctx);

            let pass = mlirCreateLLZKTransformationRedundantOperationEliminationPass();
            mlirPassManagerAddOwnedPass(manager, pass);

            mlirPassManagerDestroy(manager);
        }
    }
    #[rstest]
    fn test_mlir_register_redudant_read_and_write_elimination_pass_and_create(
        register_passes: (),
        context: TestContext,
    ) {
        unsafe {
            let manager = mlirPassManagerCreate(context.ctx);

            let pass = mlirCreateLLZKTransformationRedundantReadAndWriteEliminationPass();
            mlirPassManagerAddOwnedPass(manager, pass);

            mlirPassManagerDestroy(manager);
        }
    }
    #[rstest]
    fn test_mlir_register_unused_declaration_elimination_pass_and_create(
        register_passes: (),
        context: TestContext,
    ) {
        unsafe {
            let manager = mlirPassManagerCreate(context.ctx);

            let pass = mlirCreateLLZKTransformationUnusedDeclarationEliminationPass();
            mlirPassManagerAddOwnedPass(manager, pass);

            mlirPassManagerDestroy(manager);
        }
    }
}
