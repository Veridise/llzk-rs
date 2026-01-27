use llzk::builder::OpBuilder;
use llzk::dialect::pod::ops::RecordValue;
use llzk::prelude::melior_dialects::arith;
use llzk::prelude::*;

mod common;

#[test]
fn create_record_attr() {
    common::setup();
    let context = LlzkContext::new();
    let a = PodRecordAttribute::new("a", FeltType::new(&context).into());

    let ir = format!("{a}");
    let expected = "#pod<record@a: !felt.type>";
    assert_eq!(ir, expected);
}

#[test]
fn record_attr_name() {
    common::setup();
    let context = LlzkContext::new();
    let a = PodRecordAttribute::new("name", FeltType::new(&context).into());
    let name = a.name();

    let name = name.as_str();
    assert!(name.is_ok());
    let name = name.unwrap();
    assert_eq!(name, "name");
}

#[test]
fn record_attr_type() {
    common::setup();
    let context = LlzkContext::new();
    let a = PodRecordAttribute::new("a", FeltType::new(&context).into());
    let ty = a.r#type();

    let ir = format!("{ty}");
    let expected = "!felt.type";
    assert_eq!(ir, expected);
}

#[test]
fn create_pod_type() {
    common::setup();
    let context = LlzkContext::new();
    let records = vec![
        PodRecordAttribute::new("a", FeltType::new(&context).into()),
        PodRecordAttribute::new(
            "b",
            ArrayType::new(
                FeltType::new(&context).into(),
                &[FlatSymbolRefAttribute::new(&context, "N").into()],
            )
            .into(),
        ),
        PodRecordAttribute::new(
            "c",
            StructType::new(FlatSymbolRefAttribute::new(&context, "S"), &[]).into(),
        ),
    ];
    let ty = PodType::new(&context, &records);

    let ir = format!("{ty}");
    let expected =
        "!pod.type<[@a: !felt.type, @b: !array.type<@N x !felt.type>, @c: !struct.type<@S<[]>>]>";
    assert_eq!(ir, expected);
}

#[test]
fn get_records() {
    common::setup();
    let context = LlzkContext::new();
    let records = vec![
        PodRecordAttribute::new("a", PodType::new(&context, &[]).into()),
        PodRecordAttribute::new(
            "b",
            ArrayType::new(
                FeltType::new(&context).into(),
                &[FlatSymbolRefAttribute::new(&context, "N").into()],
            )
            .into(),
        ),
        PodRecordAttribute::new(
            "c",
            StructType::new(FlatSymbolRefAttribute::new(&context, "S"), &[]).into(),
        ),
    ];
    let ty = PodType::new(&context, &records);
    let r = ty.get_records();
    assert_eq!(r.len(), records.len());

    assert_eq!(format!("{}", r[0]), "#pod<record@a: !pod.type<[]>>");
    assert_eq!(
        format!("{}", r[1]),
        "#pod<record@b: !array.type<@N x !felt.type>>"
    );
    assert_eq!(format!("{}", r[2]), "#pod<record@c: !struct.type<@S<[]>>>");
}

#[test]
fn get_type_of_record() {
    common::setup();
    let context = LlzkContext::new();
    let records = vec![
        PodRecordAttribute::new("a", PodType::new(&context, &[]).into()),
        PodRecordAttribute::new(
            "b",
            ArrayType::new(
                FeltType::new(&context).into(),
                &[FlatSymbolRefAttribute::new(&context, "N").into()],
            )
            .into(),
        ),
        PodRecordAttribute::new(
            "c",
            StructType::new(FlatSymbolRefAttribute::new(&context, "S"), &[]).into(),
        ),
    ];
    let ty = PodType::new(&context, &records);
    let r = ty.get_type_of_record("b");
    assert!(r.is_some());
    assert_eq!(format!("{}", r.unwrap()), "!array.type<@N x !felt.type>");
}

#[test]
fn pod_new_empty_and_inferred() {
    common::setup();
    let context = LlzkContext::new();
    let location = Location::unknown(&context);
    let builder = OpBuilder::new(&context);
    let op = pod::new(&builder, location, &[], None);

    let ir = format!("{op}");
    let expected = "%pod = pod.new : <[]>\n";
    assert_eq!(ir, expected);
}

#[test]
fn pod_new_nonempty_and_inferred() {
    common::setup();
    let context = LlzkContext::new();
    let location = Location::unknown(&context);
    let builder = OpBuilder::new(&context);

    // Note: must keep hard ref to this op to prevent it being dropped.
    let arith_op = arith::constant(
        &context,
        IntegerAttribute::new(Type::index(&context), 42).into(),
        location,
    );
    let values = vec![RecordValue::new(
        StringRef::new("field1"),
        arith_op.result(0).unwrap().into(),
    )];
    let op = pod::new(&builder, location, &values, None);

    let ir = format!("{op}");
    let expected = "%pod = pod.new { @field1 = <<UNKNOWN SSA VALUE>> }  : <[@field1: index]>\n";
    assert_eq!(ir, expected);
}
