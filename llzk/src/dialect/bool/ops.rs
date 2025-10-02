use crate::{
    dialect::bool::{CmpPredicate, CmpPredicateAttribute},
    error::Error,
};

use melior::ir::{
    attribute::StringAttribute, operation::OperationBuilder, r#type::IntegerType, Identifier,
    Location, Operation, Value,
};

fn build_cmp_op<'c>(
    pred: CmpPredicate,
    location: Location<'c>,
    operands: &[Value<'c, '_>],
) -> Result<Operation<'c>, Error> {
    let ctx = unsafe { location.context().to_ref() };
    OperationBuilder::new(format!("bool.cmp").as_str(), location)
        .add_results(&[IntegerType::new(ctx, 1).into()])
        .add_operands(operands)
        .add_attributes(&[(
            Identifier::new(ctx, "predicate"),
            CmpPredicateAttribute::new(ctx, pred).into(),
        )])
        .build()
        .map_err(Into::into)
}

macro_rules! cmp_binop {
    ($name:ident, $pred:expr) => {
        pub fn $name<'c>(
            location: Location<'c>,
            lhs: Value<'c, '_>,
            rhs: Value<'c, '_>,
        ) -> Result<Operation<'c>, Error> {
            build_cmp_op($pred, location, &[lhs, rhs])
        }
    };
}

cmp_binop!(eq, CmpPredicate::Eq);
cmp_binop!(ge, CmpPredicate::Ge);
cmp_binop!(gt, CmpPredicate::Gt);
cmp_binop!(le, CmpPredicate::Le);
cmp_binop!(lt, CmpPredicate::Lt);
cmp_binop!(ne, CmpPredicate::Ne);

fn build_op<'c>(
    name: &str,
    location: Location<'c>,
    operands: &[Value<'c, '_>],
) -> Result<Operation<'c>, Error> {
    let ctx = location.context();
    OperationBuilder::new(format!("bool.{name}").as_str(), location)
        .add_results(&[IntegerType::new(unsafe { ctx.to_ref() }, 1).into()])
        .add_operands(operands)
        .build()
        .map_err(Into::into)
}

macro_rules! binop {
    ($name:ident) => {
        binop!($name, stringify!($name));
    };
    ($name:ident, $opname:expr) => {
        pub fn $name<'c>(
            location: Location<'c>,
            lhs: Value<'c, '_>,
            rhs: Value<'c, '_>,
        ) -> Result<Operation<'c>, Error> {
            build_op($opname, location, &[lhs, rhs])
        }
    };
}

macro_rules! unop {
    ($name:ident) => {
        unop!($name, stringify!($name));
    };
    ($name:ident, $opname:expr) => {
        pub fn $name<'c>(
            location: Location<'c>,
            value: Value<'c, '_>,
        ) -> Result<Operation<'c>, Error> {
            build_op($opname, location, &[value])
        }
    };
}

binop!(and);
binop!(or);
binop!(xor);
unop!(not);

/// Creates a `bool.assert` operation.
pub fn assert<'c>(
    location: Location<'c>,
    cond: Value<'c, '_>,
    msg: Option<&str>,
) -> Result<Operation<'c>, Error> {
    let ctx = unsafe { location.context().to_ref() };
    let mut builder = OperationBuilder::new("bool.assert", location).add_operands(&[cond]);
    if let Some(msg) = msg {
        builder = builder.add_attributes(&[(
            Identifier::new(ctx, "msg"),
            StringAttribute::new(ctx, msg).into(),
        )]);
    }

    builder.build().map_err(Into::into)
}
