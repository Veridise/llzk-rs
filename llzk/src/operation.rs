//! Functions related to operations.

use crate::error::{DiagnosticError, Error};
use melior::{
    diagnostic::DiagnosticSeverity,
    ir::{Value, ValueLike, operation::OperationLike},
};

/// Verifies the operation, returning an error if it failed.
pub fn verify_operation<'c: 'a, 'a>(op: &impl OperationLike<'c, 'a>) -> Result<(), Error> {
    if op.verify() {
        return Ok(());
    }
    Err(Error::OpVerificationFailed {
        name: op.name().as_string_ref().as_str()?.to_owned(),
        ir: op.to_string(),
        location: op.location().to_string(),
        diags: None,
    })
}

/// Verifies the operation, returning an error with the emitted diagnostics if it failed.
pub fn verify_operation_with_diags<'c: 'a, 'a>(
    op: &impl OperationLike<'c, 'a>,
) -> Result<(), Error> {
    let mut errors: Vec<DiagnosticError> = Vec::with_capacity(1);
    let ctx_ref = op.context();
    let id = unsafe { ctx_ref.to_ref() }.attach_diagnostic_handler(|diag| {
        if matches!(diag.severity(), DiagnosticSeverity::Error) {
            errors.push(diag.into());
        }
        // Return false to propagate the diagnostic to other handlers.
        false
    });

    let result = verify_operation(op).map_err(|mut err| {
        match &mut err {
            Error::OpVerificationFailed { diags, .. } if !errors.is_empty() => {
                diags.get_or_insert_default().extend(errors)
            }
            _ => {}
        };
        err
    });
    unsafe { ctx_ref.to_ref() }.detach_diagnostic_handler(id);
    result
}

/// Replace uses of 'of' value with the 'with' value inside the 'op' operation.
pub fn replace_uses_of_with<'c: 'a, 'a>(
    op: &impl OperationLike<'c, 'a>,
    of: Value<'c, 'a>,
    with: Value<'c, 'a>,
) {
    unsafe {
        llzk_sys::mlirOperationReplaceUsesOfWith(op.to_raw(), of.to_raw(), with.to_raw());
    }
}
