pub use codegen::LlzkCodegen;
use melior::ir::Module;
pub use state::LlzkCodegenState;

use super::Backend;
pub use params::{LlzkParams, LlzkParamsBuilder};

mod codegen;
mod counter;
mod extras;
mod factory;
mod lowering;
mod params;
mod state;

pub type LlzkBackend<'c, 's> = Backend<LlzkCodegen<'c, 's>, LlzkCodegenState<'c>>;

pub struct LlzkOutput<'c> {
    module: Module<'c>,
}

impl<'c> LlzkOutput<'c> {
    pub fn module(&self) -> &Module<'c> {
        &self.module
    }
}

impl<'c> From<Module<'c>> for LlzkOutput<'c> {
    fn from(module: Module<'c>) -> Self {
        Self { module }
    }
}

impl std::fmt::Display for LlzkOutput<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.module)
    }
}
