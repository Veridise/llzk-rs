use llzk::builder::OpBuilder;
use llzk::dialect::poly::{self, applymap, is_applymap_op, is_unifiable_cast_op, unifiable_cast};
use llzk::prelude::*;
use melior::dialect::arith;
use melior::ir::Location;
use rstest::rstest;

mod common;

#[test]
fn get_type() {
    common::setup();
    let context = LlzkContext::new();
    let t = poly::r#type::TVarType::new(&context, StringRef::new("A"));

    let ir = format!("{t}");
    let expected = "!poly.tvar<@A>";
    assert_eq!(ir, expected);
}

#[test]
fn get_type_name_ref() {
    common::setup();
    let context = LlzkContext::new();
    let t = poly::r#type::TVarType::new(&context, StringRef::new("A"));

    let ir = format!("{:?}", t.name().as_str().unwrap());
    let expected = "\"A\"";
    assert_eq!(ir, expected);
}

#[test]
fn create_read_const() {
    common::setup();
    let context = LlzkContext::new();
    let loc = Location::unknown(&context);
    let op = poly::read_const(loc, "A", FeltType::new(&context).into());

    let ir = format!("{op}");
    let expected = "%0 = poly.read_const @A : !felt.type\n";
    assert_eq!(ir, expected);
    assert!(op.verify());
}

#[test]
fn is_read_const() {
    common::setup();
    let context = LlzkContext::new();
    let loc = Location::unknown(&context);
    let op = poly::read_const(loc, "C", IntegerType::new(&context, 64).into());

    let op_ref = unsafe { OperationRef::from_raw(op.to_raw()) };
    assert!(poly::is_read_const_op(&op_ref));
}

fn create_index_constant<'c>(
    ctx: &'c Context,
    block: &Block<'c>,
    location: Location<'c>,
    i: i64,
) -> Value<'c, 'c> {
    let int_attr = IntegerAttribute::new(Type::index(ctx), i);
    let op = arith::constant(ctx, int_attr.into(), location);
    let op_ref = block.append_operation(op);
    assert_eq!(1, op_ref.result_count());
    op_ref.result(0).unwrap().into()
}

#[rstest]
#[case("affine_map<()[] -> (2)>", &[],
r"^bb0:
  %0 = poly.applymap () affine_map<() -> (2)>
")]
#[case("affine_map<(i)[] -> (i)>", &[1],
r"^bb0:
  %c1 = arith.constant 1 : index
  %0 = poly.applymap (%c1) affine_map<(d0) -> (d0)>
")]
#[case("affine_map<()[s0, s1] -> (s0 + s1)>", &[7, 9],
r"^bb0:
  %c7 = arith.constant 7 : index
  %c9 = arith.constant 9 : index
  %0 = poly.applymap ()[%c7, %c9] affine_map<()[s0, s1] -> (s0 + s1)>
")]
#[case("affine_map<(i, j) -> (i + j)>", &[2, 4],
r"^bb0:
  %c2 = arith.constant 2 : index
  %c4 = arith.constant 4 : index
  %0 = poly.applymap (%c2, %c4) affine_map<(d0, d1) -> (d0 + d1)>
")]
fn create_applymap(#[case] affine_map: &str, #[case] ops: &[i64], #[case] expected: &str) {
    common::setup();
    let context = LlzkContext::new();
    let location = Location::unknown(&context);

    let affine_map =
        Attribute::parse(&context, affine_map).expect("could not parse affine_map attribute");
    let module = Module::new(location);
    let block = module.body();
    let operands = ops
        .iter()
        .map(|i| create_index_constant(&context, &block, location, *i))
        .collect::<Vec<_>>();

    let applymap_op = applymap(location, affine_map, &operands);
    assert!(applymap_op.verify(), "op {applymap_op} failed to verify");
    assert!(is_applymap_op(&applymap_op));
    block.append_operation(applymap_op);
    let ir = format!("{block}");
    assert_eq!(ir, expected);
}

#[test]
fn create_unifiable_cast() {
    common::setup();
    let context = LlzkContext::new();
    let location = Location::unknown(&context);
    let module = Module::new(location);
    let block = module.body();

    let affine_map_str = "affine_map<()[s0, s1] -> (s0 + s1)>";
    let affine_map =
        Attribute::parse(&context, affine_map_str).expect("could not parse affine_map attribute");
    let array_ty = ArrayType::new(
        FeltType::new(&context).into(),
        &[FlatSymbolRefAttribute::new(&context, "N").into()],
    );
    let array_op = array::new(
        &OpBuilder::new(&context),
        location,
        array_ty,
        llzk::dialect::array::ArrayCtor::Values(&[]),
    );
    let array_op = block.append_operation(array_op);

    let new_array_ty = ArrayType::new(FeltType::new(&context).into(), &[affine_map]);
    let cast = unifiable_cast(
        location,
        array_op.result(0).unwrap().into(),
        new_array_ty.into(),
    );
    let cast = block.append_operation(cast);
    assert!(cast.verify(), "op {cast} failed to verify");
    assert!(is_unifiable_cast_op(&cast));

    let expected = r#"^bb0:
  %0 = "array.new"() <{mapOpGroupSizes = array<i32>, numDimsPerMap = array<i32>, operandSegmentSizes = array<i32: 0, 0>}> : () -> !array.type<@N x !felt.type>
  %1 = "poly.unifiable_cast"(%0) : (!array.type<@N x !felt.type>) -> !array.type<affine_map<()[s0, s1] -> (s0 + s1)> x !felt.type>
"#;
    let ir = format!("{block}");
    assert_eq!(ir, expected);
}
