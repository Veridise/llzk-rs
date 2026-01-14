//! Functions related to operations.

use crate::error::{DiagnosticError, Error};
use core::ffi::c_void;
use llzk_sys::mlirOperationWalkReverse;
use melior::{
    diagnostic::DiagnosticSeverity,
    ir::{
        ValueLike,
        operation::{OperationLike, OperationMutLike, OperationRefMut, WalkOrder, WalkResult},
    },
};
use mlir_sys::{MlirOperation, MlirWalkResult, mlirOperationWalk};

/// Walk iterator over mutable operation.
pub trait WalkOperationMutLike<'c: 'a, 'a> {
    /// Walk this operation (and all nested operations) in either pre- or
    /// post-order.
    ///
    /// The closure is called once per operation; by returning
    /// `WalkResult::Advance`/`Skip`/`Interrupt` you control the traversal.
    fn walk_mut<F>(&mut self, order: WalkOrder, callback: F)
    where
        F: for<'x, 'y> FnMut(OperationRefMut<'x, 'y>) -> WalkResult;

    /// Walk this operation (and all nested operations) in either pre- or
    /// post-order, with reverse iteration over operations at the same level.
    ///
    /// The closure is called once per operation; by returning
    /// `WalkResult::Advance`/`Skip`/`Interrupt` you control the traversal.
    fn walk_rev_mut<F>(&mut self, order: WalkOrder, callback: F)
    where
        F: for<'x, 'y> FnMut(OperationRefMut<'x, 'y>) -> WalkResult;
}

macro_rules! impl_walk_method {
    ($method_name:ident, $walk_fn:path) => {
        fn $method_name<F>(&mut self, order: WalkOrder, mut callback: F)
        where
            F: for<'x, 'y> FnMut(OperationRefMut<'x, 'y>) -> WalkResult,
        {
            // trampoline from C to Rust
            extern "C" fn tramp<'c: 'a, 'a, F: FnMut(OperationRefMut<'c, 'a>) -> WalkResult>(
                operation: MlirOperation,
                data: *mut c_void,
            ) -> MlirWalkResult {
                unsafe {
                    let callback: &mut F = &mut *(data as *mut F);
                    (callback)(OperationRefMut::from_raw(operation)) as _
                }
            }
            unsafe {
                $walk_fn(
                    self.to_raw(),
                    Some(tramp::<'c, 'a, F>),
                    &mut callback as *mut _ as *mut _,
                    order as _,
                );
            }
        }
    };
}

impl<'c: 'a, 'a, T> WalkOperationMutLike<'c, 'a> for T
where
    T: OperationMutLike<'c, 'a>,
{
    impl_walk_method!(walk_mut, mlirOperationWalk);
    impl_walk_method!(walk_rev_mut, mlirOperationWalkReverse);
}

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
#[inline]
pub fn replace_uses_of_with<'c: 'a, 'a>(
    op: &impl OperationLike<'c, 'a>,
    of: impl ValueLike<'c> + Copy,
    with: impl ValueLike<'c> + Copy,
) {
    unsafe {
        llzk_sys::mlirOperationReplaceUsesOfWith(op.to_raw(), of.to_raw(), with.to_raw());
    }
}

/// Moves the operation right after the reference op.
#[inline]
pub fn move_op_after<'c: 'a, 'a>(
    reference: &impl OperationLike<'c, 'a>,
    op: &impl OperationLike<'c, 'a>,
) {
    unsafe { mlir_sys::mlirOperationMoveAfter(op.to_raw(), reference.to_raw()) }
}

/// Erase the given operation.
#[inline]
pub fn erase_op<'c: 'a, 'a>(op: impl OperationLike<'c, 'a>) {
    unsafe {
        mlir_sys::mlirOperationDestroy(op.to_raw());
    }
}

/// Return `true` iff the given op is has the given name.
#[inline]
pub fn isa<'c: 'a, 'a>(op: &impl OperationLike<'c, 'a>, name: &str) -> bool {
    op.name().as_string_ref().as_str() == Result::Ok(name)
}
