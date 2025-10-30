use llzk::prelude::*;
use melior::ir::{
    Location, Type,
    r#type::{FunctionType, IntegerType},
};

mod common;

fn create_undef(context: &LlzkContext, ty: Type) -> String {
    common::setup();
    let module = llzk_module(Location::unknown(&context));
    let loc = Location::unknown(&context);
    let f = function::def(
        loc,
        "f_undef_test",
        FunctionType::new(&context, &[], &[ty]),
        &[],
        None,
    )
    .unwrap();
    f.set_allow_witness_attr(true);
    {
        let block = Block::new(&[]);
        let undef_op = block.append_operation(undef::undef(loc, ty.into()));
        block.append_operation(function::r#return(
            loc,
            &[undef_op.result(0).unwrap().into()],
        ));
        f.region(0)
            .expect("function.def must have at least 1 region")
            .append_block(block);
    }

    assert_eq!(f.region_count(), 1);
    let f = module.body().append_operation(f.into());
    assert!(f.verify());
    log::info!("Op passed verification");
    format!("{f}")
}

#[test]
fn create_undef_felt() {
    let context = LlzkContext::new();
    let ir = create_undef(&context, FeltType::new(&context).into());
    let expected = r"function.def @f_undef_test() -> !felt.type attributes {function.allow_witness} {
  %0 = undef.undef : !felt.type
  function.return %0 : !felt.type
}";
    assert_eq!(ir, expected);
}

#[test]
fn create_undef_felt_array() {
    let context = LlzkContext::new();
    let ty = ArrayType::new_with_dims(FeltType::new(&context).into(), &[4, 8]);
    let ir = create_undef(&context, ty.into());
    let expected = r"function.def @f_undef_test() -> !array.type<4,8 x !felt.type> attributes {function.allow_witness} {
  %0 = undef.undef : !array.type<4,8 x !felt.type>
  function.return %0 : !array.type<4,8 x !felt.type>
}";
    assert_eq!(ir, expected);
}
