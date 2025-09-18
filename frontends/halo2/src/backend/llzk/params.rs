use llzk::prelude::*;
use melior::Context;

/// Configuration for the LLZK backend.
#[derive(Clone, Debug)]
pub struct LlzkParams<'c> {
    context: &'c LlzkContext,
    top_level: Option<String>,
    inline: bool,
}

impl<'c> LlzkParams<'c> {
    fn new(context: &'c LlzkContext) -> Self {
        Self {
            context,
            top_level: Default::default(),
            inline: false,
        }
    }

    /// Returns a reference to the [`melior::Context`].
    pub fn context(&self) -> &'c Context {
        self.context
    }

    /// Returns the name of the top-level structure if it was configured.
    pub fn top_level(&self) -> Option<&str> {
        self.top_level.as_deref()
    }

    /// Returns wether inlining is enabled or not.
    pub fn inline(&self) -> bool {
        self.inline
    }
}

/// Builder for creating [`LlzkParams`] instances.
#[derive(Debug)]
pub struct LlzkParamsBuilder<'c>(LlzkParams<'c>);

impl<'c> LlzkParamsBuilder<'c> {
    /// Creates a new builder.
    pub fn new(context: &'c LlzkContext) -> Self {
        Self(LlzkParams::new(context))
    }

    /// Sets the name of the top-level struct.
    pub fn with_top_level<S: ToString>(mut self, s: S) -> Self {
        self.0.top_level = Some(s.to_string());
        self
    }

    /// Removes the name of the top-level struct.
    pub fn no_top_level(mut self) -> Self {
        self.0.top_level = None;
        self
    }

    /// Sets lowering to inlining everything into one module.
    pub fn inline(mut self) -> Self {
        self.0.inline = true;
        self
    }

    /// Sets lowering to creating separate modules for each group.
    pub fn no_inline(mut self) -> Self {
        self.0.inline = false;
        self
    }

    /// Completes the build process and returns the parameters.
    pub fn build(self) -> LlzkParams<'c> {
        self.0
    }
}

impl<'c> From<LlzkParamsBuilder<'c>> for LlzkParams<'c> {
    fn from(value: LlzkParamsBuilder<'c>) -> Self {
        value.0
    }
}
