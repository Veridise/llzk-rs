use std::path::Path;

use anyhow::Result;
use bindgen::Builder;
use cc::Build;
use cmake::Config;

pub trait CMakeConfig {
    fn apply(&self, cmake: &mut Config) -> Result<()>;
}

pub trait BindgenConfig {
    fn apply(&self, bindgen: Builder) -> Result<Builder>;

    fn include_path(&self, bindgen: Builder, path: &Path) -> Builder {
        bindgen.clang_arg(format!("-I{}", path.join("include").display()))
    }

    fn include_paths(&self, bindgen: Builder, paths: &[&Path]) -> Builder {
        bindgen.clang_args(
            paths
                .iter()
                .map(|path| format!("-I{}", path.join("include").display())),
        )
    }
}

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
}
