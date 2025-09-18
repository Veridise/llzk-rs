use melior::Context;

use crate::backend::codegen::CodegenParams;

use super::LlzkParams;

pub struct LlzkCodegenState<'c> {
    context: &'c Context,
    params: LlzkParams<'c>,
}

impl<'c> LlzkCodegenState<'c> {
    pub fn context(&self) -> &'c Context {
        self.context
    }

    pub fn params(&self) -> &LlzkParams<'c> {
        &self.params
    }
}

impl<'c> From<LlzkParams<'c>> for LlzkCodegenState<'c> {
    fn from(params: LlzkParams<'c>) -> Self {
        Self {
            context: params.context(),
            params,
        }
    }
}

impl CodegenParams for LlzkCodegenState<'_> {
    fn inlining_enabled(&self) -> bool {
        self.params().inline()
    }
}
