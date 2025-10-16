//! Implementation of the fundamenal configuration.

use anyhow::Result;
use bindgen::Builder;
use cc::Build;
use cmake::Config;

use crate::{
    config_traits::{bindgen::BindgenConfig, cc::CCConfig, cmake::CMakeConfig},
    llzk::LIBDIR,
    mlir::MlirConfig,
};

/// Fundamental configuration for the different build tasks.
#[derive(Debug, Copy, Clone)]
pub struct DefaultConfig<'a> {
    mlir: MlirConfig<'a>,
}

impl<'a> DefaultConfig<'a> {
    /// Creates a new configuration.
    pub const fn new(
        passes: &'a [&'a str],
        mlir_functions: &'a [&'a str],
        mlir_types: &'a [&'a str],
    ) -> Self {
        Self {
            mlir: MlirConfig::new(passes, mlir_functions, mlir_types),
        }
    }

    /// Name of the wrapper header file that includes all the exported headers.
    pub fn wrapper(&self) -> &'static str {
        "wrapper.h"
    }
}

impl CMakeConfig for DefaultConfig<'_> {
    fn apply(&self, cmake: &mut Config) -> Result<()> {
        cmake
            .define("LLZK_BUILD_DEVTOOLS", "OFF")
            .define("LLZK_ENABLE_BINDINGS_PYTHON", "OFF")
            // Force the install lib directory for consistency between Linux distros
            // See: https://stackoverflow.com/questions/76517286/how-does-cmake-decide-to-make-a-lib-or-lib64-directory-for-installations
            .define("CMAKE_INSTALL_LIBDIR", LIBDIR)
            .define("BUILD_TESTING", "OFF");
        CMakeConfig::apply(&self.mlir, cmake)
    }
}

impl BindgenConfig for DefaultConfig<'_> {
    fn apply(&self, bindgen: Builder) -> Result<Builder> {
        let bindgen = bindgen
            .allowlist_item("[Ll]lzk.*")
            .allowlist_var("LLZK_.*")
            .allowlist_recursively(false)
            .impl_debug(true)
            .header(self.wrapper())
            .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()));
        BindgenConfig::apply(&self.mlir, bindgen)
    }
}

impl CCConfig for DefaultConfig<'_> {
    fn apply(&self, cc: &mut Build) -> Result<()> {
        CCConfig::apply(&self.mlir, cc)
    }
}
