//! `poly` dialect operations and helper functions.

use crate::{
    builder::{OpBuilder, OpBuilderLike},
    ident,
    value_ext::{OwningValueRange, ValueRange},
};
use llzk_sys::llzkApplyMapOpBuildWithAffineMap;
use melior::ir::{
    Attribute, AttributeLike, Location, Operation, Type, Value, ValueLike,
    attribute::FlatSymbolRefAttribute,
    operation::{OperationBuilder, OperationLike},
};

/// Constructs a 'poly.applymap' operation.
pub fn applymap<'c>(
    location: Location<'c>,
    map: Attribute<'c>,
    map_operands: &[Value<'c, '_>],
) -> Operation<'c> {
    let ctx = location.context();
    let builder = OpBuilder::new(unsafe { ctx.to_ref() });
    let value_range = OwningValueRange::from(map_operands);
    assert!(unsafe { mlir_sys::mlirAttributeIsAAffineMap(map.to_raw()) });
    unsafe {
        Operation::from_raw(llzkApplyMapOpBuildWithAffineMap(
            builder.to_raw(),
            location.to_raw(),
            mlir_sys::mlirAffineMapAttrGetValue(map.to_raw()),
            ValueRange::try_from(&value_range).unwrap().to_raw(),
        ))
    }
}

/// Return `true` iff the given op is `poly.applymap`.
#[inline]
pub fn is_applymap_op<'c: 'a, 'a>(op: &impl OperationLike<'c, 'a>) -> bool {
    crate::operation::isa(op, "poly.applymap")
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

/// Constructs a 'poly.unifiable_cast' operation.
pub fn unifiable_cast<'c>(location: Location<'c>, symbol: &str, result: Type<'c>) -> Operation<'c> {
    let ctx = location.context();
    OperationBuilder::new("poly.unifiable_cast", location)
        .add_attributes(&[(
            ident!(ctx, "const_name"),
            FlatSymbolRefAttribute::new(unsafe { ctx.to_ref() }, symbol).into(),
        )])
        .add_results(&[result])
        .build()
        .expect("valid operation")
}

/// Return `true` iff the given op is `poly.unifiable_cast`.
#[inline]
pub fn is_unifiable_cast_op<'c: 'a, 'a>(op: &impl OperationLike<'c, 'a>) -> bool {
    crate::operation::isa(op, "poly.unifiable_cast")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prelude::*;
    use melior::dialect::arith;
    use quickcheck_macros::quickcheck;

    // #[quickcheck]
    // fn applymap(dim: i64) {
    //     let ctx = LlzkContext::new();
    //     let unknown = Location::unknown(&ctx);
    //     let index_ty = Type::index(&ctx);
    //     let ty = ArrayType::new_with_dims(index_ty, &[dim]);
    //     let op = new(
    //         &OpBuilder::new(&ctx),
    //         unknown,
    //         ty,
    //         ArrayCtor::Values(&[])
    //     );
    //     assert_eq!(1, op.result_count(), "op {op} must only have one result");
    //     let arr_ref = op.result(0).unwrap();
    //     let arr_dim_op = arith::constant(&ctx, IntegerAttribute::new(index_ty, 0).into(), unknown);
    //     assert_eq!(1, arr_dim_op.result_count(), "op {arr_dim_op} must only have one result");
    //     let arr_dim = arr_dim_op.result(0).unwrap();
    //     let len = len(&ctx, unknown, arr_ref.into(), arr_dim.into());
    //     assert!(len.verify(), "op {len} failed to verify");
    // }

    // #[quickcheck]
    // fn unifiable_cast(dim: i64) {
    //     let ctx = LlzkContext::new();
    //     let unknown = Location::unknown(&ctx);
    //     let index_ty = Type::index(&ctx);
    //     let ty = ArrayType::new_with_dims(index_ty, &[dim]);
    //     let op = new(
    //         &OpBuilder::new(&ctx),
    //         unknown,
    //         ty,
    //         ArrayCtor::Values(&[])
    //     );
    //     assert_eq!(1, op.result_count(), "op {op} must only have one result");
    //     let arr_ref = op.result(0).unwrap();
    //     let arr_dim_op = arith::constant(&ctx, IntegerAttribute::new(index_ty, 0).into(), unknown);
    //     assert_eq!(1, arr_dim_op.result_count(), "op {arr_dim_op} must only have one result");
    //     let arr_dim = arr_dim_op.result(0).unwrap();
    //     let len = len(&ctx, unknown, arr_ref.into(), arr_dim.into());
    //     assert!(len.verify(), "op {len} failed to verify");
    // }
}
