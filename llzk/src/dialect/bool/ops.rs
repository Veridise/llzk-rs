use crate::{
    dialect::bool::{CmpPredicate, CmpPredicateAttribute},
    error::Error,
    ident,
};

use melior::ir::{
    Location, Operation, Value,
    attribute::StringAttribute,
    operation::{OperationBuilder, OperationLike},
    r#type::IntegerType,
};

fn build_cmp_op<'c>(
    pred: CmpPredicate,
    location: Location<'c>,
    operands: &[Value<'c, '_>],
) -> Result<Operation<'c>, Error> {
    let ctx = location.context();
    OperationBuilder::new("bool.cmp", location)
        .add_results(&[IntegerType::new(unsafe { ctx.to_ref() }, 1).into()])
        .add_operands(operands)
        .add_attributes(&[(
            ident!(ctx, "predicate"),
            CmpPredicateAttribute::new(unsafe { ctx.to_ref() }, pred).into(),
        )])
        .build()
        .map_err(Into::into)
}

macro_rules! cmp_binop {
    ($name:ident, $pred:expr) => {
        #[doc = concat!("Creates a `bool.cmp ", stringify!($name) ,"` operation.")]
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

/// Return `true` iff the given op is `bool.cmp`.
#[inline]
pub fn is_bool_cmp<'c: 'a, 'a>(op: &impl OperationLike<'c, 'a>) -> bool {
    crate::operation::isa(op, "bool.cmp")
}

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
        #[doc = concat!("Creates a `bool.", $opname ,"` operation.")]
        pub fn $name<'c>(
            location: Location<'c>,
            lhs: Value<'c, '_>,
            rhs: Value<'c, '_>,
        ) -> Result<Operation<'c>, Error> {
            build_op($opname, location, &[lhs, rhs])
        }

        paste::paste! {
            #[doc = concat!("Return `true` iff the given op is `bool.", $opname ,"`.")]
            #[inline]
            pub fn [<is_bool_ $name>]<'c: 'a, 'a>(op: &impl OperationLike<'c, 'a>) -> bool {
                crate::operation::isa(op, concat!("bool.", $opname))
            }
        }
    };
}

macro_rules! unop {
    ($name:ident) => {
        unop!($name, stringify!($name));
    };
    ($name:ident, $opname:expr) => {
        #[doc = concat!("Creates a `bool.", $opname ,"` operation.")]
        pub fn $name<'c>(
            location: Location<'c>,
            value: Value<'c, '_>,
        ) -> Result<Operation<'c>, Error> {
            build_op($opname, location, &[value])
        }

        paste::paste! {
            #[doc = concat!("Return `true` iff the given op is `bool.", $opname ,"`.")]
            #[inline]
            pub fn [<is_bool_ $name>]<'c: 'a, 'a>(op: &impl OperationLike<'c, 'a>) -> bool {
                crate::operation::isa(op, concat!("bool.", $opname))
            }
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
    let ctx = location.context();
    let mut builder = OperationBuilder::new("bool.assert", location).add_operands(&[cond]);
    if let Some(msg) = msg {
        builder = builder.add_attributes(&[(
            ident!(ctx, "msg"),
            StringAttribute::new(unsafe { ctx.to_ref() }, msg).into(),
        )]);
    }
    builder.build().map_err(Into::into)
}

/// Return `true` iff the given op is `bool.assert`.
#[inline]
pub fn is_bool_assert<'c: 'a, 'a>(op: &impl OperationLike<'c, 'a>) -> bool {
    crate::operation::isa(op, "bool.assert")
}
