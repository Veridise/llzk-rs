//! `undef` dialect.

use llzk_sys::{llzkOperationIsAUndefOp, mlirGetDialectHandle__llzk__undef__};
use melior::{
    dialect::DialectHandle,
    ir::{Location, Operation, OperationRef, Type, operation::OperationBuilder},
};

/// Returns a handle to the `undef` dialect.
pub fn handle() -> DialectHandle {
    unsafe { DialectHandle::from_raw(mlirGetDialectHandle__llzk__undef__()) }
}

/// Constructs a 'undef.undef' operation.
pub fn undef<'c>(location: Location<'c>, result: Type<'c>) -> Operation<'c> {
    OperationBuilder::new("undef.undef", location)
        .add_results(&[result])
        .build()
        .expect("valid operation")
}

/// Returns wether the given operation is a 'undef.undef' operation or not.
pub fn is_undef_op(op: OperationRef) -> bool {
    unsafe { llzkOperationIsAUndefOp(op.to_raw()) }
}
