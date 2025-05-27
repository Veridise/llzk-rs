use rstest::rstest;

use crate::llzkRegisterAllDialects;

use super::{registry, TestRegistry};

#[rstest]
fn test_llzk_register_all_dialects(registry: TestRegistry) {
    unsafe { llzkRegisterAllDialects(registry.registry) }
}
