use crate::{error::Error, ident};

use super::{FeltConstAttribute, FeltType};
use melior::ir::{Identifier, Location, Operation, Value, operation::OperationBuilder};

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

binop!(add);
binop!(bit_and);
binop!(bit_or);
binop!(bit_xor);
binop!(div);
binop!(mul);
binop!(r#mod, "mod");
binop!(shl);
binop!(shr);
binop!(sub);
unop!(bit_not);
unop!(inv);
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
}
