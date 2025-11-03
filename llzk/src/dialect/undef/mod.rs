//! `undef` dialect.

use llzk_sys::mlirGetDialectHandle__llzk__undef__;
use melior::{
    dialect::DialectHandle,
    ir::{Location, Operation, Type, operation::OperationBuilder},
};

/// Returns a handle to the `undef` dialect.
pub fn handle() -> DialectHandle {
    unsafe { DialectHandle::from_raw(mlirGetDialectHandle__llzk__undef__()) }
}

/// Constructs a 'undef.undef' operation.
pub fn undef<'c>(location: Location<'c>, result: Type<'c>) -> Operation<'c> {
    let ctx = location.context();
    OperationBuilder::new("undef.undef", location)
        .add_results(&[result])
        .build()
        .expect("valid operation")
}
