//! Traits and implementations for working with CMake

use std::path::{Path, PathBuf};

use anyhow::Result;
use cmake::Config;

/// Trait for configurators of CMake invocations.
pub trait CMakeConfig {
    /// Configures the given [`Config`].
    ///
    /// Returns [`Err`] if any errors occur during configuration.
    fn apply(&self, cmake: &mut Config) -> Result<()>;

    /// Chains two configs together.
    fn then<O>(self, other: O) -> (Self, O)
    where
        O: CMakeConfig,
        Self: Sized,
    {
        (self, other)
    }

    /// Applies itself to a fresh CMake [`Config`] and then runs it.
    ///
    /// Takes the source path as input and returns the path where CMake built the project.
    fn build(&self, src: impl AsRef<Path>) -> Result<PathBuf> {
        // Config::new takes an impl of AsRef<Path> so we do the same here.
        let mut cmake = Config::new(src);
        self.apply(&mut cmake)?;
        Ok(cmake.build())
    }
}

impl<T: CMakeConfig> CMakeConfig for Option<T> {
    fn apply(&self, cmake: &mut Config) -> Result<()> {
        match self {
            Some(config) => config.apply(cmake),
            None => Ok(()),
        }
    }
}

impl<T1: CMakeConfig, T2: CMakeConfig> CMakeConfig for (T1, T2) {
    fn apply(&self, cmake: &mut Config) -> Result<()> {
        self.0.apply(cmake)?;
        self.1.apply(cmake)
    }
}
