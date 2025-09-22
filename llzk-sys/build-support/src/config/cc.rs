use std::path::Path;

use anyhow::Result;
use cc::Build;

pub trait CCConfig {
    fn apply(&self, cc: &mut Build) -> Result<()>;

    fn include_path(&self, cc: &mut Build, path: &Path) {
        cc.include(path.join("include"));
    }

    fn include_paths(&self, cc: &mut Build, paths: &[&Path]) {
        for path in paths {
            CCConfig::include_path(self, cc, path);
        }
    }

    fn compile(&self, output: &str) -> Result<()> {
        let mut cc = cc::Build::new();
        self.apply(&mut cc)?;
        cc.try_compile(output)?;
        Ok(())
    }
}

impl<T1: CCConfig, T2: CCConfig, T3: CCConfig> CCConfig for (T1, T2, T3) {
    fn apply(&self, cc: &mut Build) -> Result<()> {
        self.0.apply(cc)?;
        self.1.apply(cc)?;
        self.2.apply(cc)
    }
}
