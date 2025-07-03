use anyhow::Result;
use bindgen::Builder;
use cc::Build;
use cmake::Config;
use std::{env, path::Path};

use super::config_traits::{BindgenConfig, CCConfig, CMakeConfig};

pub struct DefaultConfig<'a> {
    passes: &'a [&'a str],
    mlir_functions: &'a [&'a str],
    mlir_types: &'a [&'a str],
}

impl<'a> DefaultConfig<'a> {
    pub const fn new(
        passes: &'a [&'a str],
        mlir_functions: &'a [&'a str],
        mlir_types: &'a [&'a str],
    ) -> Self {
        Self {
            passes,
            mlir_functions,
            mlir_types,
        }
    }

    pub fn mlir_path(&self) -> Result<String> {
        Ok(env::var("DEP_MLIR_PREFIX")?)
    }

    pub fn mlir_cmake_path(&self) -> Result<String> {
        Ok(env::var("DEP_MLIR_CMAKE_DIR")?)
    }

    pub fn wrapper(&self) -> &'static str {
        "wrapper.h"
    }

    fn add_allowlist_patterns(&self, bindgen: Builder) -> Builder {
        let bindgen = bindgen
            .allowlist_item("[Ll]lzk.*")
            .allowlist_var("LLZK_.*")
            .allowlist_recursively(false);

        let bindgen = self.passes.iter().fold(bindgen, |bindgen, pass| {
            bindgen.allowlist_function(format!("mlir(Create|Register).*{pass}Pass"))
        });

        let bindgen = self.mlir_functions.iter().fold(bindgen, |bindgen, func| {
            bindgen.allowlist_function(format!("mlir{func}"))
        });
        self.mlir_types.iter().fold(bindgen, |bindgen, r#type| {
            bindgen.allowlist_type(format!("Mlir{type}"))
        })
    }
}

impl CMakeConfig for DefaultConfig<'_> {
    fn apply(&self, cmake: &mut Config) -> Result<()> {
        cmake
            .define("LLZK_BUILD_DEVTOOLS", "OFF")
            .define("BUILD_TESTING", "OFF")
            .define("LLVM_DIR", self.mlir_cmake_path()?)
            .define("MLIR_DIR", self.mlir_cmake_path()?)
            .define("LLVM_ROOT", self.mlir_path()?)
            .define("MLIR_ROOT", self.mlir_path()?);
        Ok(())
    }
}

impl BindgenConfig for DefaultConfig<'_> {
    fn apply(&self, bindgen: Builder) -> Result<Builder> {
        let path = self.mlir_path()?;
        Ok(self.add_allowlist_patterns(
            BindgenConfig::include_path(self, bindgen, Path::new(&path))
                .header(self.wrapper())
                .parse_callbacks(Box::new(bindgen::CargoCallbacks::new())),
        ))
    }
}

impl CCConfig for DefaultConfig<'_> {
    fn apply(&self, cc: &mut Build) -> Result<()> {
        let path = self.mlir_path()?;
        CCConfig::include_path(self, cc, Path::new(&path));
        Ok(())
    }
}
