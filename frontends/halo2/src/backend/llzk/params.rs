use llzk::LlzkContext;
use melior::Context;

#[derive(Clone)]
pub struct LlzkParams<'c> {
    context: &'c LlzkContext,
    top_level: Option<String>,
    #[cfg(feature = "lift-field-operations")]
    lift_fixed: bool,
}

impl<'c> LlzkParams<'c> {
    fn new(context: &'c LlzkContext) -> Self {
        Self {
            context,
            top_level: Default::default(),
            #[cfg(feature = "lift-field-operations")]
            lift_fixed: false,
        }
    }

    pub fn context(&self) -> &'c Context {
        self.context
    }

    pub fn top_level(&self) -> Option<&str> {
        self.top_level.as_deref()
    }
}

#[cfg(feature = "lift-field-operations")]
impl crate::ir::lift::LiftingCfg for LlzkParams<'_> {
    fn lifting_enabled(&self) -> bool {
        self.lift_fixed
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

#[cfg(feature = "lift-field-operations")]
impl<'c> LlzkParamsBuilder<'c> {
    pub fn lift_fixed(mut self) -> Self {
        self.0.lift_fixed = true;
        self
    }

    pub fn no_lift_fixed(mut self) -> Self {
        self.0.lift_fixed = false;
        self
    }
}

impl<'c> From<LlzkParamsBuilder<'c>> for LlzkParams<'c> {
    fn from(value: LlzkParamsBuilder<'c>) -> Self {
        value.0
    }
}
