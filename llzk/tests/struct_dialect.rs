use llzk::prelude::*;
use melior::ir::Location;

#[macro_use]
mod common;

fn default_funcs<'c>(
    loc: Location<'c>,
    typ: StructType<'c>,
) -> [Result<Operation<'c>, LlzkError>; 2] {
    [
        r#struct::helpers::compute_fn(loc, typ, &[], None).map(Into::into),
        r#struct::helpers::constrain_fn(loc, typ, &[], None).map(Into::into),
    ]
}

#[test]
fn empty_struct() {
    common::setup();
    let context = LlzkContext::new();
    let module = llzk_module(Location::unknown(&context));
    let loc = Location::unknown(&context);
    let typ = StructType::from_str(&context, "empty");

    let s = r#struct::def(loc, "empty", &[], default_funcs(loc, typ)).unwrap();
    let s = module.body().append_operation(s.into());

    assert_test!(s, module, @file "expected/empty_struct.mlir" );
}

#[test]
fn empty_struct_with_one_param() {
    common::setup();
    let context = LlzkContext::new();
    let module = llzk_module(Location::unknown(&context));
    let loc = Location::unknown(&context);
    let typ = StructType::from_str_params(&context, "empty", &["T"]);

    let s = r#struct::def(loc, "empty", &["T"], default_funcs(loc, typ)).unwrap();
    let s = module.body().append_operation(s.into());

    assert_test!(s, module, @file "expected/empty_struct_with_one_param.mlir");
}

#[test]
fn empty_struct_with_pub_inputs() {
    common::setup();
    let context = LlzkContext::new();
    let module = llzk_module(Location::unknown(&context));
    let loc = Location::unknown(&context);
    let typ = StructType::from_str_params(&context, "empty", &[]);

    let inputs = vec![(FeltType::new(&context).into(), Location::unknown(&context))];
    let arg_attrs = vec![vec![PublicAttribute::new_named_attr(&context)]];
    let s = r#struct::def(loc, "empty", &[], {
        [
            r#struct::helpers::compute_fn(loc, typ, inputs.as_slice(), Some(arg_attrs.as_slice()))
                .map(Into::into),
            r#struct::helpers::constrain_fn(
                loc,
                typ,
                inputs.as_slice(),
                Some(arg_attrs.as_slice()),
            )
            .map(Into::into),
        ]
    })
    .unwrap();
    let s = module.body().append_operation(s.into());

    assert_test!(s, module, @file "expected/empty_struct_with_pub_inputs.mlir");
}

#[test]
fn signal_struct() {
    common::setup();
    let context = LlzkContext::new();
    let module = llzk_module(Location::unknown(&context));

    let s = r#struct::helpers::define_signal_struct(&context).unwrap();
    let s = module.body().append_operation(s.into());

    assert_test!(s, module, @file "expected/signal_struct.mlir");
}
