use llzk::prelude::*;
use melior::{
    dialect::arith,
    ir::{
        Location, Type,
        attribute::BoolAttribute,
        r#type::{FunctionType, IntegerType},
    },
};

mod common;

#[test]
fn f_eq() {
    common::setup();
    let context = LlzkContext::new();
    let module = llzk_module(Location::unknown(&context));
    let loc = Location::unknown(&context);
    let felt_type: Type = FeltType::new(&context).into();
    let bool_type: Type = IntegerType::new(&context, 1).into();
    let f = function::def(
        loc,
        "f_eq",
        FunctionType::new(&context, &[felt_type, felt_type], &[bool_type]),
        &[],
        None,
    )
    .unwrap();
    f.set_allow_witness_attr(true);
    {
        let block = Block::new(&[(felt_type, loc), (felt_type, loc)]);
        let felt = block.append_operation(
            bool::eq(
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
    let expected = r"function.def @f_eq(%arg0: !felt.type, %arg1: !felt.type) -> i1 attributes {function.allow_witness} {
  %0 = bool.cmp eq(%arg0, %arg1)
  function.return %0 : i1
}";
    assert_eq!(ir, expected);
}

#[test]
fn f_ne() {
    common::setup();
    let context = LlzkContext::new();
    let module = llzk_module(Location::unknown(&context));
    let loc = Location::unknown(&context);
    let felt_type: Type = FeltType::new(&context).into();
    let bool_type: Type = IntegerType::new(&context, 1).into();
    let f = function::def(
        loc,
        "f_ne",
        FunctionType::new(&context, &[felt_type, felt_type], &[bool_type]),
        &[],
        None,
    )
    .unwrap();
    f.set_allow_witness_attr(true);
    {
        let block = Block::new(&[(felt_type, loc), (felt_type, loc)]);
        let felt = block.append_operation(
            bool::ne(
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
    let expected = r"function.def @f_ne(%arg0: !felt.type, %arg1: !felt.type) -> i1 attributes {function.allow_witness} {
  %0 = bool.cmp ne(%arg0, %arg1)
  function.return %0 : i1
}";
    assert_eq!(ir, expected);
}

#[test]
fn f_lt() {
    common::setup();
    let context = LlzkContext::new();
    let module = llzk_module(Location::unknown(&context));
    let loc = Location::unknown(&context);
    let felt_type: Type = FeltType::new(&context).into();
    let bool_type: Type = IntegerType::new(&context, 1).into();
    let f = function::def(
        loc,
        "f_lt",
        FunctionType::new(&context, &[felt_type, felt_type], &[bool_type]),
        &[],
        None,
    )
    .unwrap();
    f.set_allow_witness_attr(true);
    {
        let block = Block::new(&[(felt_type, loc), (felt_type, loc)]);
        let felt = block.append_operation(
            bool::lt(
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
    let expected = r"function.def @f_lt(%arg0: !felt.type, %arg1: !felt.type) -> i1 attributes {function.allow_witness} {
  %0 = bool.cmp lt(%arg0, %arg1)
  function.return %0 : i1
}";
    assert_eq!(ir, expected);
}

#[test]
fn f_le() {
    common::setup();
    let context = LlzkContext::new();
    let module = llzk_module(Location::unknown(&context));
    let loc = Location::unknown(&context);
    let felt_type: Type = FeltType::new(&context).into();
    let bool_type: Type = IntegerType::new(&context, 1).into();
    let f = function::def(
        loc,
        "f_le",
        FunctionType::new(&context, &[felt_type, felt_type], &[bool_type]),
        &[],
        None,
    )
    .unwrap();
    f.set_allow_witness_attr(true);
    {
        let block = Block::new(&[(felt_type, loc), (felt_type, loc)]);
        let felt = block.append_operation(
            bool::le(
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
    let expected = r"function.def @f_le(%arg0: !felt.type, %arg1: !felt.type) -> i1 attributes {function.allow_witness} {
  %0 = bool.cmp le(%arg0, %arg1)
  function.return %0 : i1
}";
    assert_eq!(ir, expected);
}

#[test]
fn f_gt() {
    common::setup();
    let context = LlzkContext::new();
    let module = llzk_module(Location::unknown(&context));
    let loc = Location::unknown(&context);
    let felt_type: Type = FeltType::new(&context).into();
    let bool_type: Type = IntegerType::new(&context, 1).into();
    let f = function::def(
        loc,
        "f_gt",
        FunctionType::new(&context, &[felt_type, felt_type], &[bool_type]),
        &[],
        None,
    )
    .unwrap();
    f.set_allow_witness_attr(true);
    {
        let block = Block::new(&[(felt_type, loc), (felt_type, loc)]);
        let felt = block.append_operation(
            bool::gt(
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
    let expected = r"function.def @f_gt(%arg0: !felt.type, %arg1: !felt.type) -> i1 attributes {function.allow_witness} {
  %0 = bool.cmp gt(%arg0, %arg1)
  function.return %0 : i1
}";
    assert_eq!(ir, expected);
}

#[test]
fn f_ge() {
    common::setup();
    let context = LlzkContext::new();
    let module = llzk_module(Location::unknown(&context));
    let loc = Location::unknown(&context);
    let felt_type: Type = FeltType::new(&context).into();
    let bool_type: Type = IntegerType::new(&context, 1).into();
    let f = function::def(
        loc,
        "f_ge",
        FunctionType::new(&context, &[felt_type, felt_type], &[bool_type]),
        &[],
        None,
    )
    .unwrap();
    f.set_allow_witness_attr(true);
    {
        let block = Block::new(&[(felt_type, loc), (felt_type, loc)]);
        let felt = block.append_operation(
            bool::ge(
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
    let expected = r"function.def @f_ge(%arg0: !felt.type, %arg1: !felt.type) -> i1 attributes {function.allow_witness} {
  %0 = bool.cmp ge(%arg0, %arg1)
  function.return %0 : i1
}";
    assert_eq!(ir, expected);
}

#[test]
fn f_assert() {
    common::setup();
    let context = LlzkContext::new();
    let loc = Location::unknown(&context);

    let cond = arith::constant(&context, BoolAttribute::new(&context, false).into(), loc);
    let cond = cond.result(0).unwrap().into();
    let op = llzk::dialect::bool::assert(loc, cond, Some("assertion failed"))
        .expect("failed to build assert op");

    assert!(op.verify());
    log::info!("Op passed verification");
}
