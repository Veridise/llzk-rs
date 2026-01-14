//! `global` dialect.

use llzk_sys::mlirGetDialectHandle__llzk__global__;
use melior::{
    dialect::DialectHandle,
    ir::{
        Attribute, Location, Operation, Type, Value,
        attribute::{StringAttribute, TypeAttribute},
        operation::{OperationBuilder, OperationLike},
    },
};

use crate::{ident, symbol_ref::SymbolRefAttribute};

/// Returns a handle to the `global` dialect.
pub fn handle() -> DialectHandle {
    unsafe { DialectHandle::from_raw(mlirGetDialectHandle__llzk__global__()) }
}

/// Constructs a 'global.def' operation.
pub fn def<'c>(
    location: Location<'c>,
    name: &str,
    r#type: Type<'c>,
    constant: bool,
    initial_value: Option<Attribute<'c>>,
) -> Operation<'c> {
    let ctx = location.context();
    let mut attrs = vec![
        (
            ident!(ctx, "sym_name"),
            StringAttribute::new(unsafe { ctx.to_ref() }, name).into(),
        ),
        (ident!(ctx, "type"), TypeAttribute::new(r#type).into()),
    ];
    if constant {
        attrs.push((
            ident!(ctx, "constant"),
            Attribute::unit(unsafe { ctx.to_ref() }),
        ));
    }
    if let Some(initial_value) = initial_value {
        attrs.push((ident!(ctx, "initial_value"), initial_value));
    }
    OperationBuilder::new("global.def", location)
        .add_attributes(&attrs)
        .build()
        .expect("valid operation")
}

/// Return `true` iff the given op is `global.def`.
#[inline]
pub fn is_global_def<'c: 'a, 'a>(op: &impl OperationLike<'c, 'a>) -> bool {
    crate::operation::isa(op, "global.def")
}

/// Constructs a 'global.read' operation.
pub fn read<'c>(
    location: Location<'c>,
    name: SymbolRefAttribute<'c>,
    result: Type<'c>,
) -> Operation<'c> {
    let ctx = location.context();
    OperationBuilder::new("global.read", location)
        .add_attributes(&[(ident!(ctx, "name_ref"), name.into())])
        .add_results(&[result])
        .build()
        .expect("valid operation")
}

/// Return `true` iff the given op is `global.read`.
#[inline]
pub fn is_global_read<'c: 'a, 'a>(op: &impl OperationLike<'c, 'a>) -> bool {
    crate::operation::isa(op, "global.read")
}

/// Constructs a 'global.write' operation.
pub fn write<'c>(
    location: Location<'c>,
    name: SymbolRefAttribute<'c>,
    value: Value<'c, '_>,
) -> Operation<'c> {
    let ctx = location.context();
    OperationBuilder::new("global.write", location)
        .add_attributes(&[(ident!(ctx, "name_ref"), name.into())])
        .add_operands(&[value])
        .build()
        .expect("valid operation")
}

/// Return `true` iff the given op is `global.write`.
#[inline]
pub fn is_global_write<'c: 'a, 'a>(op: &impl OperationLike<'c, 'a>) -> bool {
    crate::operation::isa(op, "global.write")
}
