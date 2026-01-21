use llzk::{
    builder::OpBuilder,
    dialect::array::ArrayCtor,
    prelude::melior_dialects::arith,
    prelude::*,
    value_ext::{OwningValueRange, ValueRange},
};
use melior::ir::{Location, Type, r#type::FunctionType};

mod common;

#[test]
fn array_new_affine_map() {
    common::setup();
    let context = LlzkContext::new();
    let location = Location::unknown(&context);
    let module = llzk_module(location);
    let index_type = Type::index(&context);
    let f = function::def(
        location,
        "array_new",
        FunctionType::new(&context, &[index_type, index_type], &[]),
        &[],
        None,
    )
    .unwrap();
    {
        let block_arg = (index_type, location);
        let block = Block::new(&[block_arg, block_arg]);
        let arg0: Value = block.argument(0).unwrap().into();
        let arg1: Value = block.argument(1).unwrap().into();
        let builder = OpBuilder::new(&context);
        let affine_map = Attribute::parse(&context, "affine_map<()[s0, s1] -> (s0 + s1)>")
            .expect("failed to parse affine_map");
        let array_type = ArrayType::new(index_type, &[affine_map]);
        let owning_value_range = OwningValueRange::from([arg0, arg1].as_slice());
        let value_range = ValueRange::try_from(&owning_value_range).unwrap();
        let _array = block.append_operation(array::new(
            &builder,
            location,
            array_type,
            ArrayCtor::MapDimSlice(&[value_range], &[0]),
        ));
        block.append_operation(function::r#return(location, &[]));
        f.region(0)
            .expect("function.def must have at least 1 region")
            .append_block(block);
    }

    assert_eq!(f.region_count(), 1);
    let f = module.body().append_operation(f.into());
    assert!(f.verify());
    log::info!("Op passed verification");
    let ir = format!("{f}");
    let expected = concat!(
        "function.def @array_new(%arg0: index, %arg1: index) {\n",
        "  %array = array.new{()[%arg0, %arg1]} : <affine_map<()[s0, s1] -> (s0 + s1)> x index> \n",
        "  function.return\n",
        "}"
    );
    assert_eq!(ir, expected);
}

#[test]
fn array_len() {
    common::setup();
    let dim = 77;

    let ctx = LlzkContext::new();
    let unknown = Location::unknown(&ctx);
    let index_ty = Type::index(&ctx);
    let ty = ArrayType::new_with_dims(index_ty, &[dim]);
    let op = array::new(&OpBuilder::new(&ctx), unknown, ty, ArrayCtor::Values(&[]));
    assert_eq!(1, op.result_count(), "op {op} must only have one result");
    let arr_ref = op.result(0).unwrap();
    let arr_dim_op = arith::constant(&ctx, IntegerAttribute::new(index_ty, 0).into(), unknown);
    assert_eq!(
        1,
        arr_dim_op.result_count(),
        "op {arr_dim_op} must only have one result"
    );
    let arr_dim = arr_dim_op.result(0).unwrap();
    let len = array::len(unknown, arr_ref.into(), arr_dim.into());
    assert!(len.verify(), "op {len} failed to verify");
    assert!(array::is_array_len(&len));
}
