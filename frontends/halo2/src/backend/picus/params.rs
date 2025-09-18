use super::vars::NamingConvention;

/// Configuration for the Picus backend.
#[derive(Clone, Debug)]
pub struct PicusParams {
    expr_cutoff: Option<usize>,
    entrypoint: Option<String>,
    naming_convention: NamingConvention,
    optimize: bool,
    inline: bool,
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
        self.entrypoint.as_deref().unwrap_or("Main")
    }

    fn new() -> Self {
        Self {
            expr_cutoff: None,
            entrypoint: None,
            naming_convention: NamingConvention::Short,
            optimize: true,
            inline: false,
        }
    }

    /// Returns wether inlining is enabled or not.
    pub fn inline(&self) -> bool {
        self.inline
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
    pub fn expr_cutoff(&mut self, expr_cutoff: usize) -> &mut Self {
        self.0.expr_cutoff = Some(expr_cutoff);
        self
    }

    /// Removes the configured value for the maximum size for expressions.
    pub fn no_expr_cutoff(&mut self) -> &mut Self {
        self.0.expr_cutoff = None;
        self
    }

    /// Sets the name of the top-level module.
    pub fn entrypoint(&mut self, name: &str) -> &mut Self {
        self.0.entrypoint = Some(name.to_owned());
        self
    }

    /// Sets the naming convention to 'short'.
    pub fn short_names(&mut self) -> &mut Self {
        self.0.naming_convention = NamingConvention::Short;
        self
    }

    /// Enables optimizations.
    pub fn optimize(&mut self) -> &mut Self {
        self.0.optimize = true;
        self
    }

    /// Disables optimizations.
    pub fn no_optimize(&mut self) -> &mut Self {
        self.0.optimize = false;
        self
    }

    /// Sets lowering to inlining everything into one module.
    pub fn inline(&mut self) -> &mut Self {
        self.0.inline = true;
        self
    }

    /// Sets lowering to creating separate modules for each group.
    pub fn no_inline(&mut self) -> &mut Self {
        self.0.inline = false;
        self
    }

    /// Finishes the build process and returns the parameters.
    pub fn build(&mut self) -> PicusParams {
        std::mem::replace(&mut self.0, PicusParams::new())
    }
}

impl From<PicusParamsBuilder> for PicusParams {
    fn from(builder: PicusParamsBuilder) -> PicusParams {
        builder.0
    }
}
