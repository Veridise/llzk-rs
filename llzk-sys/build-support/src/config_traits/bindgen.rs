//! Traits and implementations for working with bindgen

use anyhow::Result;
use bindgen::{builder, Bindings, Builder};
use std::path::Path;

/// Trait for configurators of [`bindgen`] invocations.
pub trait BindgenConfig {
    /// Configures the given [`Builder`].
    ///
    /// Returns [`Err`] if any errors occur during configuration.
    fn apply(&self, bindgen: Builder) -> Result<Builder>;

    /// Helper method for adding the given path to the include paths list.
    fn include_path(&self, bindgen: Builder, path: &Path) -> Builder {
        bindgen.clang_arg(format!("-I{}", path.join("include").display()))
    }

    /// Helper method for adding multiple paths for the include paths list.
    fn include_paths(&self, bindgen: Builder, paths: &[&Path]) -> Builder {
        bindgen.clang_args(
            paths
                .iter()
                .map(|path| format!("-I{}", path.join("include").display())),
        )
    }

    /// Applies itself to a fresh bindgen [`Builder`] and then runs it.
    ///
    /// Returns the generated [`Bindings`].
    fn generate(&self) -> Result<Bindings> {
        let bindgen = self.apply(builder())?;
        Ok(bindgen.generate()?)
    }
}

impl<T1: BindgenConfig, T2: BindgenConfig, T3: BindgenConfig> BindgenConfig for (T1, T2, T3) {
    fn apply(&self, bindgen: Builder) -> Result<Builder> {
        self.2.apply(self.1.apply(self.0.apply(bindgen)?)?)
    }
}
