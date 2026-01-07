use llzk::dialect::poly;
use llzk::prelude::*;
use melior::ir::Location;

mod common;

#[test]
fn create_read_const() {
    common::setup();
    let context = LlzkContext::new();
    let loc = Location::unknown(&context);
    let op = poly::read_const(loc, "A", FeltType::new(&context).into());

    let ir = format!("{op}");
    let expected = "%0 = poly.read_const @A : !felt.type\n";
    assert_eq!(ir, expected);
}

#[test]
fn is_read_const() {
    common::setup();
    let context = LlzkContext::new();
    let loc = Location::unknown(&context);
    let op = poly::read_const(loc, "C", IntegerType::new(&context, 64).into());

    let op_ref = unsafe { OperationRef::from_raw(op.to_raw()) };
    assert!(poly::is_read_const_op(op_ref));
}
