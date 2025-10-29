use llzk::prelude::*;
use melior::ir::Location;

mod common;

#[test]
fn empty_struct() {
    common::setup();
    let context = LlzkContext::new();
    let module = llzk_module(Location::unknown(&context));
    let loc = Location::unknown(&context);
    let typ = StructType::from_str(&context, "empty");

    let s = r#struct::def(loc, "empty", &[], {
        [
            r#struct::helpers::compute_fn(loc, typ, &[], None).map(Into::into),
            r#struct::helpers::constrain_fn(loc, typ, &[], None).map(Into::into),
        ]
    })
    .unwrap();

    let s = module.body().append_operation(s.into());
    assert!(s.verify());
    log::info!("Op passed verification");
    let ir = format!("{s}");
    let expected = r"struct.def @empty<[]> {
  function.def @compute() -> !struct.type<@empty<[]>> attributes {function.allow_witness} {
    %self = struct.new : <@empty<[]>>
    function.return %self : !struct.type<@empty<[]>>
  }
  function.def @constrain(%arg0: !struct.type<@empty<[]>>) attributes {function.allow_constraint} {
    function.return
  }
}";
    similar_asserts::assert_eq!(ir, expected);
}

#[test]
fn empty_struct_with_one_param() {
    common::setup();
    let context = LlzkContext::new();
    let module = llzk_module(Location::unknown(&context));
    let loc = Location::unknown(&context);
    let typ = StructType::from_str_params(&context, "empty", &["T"]);

    let s = r#struct::def(loc, "empty", &["T"], {
        [
            r#struct::helpers::compute_fn(loc, typ, &[], None).map(Into::into),
            r#struct::helpers::constrain_fn(loc, typ, &[], None).map(Into::into),
        ]
    })
    .unwrap();

    let s = module.body().append_operation(s.into());
    assert!(s.verify());
    log::info!("Op passed verification");
    let ir = format!("{s}");
    let expected = r"struct.def @empty<[@T]> {
  function.def @compute() -> !struct.type<@empty<[@T]>> attributes {function.allow_witness} {
    %self = struct.new : <@empty<[@T]>>
    function.return %self : !struct.type<@empty<[@T]>>
  }
  function.def @constrain(%arg0: !struct.type<@empty<[@T]>>) attributes {function.allow_constraint} {
    function.return
  }
}";
    similar_asserts::assert_eq!(ir, expected);
}

#[test]
fn empty_struct_with_pub_inputs() {
    common::setup();
    let context = LlzkContext::new();
    let module = llzk_module(Location::unknown(&context));
    let loc = Location::unknown(&context);
    let typ = StructType::from_str_params(&context, "empty", &[]);

    let inputs = vec![(FeltType::new(&context).into(), Location::unknown(&context))];
    let arg_attrs = vec![vec![PublicAttribute::named_attr_pair(&context)]];
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
    assert!(s.verify());
    log::info!("Op passed verification");
    let ir = format!("{s}");
    let expected = r"struct.def @empty<[]> {
  function.def @compute(%arg0: !felt.type {llzk.pub}) -> !struct.type<@empty<[]>> attributes {function.allow_witness} {
    %self = struct.new : <@empty<[]>>
    function.return %self : !struct.type<@empty<[]>>
  }
  function.def @constrain(%arg0: !struct.type<@empty<[]>>, %arg1: !felt.type {llzk.pub}) attributes {function.allow_constraint} {
    function.return
  }
}";
    similar_asserts::assert_eq!(ir, expected);
}

#[test]
fn signal_struct() {
    common::setup();
    let context = LlzkContext::new();
    let module = llzk_module(Location::unknown(&context));

    let s = r#struct::helpers::define_signal_struct(&context).unwrap();

    let s = module.body().append_operation(s.into());
    assert!(s.verify());
    log::info!("Op passed verification");
    let ir = format!("{s}");
    let expected = r"struct.def @Signal<[]> {
  struct.field @reg : !felt.type {llzk.pub}
  function.def @compute(%arg0: !felt.type) -> !struct.type<@Signal<[]>> attributes {function.allow_witness} {
    %self = struct.new : <@Signal<[]>>
    struct.writef %self[@reg] = %arg0 : <@Signal<[]>>, !felt.type
    function.return %self : !struct.type<@Signal<[]>>
  }
  function.def @constrain(%arg0: !struct.type<@Signal<[]>>, %arg1: !felt.type) attributes {function.allow_constraint} {
    function.return
  }
}";
    similar_asserts::assert_eq!(ir, expected);
}
