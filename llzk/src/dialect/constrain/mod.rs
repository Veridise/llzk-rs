//! `constrain` dialect.

use llzk_sys::mlirGetDialectHandle__llzk__constrain__;
use melior::{
    dialect::DialectHandle,
    ir::{
        Location, Operation, Value,
        operation::{OperationBuilder, OperationLike},
    },
};

/// Returns a handle to the `constrain` dialect.
pub fn handle() -> DialectHandle {
    unsafe { DialectHandle::from_raw(mlirGetDialectHandle__llzk__constrain__()) }
}

/// Creates a `constrain.eq` operation.
pub fn eq<'c>(location: Location<'c>, lhs: Value<'c, '_>, rhs: Value<'c, '_>) -> Operation<'c> {
    OperationBuilder::new("constrain.eq", location)
        .add_operands(&[lhs, rhs])
        .build()
        .expect("valid operation")
}

/// Return `true` iff the given op is `constrain.eq`.
#[inline]
pub fn is_constrain_eq<'c: 'a, 'a>(op: &impl OperationLike<'c, 'a>) -> bool {
    crate::operation::isa(op, "constrain.eq")
}

/// Creates a `constrain.in` operation.
pub fn r#in<'c>(location: Location<'c>, lhs: Value<'c, '_>, rhs: Value<'c, '_>) -> Operation<'c> {
    OperationBuilder::new("constrain.in", location)
        .add_operands(&[lhs, rhs])
        .build()
        .expect("valid operation")
}

/// Return `true` iff the given op is `constrain.in`.
#[inline]
pub fn is_constrain_in<'c: 'a, 'a>(op: &impl OperationLike<'c, 'a>) -> bool {
    crate::operation::isa(op, "constrain.in")
}
