use llzk::builder::OpBuilder;
use llzk::prelude::*;
use melior::ir::{Location, r#type::FunctionType};

mod common;

#[test]
fn empty_function() {
    common::setup();
    let context = LlzkContext::new();
    let module = llzk_module(Location::unknown(&context));
    let loc = Location::unknown(&context);
    let f = function::def(
        loc,
        "empty",
        FunctionType::new(&context, &[], &[]),
        &[],
        None,
    )
    .unwrap();
    {
        let block = Block::new(&[]);
        block.append_operation(function::r#return(loc, &[]));
        f.region(0)
            .expect("function.def must have at least 1 region")
            .append_block(block);
    }

    assert_eq!(f.region_count(), 1);
    let f = module.body().append_operation(f.into());
    assert!(f.verify());
    log::info!("Op passed verification");
    let ir = format!("{f}");
    let expected = r"function.def @empty() {
  function.return
}";
    assert_eq!(ir, expected);
}

#[test]
fn function_call() {
    common::setup();
    let context = LlzkContext::new();
    let module = llzk_module(Location::unknown(&context));
    let loc = Location::unknown(&context);
    let felt_type: Type = FeltType::new(&context).into();
    let f = function::def(
        loc,
        "recursive",
        FunctionType::new(&context, &[], &[felt_type]),
        &[],
        None,
    )
    .unwrap();
    {
        let block = Block::new(&[]);
        let builder =
            OpBuilder::at_block_begin(&context, unsafe { BlockRef::from_raw(block.to_raw()) });
        // Build call to itself
        let v = block
            .append_operation(
                function::call(&builder, loc, "recursive", &[], &[felt_type])
                    .unwrap()
                    .into(),
            )
            .result(0)
            .map(Value::from)
            .unwrap();
        // Build return operation
        block.append_operation(function::r#return(loc, &[v]));
        // Add Block to function
        f.region(0)
            .expect("function.def must have at least 1 region")
            .append_block(block);
    }

    assert_eq!(f.region_count(), 1);
    let f = module.body().append_operation(f.into());
    assert!(f.verify());
    log::info!("Op passed verification");
    let ir = format!("{f}");
    let expected = r"function.def @recursive() -> !felt.type {
  %0 = function.call @recursive() : () -> !felt.type 
  function.return %0 : !felt.type
}";
    assert_eq!(ir, expected);
}
