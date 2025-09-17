use super::vars::NamingConvention;

/// Configuration for the Picus backend.
#[derive(Clone, Debug)]
pub struct PicusParams {
    expr_cutoff: Option<usize>,
    entrypoint: String,
    naming_convention: NamingConvention,
    optimize: bool,
}

impl PicusParams {
    /// Returns the naming convention of the variables.
    pub fn naming_convention(&self) -> NamingConvention {
        self.naming_convention
    }

    /// Returns true if optimization is enabled.
    pub fn optimize(&self) -> bool {
        self.optimize
    }

    /// Returns the maximum size of expressions, if configured.
    pub fn expr_cutoff(&self) -> Option<usize> {
        self.expr_cutoff
    }

    /// Returns the name of the top-level module.
    pub fn entrypoint(&self) -> &str {
        &self.entrypoint
    }

    fn new() -> Self {
        Self {
            expr_cutoff: None,
            entrypoint: "Main".to_owned(),
            naming_convention: NamingConvention::Short,
            optimize: true,
        }
    }
}

impl Default for PicusParams {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for configuring the parameters of the Picus backend.
#[derive(Debug, Default)]
pub struct PicusParamsBuilder(PicusParams);

impl PicusParamsBuilder {
    /// Creates a new builder using the parameter F as the prime.
    pub fn new() -> Self {
        Self(PicusParams::new())
    }

    /// Sets the maximum size for the expressions.
    pub fn expr_cutoff(self, expr_cutoff: usize) -> Self {
        let mut p = self.0;
        p.expr_cutoff = Some(expr_cutoff);
        Self(p)
    }

    /// Removes the configured value for the maximum size for expressions.
    pub fn no_expr_cutoff(self) -> Self {
        let mut p = self.0;
        p.expr_cutoff = None;
        Self(p)
    }

    /// Sets the name of the top-level module.
    pub fn entrypoint(self, name: &str) -> Self {
        let mut p = self.0;
        p.entrypoint = name.to_owned();
        Self(p)
    }

    /// Sets the naming convention to 'short'.
    pub fn short_names(mut self) -> Self {
        self.0.naming_convention = NamingConvention::Short;
        self
    }

    /// Enables optimizations.
    pub fn optimize(mut self) -> Self {
        self.0.optimize = true;
        self
    }

    /// Disables optimizations.
    pub fn no_optimize(mut self) -> Self {
        self.0.optimize = false;
        self
    }

    /// Finishes the build process and returns the parameters.
    pub fn build(self) -> PicusParams {
        self.0
    }
}

impl From<PicusParamsBuilder> for PicusParams {
    fn from(builder: PicusParamsBuilder) -> PicusParams {
        builder.0
    }
}
