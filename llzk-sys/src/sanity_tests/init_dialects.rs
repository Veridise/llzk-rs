use rstest::rstest;

use crate::llzkRegisterAllDialects;

use super::{TestRegistry, registry};

#[rstest]
fn test_llzk_register_all_dialects(registry: TestRegistry) {
    unsafe { llzkRegisterAllDialects(registry.registry) }
}
