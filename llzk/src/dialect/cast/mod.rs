//! `cast` dialect.

use llzk_sys::mlirGetDialectHandle__llzk__cast__;
use melior::dialect::DialectHandle;
use melior::ir::r#type::IntegerType;
use melior::ir::{Location, Operation, Type, Value, operation::OperationBuilder};

use crate::prelude::FeltType;

/// Returns a handle to the `cast` dialect.
pub fn handle() -> DialectHandle {
    unsafe { DialectHandle::from_raw(mlirGetDialectHandle__llzk__cast__()) }
}

/// Creates a 'cast.tofelt' operation.
pub fn tofelt<'c>(location: Location<'c>, val: Value<'c, '_>) -> Operation<'c> {
    let ctx = location.context();
    OperationBuilder::new("cast.tofelt", location)
        .add_results(&[FeltType::new(unsafe { ctx.to_ref() }).into()])
        .add_operands(&[val])
        .build()
        .expect("valid operation")
}

/// Creates a 'cast.toindex' operation.
pub fn toindex<'c>(location: Location<'c>, val: Value<'c, '_>) -> Operation<'c> {
    let ctx = location.context();
    OperationBuilder::new("cast.toindex", location)
        .add_results(&[Type::index(unsafe { ctx.to_ref() })])
        .add_operands(&[val])
        .build()
        .expect("valid operation")
}

/// Creates a 'cast.toint' operation.
pub fn toint<'c>(
    location: Location<'c>,
    result: IntegerType<'c>,
    val: Value<'c, '_>,
) -> Operation<'c> {
    OperationBuilder::new("cast.toint", location)
        .add_results(&[result.into()])
        .add_operands(&[val])
        .build()
        .expect("valid operation")
}
