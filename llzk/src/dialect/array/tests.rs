use melior::{
    dialect::arith,
    ir::{
        attribute::IntegerAttribute, operation::OperationLike, Location, Module, Type,
        Value,
    },
    Context,
};
use rstest::rstest;

use crate::{
    builder::{OpBuilder, OpBuilderLike},
    dialect::array::{new, ArrayCtor},
    test::ctx,
};

use super::ArrayType;

#[rstest]
fn type_new_with_dims(ctx: Context) {
    let idx_typ = Type::index(&ctx);
    let arr_typ = ArrayType::new_with_dims(idx_typ.clone(), &[2]);

    assert_eq!(arr_typ.element_type(), idx_typ);
    assert_eq!(arr_typ.num_dims(), 1);
    assert_eq!(
        arr_typ.dim(0),
        IntegerAttribute::new(Type::index(&ctx), 2).into()
    );
}

#[rstest]
fn op_new_with_values(ctx: Context) {
    let op_builder = OpBuilder::new(&ctx);
    let arr_typ = ArrayType::new_with_dims(Type::index(&ctx), &[2]);
    let module = Module::new(Location::unknown(&ctx));
    assert_eq!(ctx, module.context());
    op_builder.set_insertion_point_at_start(module.body());
    let op = op_builder.insert(Location::unknown(&ctx), |ctx_ref, loc| {
        assert_eq!(ctx, ctx_ref);
        let op1 = op_builder.insert(loc, |ctx, loc| {
            arith::constant(
                unsafe { ctx.to_ref() },
                IntegerAttribute::new(Type::index(unsafe { ctx.to_ref() }), 1).into(),
                loc,
            )
        });

        let op2 = op_builder.insert(loc, |ctx, loc| {
            arith::constant(
                unsafe { ctx.to_ref() },
                IntegerAttribute::new(Type::index(unsafe { ctx.to_ref() }), 1).into(),
                loc,
            )
        });

        let vals: [Value; 2] = [op1.result(0).unwrap().into(), op2.result(0).unwrap().into()];
        new(&op_builder, loc, arr_typ, ArrayCtor::Values(&vals))
    });
    assert!(op.verify());
}
