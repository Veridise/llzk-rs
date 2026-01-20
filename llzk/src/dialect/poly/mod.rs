//! `poly` dialect.

pub mod r#type;

use crate::ident;
use llzk_sys::mlirGetDialectHandle__llzk__polymorphic__;
use melior::{
    dialect::DialectHandle,
    ir::{
        Location, Operation, Type,
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

/// Return `true` iff the given op is `poly.read_const`.
#[inline]
pub fn is_read_const_op<'c: 'a, 'a>(op: &impl OperationLike<'c, 'a>) -> bool {
    crate::operation::isa(op, "poly.read_const")
}
