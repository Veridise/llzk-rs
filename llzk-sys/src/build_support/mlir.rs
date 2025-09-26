//! Configuration related to MLIR and LLVM.

use anyhow::{bail, Result};
use bindgen::Builder;
use cc::Build;
use cmake::Config;
use std::borrow::Cow;
use std::{env, path::Path};

use super::config_traits::{BindgenConfig, CCConfig, CMakeConfig};

/// Configuration specific to linking MLIR and LLVM.
pub struct MlirConfig<'a> {
    passes: &'a [&'a str],
    mlir_functions: &'a [&'a str],
    mlir_types: &'a [&'a str],
}

impl<'a> MlirConfig<'a> {
    /// Creates a new MLIR configuration.
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

    /// Returns the path configured by the `MLIR_SYS_200_PREFIX` environment variable.
    ///
    /// Returns [`Err`] if the path does not exists or is not a directory.
    pub fn mlir_path(&self) -> Result<Cow<'static, Path>> {
        let path = Path::new(env!("MLIR_SYS_200_PREFIX"));
        if !path.is_dir() {
            bail!("MLIR prefix path {} is not a directory", path.display());
        }
        Ok(Cow::Borrowed(path))
    }

    /// Returns the path `$MLIR_SYS_200_PREFIX/lib/cmake`.
    ///
    /// Returns [`Err`] if the path does not exists or is not a directory. Also if [`Self::mlir_path`]
    /// fails
    pub fn mlir_cmake_path(&self) -> Result<Cow<'static, Path>> {
        let path = self.mlir_path()?.join("lib/cmake");
        if !path.is_dir() {
            bail!("MLIR cmake path {} is not a directory", path.display());
        }
        Ok(Cow::Owned(path))
    }

    /// Name of the wrapper header file that includes all the exported headers.
    ///
    /// TODO: We should move this to DefaultConfig.
    pub fn wrapper(&self) -> &'static str {
        "wrapper.h"
    }

    /// Configures the allow list of functions and types for LLZK and MLIR.
    ///
    /// TODO: We should move the LLZK stuff to DefaultConfig.
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

    /// Returns the LLVM and MLIR directories for used by CMake to locate them.
    fn cmake_flags_list(&self) -> Result<Vec<(&'static str, Cow<'static, Path>)>> {
        Ok(vec![
            ("LLVM_DIR", self.mlir_cmake_path()?),
            ("MLIR_DIR", self.mlir_cmake_path()?),
            ("LLVM_ROOT", self.mlir_path()?),
            ("MLIR_ROOT", self.mlir_path()?),
        ])
    }

    /// Returns the LLVM and MLIR directories for used by CMake to locate them in CLI argument form.
    pub fn cmake_flags(&self) -> Result<Vec<String>> {
        Ok(self
            .cmake_flags_list()?
            .into_iter()
            .map(|(k, v)| format!("-D{k}={}", v.display()))
            .collect())
    }
}

impl CMakeConfig for MlirConfig<'_> {
    fn apply(&self, cmake: &mut Config) -> Result<()> {
        for (k, v) in self.cmake_flags_list()? {
            cmake.define(k, &*v);
        }
        Ok(())
    }
}

impl BindgenConfig for MlirConfig<'_> {
    fn apply(&self, bindgen: Builder) -> Result<Builder> {
        let path = self.mlir_path()?;
        Ok(self.add_allowlist_patterns(
            BindgenConfig::include_path(self, bindgen, &path)
                // TODO: Methods below should be moved to DefaultConfig.
                .header(self.wrapper())
                .parse_callbacks(Box::new(bindgen::CargoCallbacks::new())),
        ))
    }
}

impl CCConfig for MlirConfig<'_> {
    fn apply(&self, cc: &mut Build) -> Result<()> {
        let path = self.mlir_path()?;
        CCConfig::include_path(self, cc, &path);
        Ok(())
    }
}
