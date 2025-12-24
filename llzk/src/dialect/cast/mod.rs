//! `cast` dialect.

use llzk_sys::mlirGetDialectHandle__llzk__cast__;
use melior::dialect::DialectHandle;
use melior::ir::{Location, Operation, Type, Value, operation::OperationBuilder};

use crate::prelude::FeltType;

/// Returns a handle to the `cast` dialect.
pub fn handle() -> DialectHandle {
    unsafe { DialectHandle::from_raw(mlirGetDialectHandle__llzk__cast__()) }
}

/// Creates a 'cast.tofelt' operation.
pub fn tofelt<'c>(location: Location<'c>, result: Type<'c>, val: Value<'c, '_>) -> Operation<'c> {
    let ctx = unsafe { location.context().to_ref() };
    OperationBuilder::new("cast.tofelt", location)
        .add_results(&[FeltType::new(ctx).into()])
        .add_operands(&[val])
        .build()
        .expect("valid operation")
}

/// Creates a 'cast.toindex' operation.
pub fn toindex<'c>(location: Location<'c>, val: Value<'c, '_>) -> Operation<'c> {
    let ctx = unsafe { location.context().to_ref() };
    OperationBuilder::new("cast.toindex", location)
        .add_results(&[Type::index(ctx)])
        .add_operands(&[val])
        .build()
        .expect("valid operation")
}
