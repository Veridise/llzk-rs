use llzk::prelude::*;
use melior::ir::{Location, Type, r#type::FunctionType};

mod common;

#[test]
fn f_constant() {
    common::setup();
    let context = LlzkContext::new();
    let module = llzk_module(Location::unknown(&context));
    let loc = Location::unknown(&context);
    let f = function::def(
        loc,
        "f_constant",
        FunctionType::new(&context, &[], &[FeltType::new(&context).into()]),
        &[],
        None,
    )
    .unwrap();
    {
        let block = Block::new(&[]);
        let felt = block
            .append_operation(felt::constant(loc, FeltConstAttribute::new(&context, 42)).unwrap());
        block.append_operation(function::r#return(loc, &[felt.result(0).unwrap().into()]));
        f.region(0)
            .expect("function.def must have at least 1 region")
            .append_block(block);
    }

    assert_eq!(f.region_count(), 1);
    let f = module.body().append_operation(f.into());
    assert!(f.verify());
    log::info!("Op passed verification");
    let ir = format!("{f}");
    let expected = r"function.def @f_constant() -> !felt.type {
  %felt_const_42 = felt.const  42
  function.return %felt_const_42 : !felt.type
}";
    assert_eq!(ir, expected);
}

#[test]
fn f_add() {
    common::setup();
    let context = LlzkContext::new();
    let module = llzk_module(Location::unknown(&context));
    let loc = Location::unknown(&context);
    let felt_type: Type = FeltType::new(&context).into();
    let f = function::def(
        loc,
        "f_add",
        FunctionType::new(&context, &[felt_type, felt_type], &[felt_type]),
        &[],
        None,
    )
    .unwrap();
    {
        let block = Block::new(&[(felt_type, loc), (felt_type, loc)]);
        let felt = block.append_operation(
            felt::add(
                loc,
                block.argument(0).unwrap().into(),
                block.argument(1).unwrap().into(),
            )
            .unwrap(),
        );
        block.append_operation(function::r#return(loc, &[felt.result(0).unwrap().into()]));
        f.region(0)
            .expect("function.def must have at least 1 region")
            .append_block(block);
    }

    assert_eq!(f.region_count(), 1);
    let f = module.body().append_operation(f.into());
    assert!(f.verify());
    log::info!("Op passed verification");
    let ir = format!("{f}");
    let expected = r"function.def @f_add(%arg0: !felt.type, %arg1: !felt.type) -> !felt.type {
  %0 = felt.add %arg0, %arg1 : !felt.type, !felt.type
  function.return %0 : !felt.type
}";
    assert_eq!(ir, expected);
}

#[test]
fn f_sub() {
    common::setup();
    let context = LlzkContext::new();
    let module = llzk_module(Location::unknown(&context));
    let loc = Location::unknown(&context);
    let felt_type: Type = FeltType::new(&context).into();
    let f = function::def(
        loc,
        "f_sub",
        FunctionType::new(&context, &[felt_type, felt_type], &[felt_type]),
        &[],
        None,
    )
    .unwrap();
    {
        let block = Block::new(&[(felt_type, loc), (felt_type, loc)]);
        let felt = block.append_operation(
            felt::sub(
                loc,
                block.argument(0).unwrap().into(),
                block.argument(1).unwrap().into(),
            )
            .unwrap(),
        );
        block.append_operation(function::r#return(loc, &[felt.result(0).unwrap().into()]));
        f.region(0)
            .expect("function.def must have at least 1 region")
            .append_block(block);
    }

    assert_eq!(f.region_count(), 1);
    let f = module.body().append_operation(f.into());
    assert!(f.verify());
    log::info!("Op passed verification");
    let ir = format!("{f}");
    let expected = r"function.def @f_sub(%arg0: !felt.type, %arg1: !felt.type) -> !felt.type {
  %0 = felt.sub %arg0, %arg1 : !felt.type, !felt.type
  function.return %0 : !felt.type
}";
    assert_eq!(ir, expected);
}

#[test]
fn f_mul() {
    common::setup();
    let context = LlzkContext::new();
    let module = llzk_module(Location::unknown(&context));
    let loc = Location::unknown(&context);
    let felt_type: Type = FeltType::new(&context).into();
    let f = function::def(
        loc,
        "f_mul",
        FunctionType::new(&context, &[felt_type, felt_type], &[felt_type]),
        &[],
        None,
    )
    .unwrap();
    {
        let block = Block::new(&[(felt_type, loc), (felt_type, loc)]);
        let felt = block.append_operation(
            felt::mul(
                loc,
                block.argument(0).unwrap().into(),
                block.argument(1).unwrap().into(),
            )
            .unwrap(),
        );
        block.append_operation(function::r#return(loc, &[felt.result(0).unwrap().into()]));
        f.region(0)
            .expect("function.def must have at least 1 region")
            .append_block(block);
    }

    assert_eq!(f.region_count(), 1);
    let f = module.body().append_operation(f.into());
    assert!(f.verify());
    log::info!("Op passed verification");
    let ir = format!("{f}");
    let expected = r"function.def @f_mul(%arg0: !felt.type, %arg1: !felt.type) -> !felt.type {
  %0 = felt.mul %arg0, %arg1 : !felt.type, !felt.type
  function.return %0 : !felt.type
}";
    assert_eq!(ir, expected);
}

#[test]
fn f_div() {
    common::setup();
    let context = LlzkContext::new();
    let module = llzk_module(Location::unknown(&context));
    let loc = Location::unknown(&context);
    let felt_type: Type = FeltType::new(&context).into();
    let f = function::def(
        loc,
        "f_div",
        FunctionType::new(&context, &[felt_type, felt_type], &[felt_type]),
        &[],
        None,
    )
    .unwrap();
    {
        let block = Block::new(&[(felt_type, loc), (felt_type, loc)]);
        let felt = block.append_operation(
            felt::div(
                loc,
                block.argument(0).unwrap().into(),
                block.argument(1).unwrap().into(),
            )
            .unwrap(),
        );
        block.append_operation(function::r#return(loc, &[felt.result(0).unwrap().into()]));
        f.region(0)
            .expect("function.def must have at least 1 region")
            .append_block(block);
    }

    assert_eq!(f.region_count(), 1);
    let f = module.body().append_operation(f.into());
    assert!(f.verify());
    log::info!("Op passed verification");
    let ir = format!("{f}");
    let expected = r"function.def @f_div(%arg0: !felt.type, %arg1: !felt.type) -> !felt.type {
  %0 = felt.div %arg0, %arg1 : !felt.type, !felt.type
  function.return %0 : !felt.type
}";
    assert_eq!(ir, expected);
}

#[test]
fn f_mod() {
    common::setup();
    let context = LlzkContext::new();
    let module = llzk_module(Location::unknown(&context));
    let loc = Location::unknown(&context);
    let felt_type: Type = FeltType::new(&context).into();
    let f = function::def(
        loc,
        "f_mod",
        FunctionType::new(&context, &[felt_type, felt_type], &[felt_type]),
        &[],
        None,
    )
    .unwrap();
    {
        let block = Block::new(&[(felt_type, loc), (felt_type, loc)]);
        let felt = block.append_operation(
            felt::r#mod(
                loc,
                block.argument(0).unwrap().into(),
                block.argument(1).unwrap().into(),
            )
            .unwrap(),
        );
        block.append_operation(function::r#return(loc, &[felt.result(0).unwrap().into()]));
        f.region(0)
            .expect("function.def must have at least 1 region")
            .append_block(block);
    }

    assert_eq!(f.region_count(), 1);
    let f = module.body().append_operation(f.into());
    assert!(f.verify());
    log::info!("Op passed verification");
    let ir = format!("{f}");
    let expected = r"function.def @f_mod(%arg0: !felt.type, %arg1: !felt.type) -> !felt.type {
  %0 = felt.mod %arg0, %arg1 : !felt.type, !felt.type
  function.return %0 : !felt.type
}";
    assert_eq!(ir, expected);
}

#[test]
fn f_neg() {
    common::setup();
    let context = LlzkContext::new();
    let module = llzk_module(Location::unknown(&context));
    let loc = Location::unknown(&context);
    let felt_type: Type = FeltType::new(&context).into();
    let f = function::def(
        loc,
        "f_neg",
        FunctionType::new(&context, &[felt_type], &[felt_type]),
        &[],
        None,
    )
    .unwrap();
    {
        let block = Block::new(&[(felt_type, loc)]);
        let felt =
            block.append_operation(felt::neg(loc, block.argument(0).unwrap().into()).unwrap());
        block.append_operation(function::r#return(loc, &[felt.result(0).unwrap().into()]));
        f.region(0)
            .expect("function.def must have at least 1 region")
            .append_block(block);
    }

    assert_eq!(f.region_count(), 1);
    let f = module.body().append_operation(f.into());
    assert!(f.verify());
    log::info!("Op passed verification");
    let ir = format!("{f}");
    let expected = r"function.def @f_neg(%arg0: !felt.type) -> !felt.type {
  %0 = felt.neg %arg0 : !felt.type
  function.return %0 : !felt.type
}";
    assert_eq!(ir, expected);
}

#[test]
fn f_inv() {
    common::setup();
    let context = LlzkContext::new();
    let module = llzk_module(Location::unknown(&context));
    let loc = Location::unknown(&context);
    let felt_type: Type = FeltType::new(&context).into();
    let f = function::def(
        loc,
        "f_inv",
        FunctionType::new(&context, &[felt_type], &[felt_type]),
        &[],
        None,
    )
    .unwrap();
    f.set_allow_witness_attr(true);
    {
        let block = Block::new(&[(felt_type, loc)]);
        let felt =
            block.append_operation(felt::inv(loc, block.argument(0).unwrap().into()).unwrap());
        block.append_operation(function::r#return(loc, &[felt.result(0).unwrap().into()]));
        f.region(0)
            .expect("function.def must have at least 1 region")
            .append_block(block);
    }

    assert_eq!(f.region_count(), 1);
    let f = module.body().append_operation(f.into());
    assert!(f.verify());
    log::info!("Op passed verification");
    let ir = format!("{f}");
    let expected = r"function.def @f_inv(%arg0: !felt.type) -> !felt.type attributes {function.allow_witness} {
  %0 = felt.inv %arg0 : !felt.type
  function.return %0 : !felt.type
}";
    assert_eq!(ir, expected);
}

#[test]
fn f_bit_not() {
    common::setup();
    let context = LlzkContext::new();
    let module = llzk_module(Location::unknown(&context));
    let loc = Location::unknown(&context);
    let felt_type: Type = FeltType::new(&context).into();
    let f = function::def(
        loc,
        "f_bit_not",
        FunctionType::new(&context, &[felt_type], &[felt_type]),
        &[],
        None,
    )
    .unwrap();
    f.set_allow_witness_attr(true);
    {
        let block = Block::new(&[(felt_type, loc)]);
        let felt =
            block.append_operation(felt::bit_not(loc, block.argument(0).unwrap().into()).unwrap());
        block.append_operation(function::r#return(loc, &[felt.result(0).unwrap().into()]));
        f.region(0)
            .expect("function.def must have at least 1 region")
            .append_block(block);
    }

    assert_eq!(f.region_count(), 1);
    let f = module.body().append_operation(f.into());
    assert!(f.verify());
    log::info!("Op passed verification");
    let ir = format!("{f}");
    let expected = r"function.def @f_bit_not(%arg0: !felt.type) -> !felt.type attributes {function.allow_witness} {
  %0 = felt.bit_not %arg0 : !felt.type
  function.return %0 : !felt.type
}";
    assert_eq!(ir, expected);
}

#[test]
fn f_shl() {
    common::setup();
    let context = LlzkContext::new();
    let module = llzk_module(Location::unknown(&context));
    let loc = Location::unknown(&context);
    let felt_type: Type = FeltType::new(&context).into();
    let f = function::def(
        loc,
        "f_shl",
        FunctionType::new(&context, &[felt_type, felt_type], &[felt_type]),
        &[],
        None,
    )
    .unwrap();
    f.set_allow_witness_attr(true);
    {
        let block = Block::new(&[(felt_type, loc), (felt_type, loc)]);
        let felt = block.append_operation(
            felt::shl(
                loc,
                block.argument(0).unwrap().into(),
                block.argument(1).unwrap().into(),
            )
            .unwrap(),
        );
        block.append_operation(function::r#return(loc, &[felt.result(0).unwrap().into()]));
        f.region(0)
            .expect("function.def must have at least 1 region")
            .append_block(block);
    }

    assert_eq!(f.region_count(), 1);
    let f = module.body().append_operation(f.into());
    assert!(f.verify());
    log::info!("Op passed verification");
    let ir = format!("{f}");
    let expected = r"function.def @f_shl(%arg0: !felt.type, %arg1: !felt.type) -> !felt.type attributes {function.allow_witness} {
  %0 = felt.shl %arg0, %arg1 : !felt.type, !felt.type
  function.return %0 : !felt.type
}";
    assert_eq!(ir, expected);
}

#[test]
fn f_shr() {
    common::setup();
    let context = LlzkContext::new();
    let module = llzk_module(Location::unknown(&context));
    let loc = Location::unknown(&context);
    let felt_type: Type = FeltType::new(&context).into();
    let f = function::def(
        loc,
        "f_shr",
        FunctionType::new(&context, &[felt_type, felt_type], &[felt_type]),
        &[],
        None,
    )
    .unwrap();
    f.set_allow_witness_attr(true);
    {
        let block = Block::new(&[(felt_type, loc), (felt_type, loc)]);
        let felt = block.append_operation(
            felt::shr(
                loc,
                block.argument(0).unwrap().into(),
                block.argument(1).unwrap().into(),
            )
            .unwrap(),
        );
        block.append_operation(function::r#return(loc, &[felt.result(0).unwrap().into()]));
        f.region(0)
            .expect("function.def must have at least 1 region")
            .append_block(block);
    }

    assert_eq!(f.region_count(), 1);
    let f = module.body().append_operation(f.into());
    assert!(f.verify());
    log::info!("Op passed verification");
    let ir = format!("{f}");
    let expected = r"function.def @f_shr(%arg0: !felt.type, %arg1: !felt.type) -> !felt.type attributes {function.allow_witness} {
  %0 = felt.shr %arg0, %arg1 : !felt.type, !felt.type
  function.return %0 : !felt.type
}";
    assert_eq!(ir, expected);
}

#[test]
fn f_bit_and() {
    common::setup();
    let context = LlzkContext::new();
    let module = llzk_module(Location::unknown(&context));
    let loc = Location::unknown(&context);
    let felt_type: Type = FeltType::new(&context).into();
    let f = function::def(
        loc,
        "f_bit_and",
        FunctionType::new(&context, &[felt_type, felt_type], &[felt_type]),
        &[],
        None,
    )
    .unwrap();
    f.set_allow_witness_attr(true);
    {
        let block = Block::new(&[(felt_type, loc), (felt_type, loc)]);
        let felt = block.append_operation(
            felt::bit_and(
                loc,
                block.argument(0).unwrap().into(),
                block.argument(1).unwrap().into(),
            )
            .unwrap(),
        );
        block.append_operation(function::r#return(loc, &[felt.result(0).unwrap().into()]));
        f.region(0)
            .expect("function.def must have at least 1 region")
            .append_block(block);
    }

    assert_eq!(f.region_count(), 1);
    let f = module.body().append_operation(f.into());
    assert!(f.verify());
    log::info!("Op passed verification");
    let ir = format!("{f}");
    let expected = r"function.def @f_bit_and(%arg0: !felt.type, %arg1: !felt.type) -> !felt.type attributes {function.allow_witness} {
  %0 = felt.bit_and %arg0, %arg1 : !felt.type, !felt.type
  function.return %0 : !felt.type
}";
    assert_eq!(ir, expected);
}

#[test]
fn f_bit_or() {
    common::setup();
    let context = LlzkContext::new();
    let module = llzk_module(Location::unknown(&context));
    let loc = Location::unknown(&context);
    let felt_type: Type = FeltType::new(&context).into();
    let f = function::def(
        loc,
        "f_bit_or",
        FunctionType::new(&context, &[felt_type, felt_type], &[felt_type]),
        &[],
        None,
    )
    .unwrap();
    f.set_allow_witness_attr(true);
    {
        let block = Block::new(&[(felt_type, loc), (felt_type, loc)]);
        let felt = block.append_operation(
            felt::bit_or(
                loc,
                block.argument(0).unwrap().into(),
                block.argument(1).unwrap().into(),
            )
            .unwrap(),
        );
        block.append_operation(function::r#return(loc, &[felt.result(0).unwrap().into()]));
        f.region(0)
            .expect("function.def must have at least 1 region")
            .append_block(block);
    }

    assert_eq!(f.region_count(), 1);
    let f = module.body().append_operation(f.into());
    assert!(f.verify());
    log::info!("Op passed verification");
    let ir = format!("{f}");
    let expected = r"function.def @f_bit_or(%arg0: !felt.type, %arg1: !felt.type) -> !felt.type attributes {function.allow_witness} {
  %0 = felt.bit_or %arg0, %arg1 : !felt.type, !felt.type
  function.return %0 : !felt.type
}";
    assert_eq!(ir, expected);
}

#[test]
fn f_bit_xor() {
    common::setup();
    let context = LlzkContext::new();
    let module = llzk_module(Location::unknown(&context));
    let loc = Location::unknown(&context);
    let felt_type: Type = FeltType::new(&context).into();
    let f = function::def(
        loc,
        "f_bit_xor",
        FunctionType::new(&context, &[felt_type, felt_type], &[felt_type]),
        &[],
        None,
    )
    .unwrap();
    f.set_allow_witness_attr(true);
    {
        let block = Block::new(&[(felt_type, loc), (felt_type, loc)]);
        let felt = block.append_operation(
            felt::bit_xor(
                loc,
                block.argument(0).unwrap().into(),
                block.argument(1).unwrap().into(),
            )
            .unwrap(),
        );
        block.append_operation(function::r#return(loc, &[felt.result(0).unwrap().into()]));
        f.region(0)
            .expect("function.def must have at least 1 region")
            .append_block(block);
    }

    assert_eq!(f.region_count(), 1);
    let f = module.body().append_operation(f.into());
    assert!(f.verify());
    log::info!("Op passed verification");
    let ir = format!("{f}");
    let expected = r"function.def @f_bit_xor(%arg0: !felt.type, %arg1: !felt.type) -> !felt.type attributes {function.allow_witness} {
  %0 = felt.bit_xor %arg0, %arg1 : !felt.type, !felt.type
  function.return %0 : !felt.type
}";
    assert_eq!(ir, expected);
}
