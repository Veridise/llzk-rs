//! Extensions for working with MLIR values.

use crate::error::Error;
use crate::prelude::replace_uses_of_with;
use llzk_sys::MlirValueRange;
use melior::ir::{BlockRef, OperationRef, Value, ValueLike};
use std::marker::PhantomData;

/// Wrapper around a MLIR `ValueRange`, a non-owned iterator of MLIR values.
#[derive(Debug)]
pub struct ValueRange<'c, 'a, 'b> {
    raw: MlirValueRange,
    _context: PhantomData<&'a [Value<'c, 'b>]>,
}

impl ValueRange<'_, '_, '_> {
    /// Returns the raw representation of the value range.
    pub fn to_raw(&self) -> MlirValueRange {
        self.raw
    }
}

/// Return `true` iff the given Value has any uses.
#[inline]
pub fn has_uses<'c>(val: impl ValueLike<'c> + Copy) -> bool {
    unsafe {
        let first_use = mlir_sys::mlirValueGetFirstUse(val.to_raw());
        !mlir_sys::mlirOpOperandIsNull(first_use)
    }
}

/// Returns the one user of a value.
///
/// Error if the value has more than one use or not at all.
pub fn get_single_user<'ctx, 'op>(
    value: impl ValueLike<'ctx> + Clone + std::fmt::Display,
) -> Result<OperationRef<'ctx, 'op>, Error> {
    // There is no `OpOperand` type in melior as far as I'm aware.
    let first_use = unsafe { mlir_sys::mlirValueGetFirstUse(value.to_raw()) };
    if first_use.ptr.is_null() {
        return Err(Error::GeneralError("expected value to have uses"));
    }
    let second_use = unsafe { mlir_sys::mlirOpOperandGetNextUse(first_use) };
    if !second_use.ptr.is_null() {
        return Err(Error::GeneralError("expected value to have a single use"));
    }
    unsafe { OperationRef::from_option_raw(mlir_sys::mlirOpOperandGetOwner(first_use)) }
        .ok_or_else(|| Error::GeneralError("invalid OpRef for user of value"))
}

/// Replace all uses of `orig` within the given [BlockRef] with `replacement`.
/// Based on `mlir::replaceAllUsesInRegionWith` which is not exposed through any CAPI.
pub fn replace_all_uses_in_block_with<'c>(
    block: BlockRef,
    orig: impl ValueLike<'c> + Copy,
    replacement: impl ValueLike<'c> + Copy,
) {
    unsafe {
        let mut op_use = mlir_sys::mlirValueGetFirstUse(orig.to_raw());
        while !op_use.ptr.is_null() {
            // Save next use *before* mutating (early-inc behavior)
            let next = mlir_sys::mlirOpOperandGetNextUse(op_use);
            // If the use is within the given block, replace it
            let owner = mlir_sys::mlirOpOperandGetOwner(op_use);
            if mlir_sys::mlirBlockEqual(mlir_sys::mlirOperationGetBlock(owner), block.to_raw()) {
                replace_uses_of_with(&OperationRef::from_raw(owner), orig, replacement);
            }
            // increment to next use
            op_use = next;
        }
    }
}

/// Replaces all uses of the first value with the second.
pub fn replace_all_uses<'c>(of: impl ValueLike<'c> + Copy, with: impl ValueLike<'c> + Copy) {
    unsafe { mlir_sys::mlirValueReplaceAllUsesOfWith(of.to_raw(), with.to_raw()) }
}
