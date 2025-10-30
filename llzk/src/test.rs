use melior::{
    Context,
    dialect::DialectRegistry,
    utility::{register_all_dialects, register_all_llvm_translations},
};
use rstest::fixture;

use crate::register_all_llzk_dialects;

pub fn load_all_dialects(context: &Context) {
    let registry = DialectRegistry::new();
    register_all_dialects(&registry);
    register_all_llzk_dialects(&registry);
    context.append_dialect_registry(&registry);
    context.load_all_available_dialects();
}

#[fixture]
pub fn ctx() -> Context {
    let context = Context::new();

    context.attach_diagnostic_handler(|diagnostic| {
        eprintln!("{}", diagnostic);
        true
    });

    load_all_dialects(&context);
    register_all_llvm_translations(&context);

    context
}
