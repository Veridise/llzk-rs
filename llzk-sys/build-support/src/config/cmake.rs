use std::path::{Path, PathBuf};

use anyhow::Result;
use cmake::Config;

pub trait CMakeConfig {
    fn apply(&self, cmake: &mut Config) -> Result<()>;

    fn build(&self, path: impl AsRef<Path>) -> Result<PathBuf> {
        let mut cmake = Config::new(path);
        self.apply(&mut cmake)?;
        Ok(cmake.build())
    }
}

impl<T: CMakeConfig> CMakeConfig for Option<T> {
    fn apply(&self, cmake: &mut Config) -> Result<()> {
        match self {
            Some(conf) => conf.apply(cmake),
            None => Ok(()),
        }
    }
}

impl<T: CMakeConfig> CMakeConfig for &T {
    fn apply(&self, cmake: &mut Config) -> Result<()> {
        (*self).apply(cmake)
    }
}

impl<T1: CMakeConfig, T2: CMakeConfig> CMakeConfig for (T1, T2) {
    fn apply(&self, cmake: &mut Config) -> Result<()> {
        self.0.apply(cmake)?;
        self.1.apply(cmake)
    }
}

impl<T1: CMakeConfig, T2: CMakeConfig, T3: CMakeConfig> CMakeConfig for (T1, T2, T3) {
    fn apply(&self, cmake: &mut Config) -> Result<()> {
        self.0.apply(cmake)?;
        self.1.apply(cmake)?;
        self.2.apply(cmake)
    }
}
