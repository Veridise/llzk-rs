use anyhow::Result;
use bindgen::Builder;
use cc::Build;
use cmake::Config;
use std::{env, path::Path};

use super::config_traits::{BindgenConfig, CCConfig, CMakeConfig};

pub struct MlirConfig<'a> {
    passes: &'a [&'a str],
    mlir_functions: &'a [&'a str],
    mlir_types: &'a [&'a str],
}

impl<'a> MlirConfig<'a> {
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

    fn cmake_flags_list(&self) -> Result<Vec<(&'static str, String)>> {
        Ok(vec![
            ("LLVM_DIR", self.mlir_cmake_path()?),
            ("MLIR_DIR", self.mlir_cmake_path()?),
            ("LLVM_ROOT", self.mlir_path()?),
            ("MLIR_ROOT", self.mlir_path()?),
        ])
    }

    pub fn cmake_flags(&self) -> Result<Vec<String>> {
        Ok(self
            .cmake_flags_list()?
            .into_iter()
            .map(|(k, v)| format!("-D{k}={v}"))
            .collect())
    }
}

impl CMakeConfig for MlirConfig<'_> {
    fn apply(&self, cmake: &mut Config) -> Result<()> {
        for (k, v) in self.cmake_flags_list()? {
            cmake.define(k, v);
        }
        Ok(())
    }
}

impl BindgenConfig for MlirConfig<'_> {
    fn apply(&self, bindgen: Builder) -> Result<Builder> {
        let path = self.mlir_path()?;
        Ok(self.add_allowlist_patterns(
            BindgenConfig::include_path(self, bindgen, Path::new(&path))
                .header(self.wrapper())
                .parse_callbacks(Box::new(bindgen::CargoCallbacks::new())),
        ))
    }
}

impl CCConfig for MlirConfig<'_> {
    fn apply(&self, cc: &mut Build) -> Result<()> {
        let path = self.mlir_path()?;
        CCConfig::include_path(self, cc, Path::new(&path));
        Ok(())
    }
}
