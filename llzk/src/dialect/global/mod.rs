use llzk_sys::mlirGetDialectHandle__llzk__global__;
use melior::{
    dialect::DialectHandle,
    ir::{
        Attribute, Identifier, Location, Operation, Type, Value,
        attribute::{StringAttribute, TypeAttribute},
        operation::OperationBuilder,
    },
};

use crate::{ident, symbol_ref::SymbolRefAttribute};

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
    let context = unsafe { location.context().to_ref() };
    let mut attrs = vec![
        (
            Identifier::new(context, "sym_name"),
            StringAttribute::new(context, name).into(),
        ),
        (
            Identifier::new(context, "type"),
            TypeAttribute::new(r#type).into(),
        ),
    ];
    if constant {
        attrs.push((
            Identifier::new(context, "constant"),
            Attribute::unit(context),
        ));
    }
    if let Some(initial_value) = initial_value {
        attrs.push((Identifier::new(context, "initial_value"), initial_value));
    }
    OperationBuilder::new("global.def", location)
        .add_attributes(&attrs)
        .build()
        .expect("valid operation")
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
