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
