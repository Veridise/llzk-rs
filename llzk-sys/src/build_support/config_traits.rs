//! Traits for configurators of the tools used for building.

use std::path::Path;

use anyhow::Result;
use bindgen::Builder;
use cc::Build;
use cmake::Config;

/// Trait for configurators of CMake invocations.
pub trait CMakeConfig {
    /// Configures the given [`Config`].
    ///
    /// Returns [`Err`] if any errors occur during configuration.
    fn apply(&self, cmake: &mut Config) -> Result<()>;
}

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
}

/// Trait for configurators of [`cc`] invocations.
pub trait CCConfig {
    /// Configures the given [`Build`].
    ///
    /// Returns [`Err`] if any errors occur during configuration.
    fn apply(&self, cc: &mut Build) -> Result<()>;

    /// Helper method for adding the given path to the include paths list.
    fn include_path(&self, cc: &mut Build, path: &Path) {
        cc.include(path.join("include"));
    }

    /// Helper method for adding multiple paths for the include paths list.
    fn include_paths(&self, cc: &mut Build, paths: &[&Path]) {
        for path in paths {
            CCConfig::include_path(self, cc, path);
        }
    }
}
