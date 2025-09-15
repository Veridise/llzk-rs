use llzk::prelude::*;
use melior::Context;

#[derive(Clone)]
pub struct LlzkParams<'c> {
    context: &'c LlzkContext,
    top_level: Option<String>,
}

impl<'c> LlzkParams<'c> {
    fn new(context: &'c LlzkContext) -> Self {
        Self {
            context,
            top_level: Default::default(),
        }
    }

    pub fn context(&self) -> &'c Context {
        self.context
    }

    pub fn top_level(&self) -> Option<&str> {
        self.top_level.as_deref()
    }
}

pub struct LlzkParamsBuilder<'c>(LlzkParams<'c>);

impl<'c> LlzkParamsBuilder<'c> {
    pub fn new(context: &'c LlzkContext) -> Self {
        Self(LlzkParams::new(context))
    }

    pub fn with_top_level<S: ToString>(mut self, s: S) -> Self {
        self.0.top_level = Some(s.to_string());
        self
    }

    pub fn no_top_level(mut self) -> Self {
        self.0.top_level = None;
        self
    }

    pub fn build(self) -> LlzkParams<'c> {
        self.0
    }
}

impl<'c> From<LlzkParamsBuilder<'c>> for LlzkParams<'c> {
    fn from(value: LlzkParamsBuilder<'c>) -> Self {
        value.0
    }
}
