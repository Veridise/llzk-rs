use anyhow::Result;
use bindgen::Builder;
use cc::Build;
use cmake::Config;

use super::{
    config_traits::{BindgenConfig, CCConfig, CMakeConfig},
    mlir::MlirConfig,
};

pub struct DefaultConfig<'a> {
    mlir: MlirConfig<'a>,
}

impl<'a> DefaultConfig<'a> {
    pub const fn new(
        passes: &'a [&'a str],
        mlir_functions: &'a [&'a str],
        mlir_types: &'a [&'a str],
    ) -> Self {
        Self {
            mlir: MlirConfig::new(passes, mlir_functions, mlir_types),
        }
    }
}

impl CMakeConfig for DefaultConfig<'_> {
    fn apply(&self, cmake: &mut Config) -> Result<()> {
        cmake
            .define("LLZK_BUILD_DEVTOOLS", "OFF")
            .define("BUILD_TESTING", "OFF");
        CMakeConfig::apply(&self.mlir, cmake)
    }
}

impl BindgenConfig for DefaultConfig<'_> {
    fn apply(&self, bindgen: Builder) -> Result<Builder> {
        BindgenConfig::apply(&self.mlir, bindgen)
    }
}

impl CCConfig for DefaultConfig<'_> {
    fn apply(&self, cc: &mut Build) -> Result<()> {
        CCConfig::apply(&self.mlir, cc)
    }
}
