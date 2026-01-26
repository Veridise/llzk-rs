use crate::{error::Error, ident};

use super::{FeltConstAttribute, FeltType};
use melior::ir::{
    Location, Operation, Value,
    operation::{OperationBuilder, OperationLike},
};

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
        binop!($name, stringify!($name));
    };
    ($name:ident, $opname:expr) => {
        #[doc = concat!("Creates a `felt.", $opname ,"` operation.")]
        pub fn $name<'c>(
            location: Location<'c>,
            lhs: Value<'c, '_>,
            rhs: Value<'c, '_>,
        ) -> Result<Operation<'c>, Error> {
            build_op($opname, location, &[lhs, rhs])
        }

        paste::paste! {
            #[doc = concat!("Return `true` iff the given op is `felt.", $opname ,"`.")]
            #[inline]
            pub fn [<is_felt_ $name>]<'c: 'a, 'a>(op: &impl OperationLike<'c, 'a>) -> bool {
                crate::operation::isa(op, concat!("felt.", $opname))
            }
        }
    };
}

macro_rules! unop {
    ($name:ident) => {
        unop!($name, stringify!($name));
    };
    ($name:ident, $opname:expr) => {
        #[doc = concat!("Creates a `felt.", $opname ,"` operation.")]
        pub fn $name<'c>(
            location: Location<'c>,
            value: Value<'c, '_>,
        ) -> Result<Operation<'c>, Error> {
            build_op($opname, location, &[value])
        }

        paste::paste! {
            #[doc = concat!("Return `true` iff the given op is `felt.", $opname ,"`.")]
            #[inline]
            pub fn [<is_felt_ $name>]<'c: 'a, 'a>(op: &impl OperationLike<'c, 'a>) -> bool {
                crate::operation::isa(op, concat!("felt.", $opname))
            }
        }
    };
}

binop!(add);
binop!(bit_and);
binop!(bit_or);
binop!(bit_xor);
binop!(div);
binop!(mul);
binop!(pow);
binop!(shl);
binop!(shr);
binop!(sintdiv);
binop!(smod);
binop!(sub);
binop!(uintdiv);
binop!(umod);
unop!(bit_not);
unop!(inv);
unop!(neg);

/// Creates a `felt.const` operation.
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

/// Return `true` iff the given op is `felt.const`.
#[inline]
pub fn is_felt_const<'c: 'a, 'a>(op: &impl OperationLike<'c, 'a>) -> bool {
    crate::operation::isa(op, "felt.const")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prelude::*;
    use quickcheck_macros::quickcheck;

    #[quickcheck]
    fn felt_const_op(value: u64) {
        let ctx = LlzkContext::new();
        let op = constant(
            Location::unknown(&ctx),
            FeltConstAttribute::new(&ctx, value),
        )
        .unwrap();
        assert!(op.verify(), "operation {op:?} failed verification");
    }

    #[quickcheck]
    fn felt_const_op_isa(value: u64) {
        let ctx = LlzkContext::new();
        let op = constant(
            Location::unknown(&ctx),
            FeltConstAttribute::new(&ctx, value),
        )
        .unwrap();
        assert!(is_felt_const(&op), "operation {op:?} failed isa test");
    }
}
