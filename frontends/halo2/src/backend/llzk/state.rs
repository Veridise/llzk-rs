use std::marker::PhantomData;

use melior::Context;

use super::LlzkParams;

pub struct LlzkCodegenState<'c, F> {
    context: &'c Context,
    params: LlzkParams<'c>,
    _marker: PhantomData<F>,
}

impl<'c, F> LlzkCodegenState<'c, F> {
    pub fn context(&self) -> &'c Context {
        self.context
    }

    pub fn params(&self) -> &LlzkParams<'c> {
        &self.params
    }
}

impl<'c, F> From<LlzkParams<'c>> for LlzkCodegenState<'c, F> {
    fn from(params: LlzkParams<'c>) -> Self {
        Self {
            context: params.context(),
            params,
            _marker: PhantomData,
        }
    }
}
