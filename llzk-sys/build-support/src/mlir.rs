//! Configuration related to MLIR and LLVM.

use anyhow::{bail, Result};
use bindgen::Builder;
use cc::Build;
use cmake::Config;
use std::{
    env,
    path::{Path, PathBuf},
    process::Command,
};

use super::config_traits::{bindgen::BindgenConfig, cc::CCConfig, cmake::CMakeConfig};

const LLVM_MAJOR_VERSION: usize = 20;

/// Configuration specific to linking MLIR and LLVM.
#[derive(Debug, Copy, Clone)]
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

    /// Returns the prefix of the LLVM installation.
    ///
    /// Returns [`Err`] if the path does not exists or is not a directory.
    pub fn mlir_path(&self) -> Result<PathBuf> {
        let path = PathBuf::from(llvm_config("--prefix")?);
        if !path.is_dir() {
            bail!("MLIR prefix path {} is not a directory", path.display());
        }
        Ok(path)
    }

    /// Returns the CMake path for MLIR.
    ///
    /// Returns [`Err`] if the path does not exists or is not a directory. Also if [`Self::mlir_path`]
    /// fails
    pub fn mlir_cmake_path(&self) -> Result<PathBuf> {
        let path = self.mlir_path()?.join("lib/cmake");
        if !path.is_dir() {
            bail!("MLIR cmake path {} is not a directory", path.display());
        }
        Ok(path)
    }

    /// Configures the allow list of functions and types for LLZK and MLIR.
    fn add_allowlist_patterns(&self, bindgen: Builder) -> Builder {
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
    fn cmake_flags_list(&self) -> Result<Vec<(&'static str, PathBuf)>> {
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
        Ok(self.add_allowlist_patterns(BindgenConfig::include_path(self, bindgen, &path)))
    }
}

impl CCConfig for MlirConfig<'_> {
    fn apply(&self, cc: &mut Build) -> Result<()> {
        let path = self.mlir_path()?;
        CCConfig::include_path(self, cc, &path);
        Ok(())
    }
}

/// Invokes `llvm-config`.
///
/// Taken from mlir-sys.
fn llvm_config(argument: &str) -> Result<String> {
    let prefix = env::var(format!("MLIR_SYS_{LLVM_MAJOR_VERSION}0_PREFIX"))
        .map(|path| Path::new(&path).join("bin"))
        .unwrap_or_default();
    let llvm_config_exe = if cfg!(target_os = "windows") {
        "llvm-config.exe"
    } else {
        "llvm-config"
    };

    let call = format!(
        "{} --link-static {argument}",
        prefix.join(llvm_config_exe).display(),
    );

    Ok(str::from_utf8(
        &if cfg!(target_os = "windows") {
            Command::new("cmd").args(["/C", &call]).output()?
        } else {
            Command::new("sh").arg("-c").arg(&call).output()?
        }
        .stdout,
    )?
    .trim()
    .to_string())
}
