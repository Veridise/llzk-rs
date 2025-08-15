use std::{borrow::Borrow, ops::Deref};

use llzk_sys::llzkRegisterAllDialects;
use melior::{dialect::DialectRegistry, Context};

pub mod builder;
pub mod dialect;
pub mod error;
mod macros;
pub mod symbol_ref;
#[cfg(test)]
mod test;
pub mod utils;
pub mod value_range;

/// A batteries-included MLIR context that automatically loads all the LLZK dialects.
pub struct LlzkContext {
    ctx: Context,
    _registry: DialectRegistry,
}

impl LlzkContext {
    pub fn new() -> Self {
        let ctx = Context::new();
        let registry = DialectRegistry::new();

        register_all_llzk_dialects(&registry);
        ctx.append_dialect_registry(&registry);
        ctx.load_all_available_dialects();
        Self {
            ctx,
            _registry: registry,
        }
    }
}

impl Default for LlzkContext {
    fn default() -> Self {
        Self::new()
    }
}

impl Deref for LlzkContext {
    type Target = Context;

    fn deref(&self) -> &Self::Target {
        &self.ctx
    }
}

impl Borrow<Context> for LlzkContext {
    fn borrow(&self) -> &Context {
        &self.ctx
    }
}

impl AsRef<Context> for LlzkContext {
    fn as_ref(&self) -> &Context {
        &self.ctx
    }
}

impl std::fmt::Debug for LlzkContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LlzkContext")
            .field("registered dialects", &self.registered_dialect_count())
            .field("loaded dialects", &self.loaded_dialect_count())
            .field("ctx", &self.ctx)
            .field("registry", &self._registry)
            .finish()
    }
}

/// Adds all LLZK dialects into the given registry.
pub fn register_all_llzk_dialects(registry: &DialectRegistry) {
    unsafe { llzkRegisterAllDialects(registry.to_raw()) }
}
