//! Configuration related to MLIR and LLVM.

use anyhow::{bail, Result};
use bindgen::Builder;
use cc::Build;
use cmake::Config;
use std::borrow::Cow;
use std::path::Path;

use super::config::{bindgen::BindgenConfig, cc::CCConfig, cmake::CMakeConfig};

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
    pub fn mlir_path(&self) -> Result<Option<Cow<'static, Path>>> {
        let Some(path) = option_env!("MLIR_SYS_200_PREFIX").map(Path::new) else {
            return Ok(None);
        };

        if !path.is_dir() {
            bail!("MLIR prefix path {} is not a directory", path.display());
        }
        Ok(Some(Cow::Borrowed(path)))
    }

    /// Returns the path `$MLIR_SYS_200_PREFIX/lib/cmake`.
    ///
    /// Returns [`Err`] if the path does not exists or is not a directory. Also if [`Self::mlir_path`]
    /// fails
    pub fn mlir_cmake_path(&self) -> Result<Option<Cow<'static, Path>>> {
        self.mlir_path()?
            .map(|path| {
                let path = path.join("lib/cmake");
                if !path.is_dir() {
                    bail!("MLIR cmake path {} is not a directory", path.display());
                }
                Ok(Cow::Owned(path))
            })
            .transpose()
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
        let mut list = Vec::with_capacity(4);
        if let Some(path) = self.mlir_path()? {
            list.push(("LLVM_ROOT", path.clone()));
            list.push(("MLIR_ROOT", path));
        }
        if let Some(cmake_path) = self.mlir_cmake_path()? {
            list.push(("LLVM_DIR", cmake_path.clone()));
            list.push(("MLIR_DIR", cmake_path));
        }
        Ok(list)
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
        Ok(self
            .add_allowlist_patterns(match path {
                Some(path) => BindgenConfig::include_path(self, bindgen, &path),
                None => bindgen,
            })
            // TODO: Methods below should be moved to DefaultConfig.
            .header(self.wrapper())
            .parse_callbacks(Box::new(bindgen::CargoCallbacks::new())))
    }
}

impl CCConfig for MlirConfig<'_> {
    fn apply(&self, cc: &mut Build) -> Result<()> {
        if let Some(path) = self.mlir_path()? {
            CCConfig::include_path(self, cc, &path);
        }
        Ok(())
    }
}
