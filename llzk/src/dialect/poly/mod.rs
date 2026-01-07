//! `poly` dialect.

pub mod r#type;

use crate::ident;
use llzk_sys::mlirGetDialectHandle__llzk__polymorphic__;
use melior::{
    dialect::DialectHandle,
    ir::{
        Location, Operation, OperationRef, Type,
        attribute::FlatSymbolRefAttribute,
        operation::{OperationBuilder, OperationLike},
    },
};

/// Returns a handle to the `poly` dialect.
pub fn handle() -> DialectHandle {
    unsafe { DialectHandle::from_raw(mlirGetDialectHandle__llzk__polymorphic__()) }
}

/// Constructs a 'poly.read_const' operation.
pub fn read_const<'c>(location: Location<'c>, symbol: &str, result: Type<'c>) -> Operation<'c> {
    let ctx = location.context();
    OperationBuilder::new("poly.read_const", location)
        .add_attributes(&[(
            ident!(ctx, "const_name"),
            FlatSymbolRefAttribute::new(unsafe { ctx.to_ref() }, symbol).into(),
        )])
        .add_results(&[result])
        .build()
        .expect("valid operation")
}

/// Returns whether the given operation is a 'poly.read_const' operation or not.
pub fn is_read_const_op(op: OperationRef) -> bool {
    op.name().as_string_ref().as_str() == Result::Ok("poly.read_const")
}
