//! LLZK passes.

use llzk_macro::passes;

passes!(
    "LLZKTransformation",
    [
        mlirCreateLLZKTransformationRedundantOperationEliminationPass,
        mlirCreateLLZKTransformationRedundantReadAndWriteEliminationPass,
        mlirCreateLLZKTransformationUnusedDeclarationEliminationPass
    ]
);

passes!(
    "LLZKArrayTransformation",
    [mlirCreateLLZKArrayTransformationArrayToScalarPass]
);

passes!(
    "LLZKIncludeTransformation",
    [mlirCreateLLZKIncludeTransformationInlineIncludesPass]
);

passes!(
    "LLZKPolymorphicTransformation",
    [mlirCreateLLZKPolymorphicTransformationFlatteningPass]
);

passes!(
    "LLZKValidation",
    [mlirCreateLLZKValidationFieldWriteValidatorPass]
);

/// Registers all the available LLZK passes.
pub fn register_all_llzk_passes() {
    register_llzk_transformation_passes();
    register_llzk_array_transformation_passes();
    register_llzk_include_transformation_passes();
    register_llzk_polymorphic_transformation_passes();
    register_llzk_validation_passes();
}

#[cfg(test)]
mod tests {
    //! Tests to make sure that the expected function were generated.

    use melior::{Context, pass::PassManager};

    #[test]
    fn generated_pass_functions() {
        let ctx = Context::new();
        // Use a PassManager to manage the lifetime of the creates passes to avoid memory leaks.
        let pm = PassManager::new(&ctx);
        super::register_llzk_transformation_passes();
        super::register_redundant_operation_elimination_pass();
        super::register_redundant_read_and_write_elimination_pass();
        super::register_unused_declaration_elimination_pass();
        pm.add_pass(super::create_redundant_operation_elimination_pass());
        pm.add_pass(super::create_redundant_read_and_write_elimination_pass());
        pm.add_pass(super::create_unused_declaration_elimination_pass());

        super::register_llzk_array_transformation_passes();
        super::register_array_to_scalar_pass();
        pm.add_pass(super::create_array_to_scalar_pass());

        super::register_llzk_include_transformation_passes();
        super::register_inline_includes_pass();
        pm.add_pass(super::create_inline_includes_pass());

        super::register_llzk_polymorphic_transformation_passes();
        super::register_flattening_pass();
        pm.add_pass(super::create_flattening_pass());

        super::register_llzk_validation_passes();
        super::register_field_write_validator_pass();
        pm.add_pass(super::create_field_write_validator_pass());
    }
}
