use llzk::builder::{OpBuilder, OpBuilderLike};
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
        let name = FlatSymbolRefAttribute::new(&context, "recursive");
        let v = block
            .append_operation(
                function::call(&builder, loc, name, &[], &[felt_type])
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

fn make_empty_struct<'c>(context: &'c LlzkContext, name: &str) -> StructDefOp<'c> {
    let loc = Location::unknown(&context);
    let typ = StructType::from_str(&context, name);
    r#struct::def(loc, name, &[], {
        [
            r#struct::helpers::compute_fn(loc, typ, &[], None).map(Into::into),
            r#struct::helpers::constrain_fn(loc, typ, &[], None).map(Into::into),
        ]
    })
    .unwrap()
}

#[test]
fn func_def_op_self_value_of_compute() {
    common::setup();
    let context = LlzkContext::new();
    let module = llzk_module(Location::unknown(&context));
    let module_body = module.body();

    let s = make_empty_struct(&context, "StructA");
    let s = StructDefOpRef::try_from(module_body.append_operation(s.into())).unwrap();
    llzk::operation::verify_operation_with_diags(&s).expect("verification failed");
    log::info!("Struct passed verification");

    let compute_fn = s
        .get_compute_func()
        .expect("failed to get compute function");
    let self_val = compute_fn.self_value_of_compute().unwrap();
    // Get the expected value. The first operation in the compute function is
    // the CreateStructOp, whose first result is the self value.
    let expected = compute_fn
        .region(0)
        .expect("failed to get first region")
        .first_block()
        .expect("failed to get first block")
        .first_operation()
        .expect("failed to get first operation")
        .result(0)
        .expect("failed to get first result");

    similar_asserts::assert_eq!(self_val, expected.into());
}

#[test]
fn func_def_op_self_value_of_constrain() {
    common::setup();
    let context = LlzkContext::new();
    let module = llzk_module(Location::unknown(&context));
    let module_body = module.body();

    let s = make_empty_struct(&context, "StructA");
    let s = StructDefOpRef::try_from(module_body.append_operation(s.into())).unwrap();
    llzk::operation::verify_operation_with_diags(&s).expect("verification failed");
    log::info!("Struct passed verification");

    let constrain_fn = s
        .get_constrain_func()
        .expect("failed to get constrain function");
    let self_val = constrain_fn.self_value_of_constrain().unwrap();
    // Get the expected value. The first argument of the function is the self value.
    let expected = constrain_fn
        .argument(0)
        .expect("failed to get first argument");

    similar_asserts::assert_eq!(self_val, expected.into());
}

#[test]
fn call_op_self_value_of_compute() {
    common::setup();
    let context = LlzkContext::new();
    let module = llzk_module(Location::unknown(&context));
    let module_body = module.body();

    let s1 = make_empty_struct(&context, "StructA");
    let s1 = StructDefOpRef::try_from(module_body.append_operation(s1.into())).unwrap();
    assert!(s1.verify());
    log::info!("Struct 1 passed verification");

    let s2 = make_empty_struct(&context, "StructB");
    let s2 = StructDefOpRef::try_from(module_body.append_operation(s2.into())).unwrap();
    assert!(s2.verify());
    log::info!("Struct 2 passed verification");

    let s2_compute_body = s2
        .get_compute_func()
        .expect("failed to get compute function")
        .region(0)
        .expect("failed to get first region")
        .first_block()
        .expect("failed to get first block");
    let builder = OpBuilder::at_block_begin(&context, s2_compute_body);
    let loc = Location::unknown(&context);
    let call = builder.insert(loc, |_, loc| {
        let name = SymbolRefAttribute::new(&context, "StructA", &["compute"]);
        function::call(&builder, loc, name, &[], &[s1.r#type()])
            .unwrap()
            .into()
    });

    assert_test!(module.as_operation(), module, @file "expected/call_op_self_value_of_compute.mlir" );

    // Now actually test the `self_value_of_compute` function
    let call = CallOpRef::try_from(call).unwrap();
    let self_val = call.self_value_of_compute();
    similar_asserts::assert_eq!(
        format!("{}", self_val.unwrap()),
        // Yes, the line does have a trailing space, here and in the entire IR above.
        "%0 = function.call @StructA::@compute() : () -> !struct.type<@StructA<[]>> "
    );
}
