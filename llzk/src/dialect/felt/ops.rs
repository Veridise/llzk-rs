use crate::{error::Error, ident};

use super::{FeltConstAttribute, FeltType};
use melior::ir::{operation::OperationBuilder, Identifier, Location, Operation, Value};

fn build_op<'c>(
    name: &str,
    location: Location<'c>,
    operands: &[Value<'c, '_>],
) -> Result<Operation<'c>, Error> {
    let ctx = location.context();
    OperationBuilder::new(format!("felt.{name}").as_str(), location)
        .add_results(&[FeltType::new(unsafe { ctx.to_ref() }).into()])
        .add_operands(operands)
        .build()
        .map_err(Into::into)
}

macro_rules! binop {
    ($name:ident) => {
        pub fn $name<'c>(
            location: Location<'c>,
            lhs: Value<'c, '_>,
            rhs: Value<'c, '_>,
        ) -> Result<Operation<'c>, Error> {
            build_op(stringify!($name), location, &[lhs, rhs])
        }
    };
}

macro_rules! unop {
    ($name:ident) => {
        pub fn $name<'c>(
            location: Location<'c>,
            value: Value<'c, '_>,
        ) -> Result<Operation<'c>, Error> {
            build_op(stringify!($name), location, &[value])
        }
    };
}

binop!(add);
binop!(sub);
binop!(mul);
unop!(neg);

pub fn constant<'c>(
    location: Location<'c>,
    value: FeltConstAttribute<'c>,
) -> Result<Operation<'c>, Error> {
    let ctx = location.context();
    OperationBuilder::new("felt.const", location)
        .add_results(&[FeltType::new(unsafe { ctx.to_ref() }).into()])
        .add_attributes(&[(ident!(ctx, "value"), value.into())])
        .build()
        .map_err(Into::into)
}
