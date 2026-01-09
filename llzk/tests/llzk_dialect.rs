use llzk::dialect::llzk::*;
use llzk::prelude::*;

mod common;

#[test]
fn create_pub_attr() {
    common::setup();
    let context = LlzkContext::new();
    let a = PublicAttribute::new(&context);

    let ir = format!("{a}");
    let expected = "#llzk.pub";
    assert_eq!(ir, expected);
}

#[test]
fn create_loop_bounds_attr() {
    common::setup();
    let context = LlzkContext::new();
    let a = LoopBoundsAttribute::new(&context, 0, 10, 1);

    let ir = format!("{a}");
    let expected = "#llzk.loopbounds<0 to 10 step 1>";
    assert_eq!(ir, expected);
}
