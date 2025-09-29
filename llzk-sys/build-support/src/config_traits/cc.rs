//! Traits and implementations for working with CC

use anyhow::Result;
use cc::Build;
use std::path::Path;

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

    /// Applies itself to a fresh CMake [`Config`] and then runs it.
    ///
    /// Takes the source path as input and returns the path where CMake built the project.
    fn try_compile(&self, name: &str) -> Result<()> {
        let mut cc = Build::new();
        self.apply(&mut cc)?;
        Ok(cc.try_compile(name)?)
    }
}

impl<T1: CCConfig, T2: CCConfig, T3: CCConfig> CCConfig for (T1, T2, T3) {
    fn apply(&self, cc: &mut Build) -> Result<()> {
        self.0.apply(cc)?;
        self.1.apply(cc)?;
        self.2.apply(cc)
    }
}
