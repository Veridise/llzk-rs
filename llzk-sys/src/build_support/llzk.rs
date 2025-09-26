//! Configuration and build steps related to building LLZK and other projects based on LLZK.

use anyhow::{anyhow, bail, Result};
use bindgen::Builder;
use cc::Build;
use cmake::Config;
use std::{
    collections::HashSet,
    ffi::OsStr,
    fs,
    hash::Hash,
    path::{Path, PathBuf},
    process::Command,
};

use crate::build_support::mlir::MlirConfig;

use super::config_traits::{BindgenConfig, CCConfig, CMakeConfig};

/// Location relative to CMake's output directory where libraries will be installed.
pub const LIBDIR: &str = "lib";

/// The result of building `llzk-lib` CMake project.
pub struct LlzkBuild {
    path: PathBuf,
    cached_libraries: Option<(Vec<PathBuf>, Vec<String>)>,
}

impl From<PathBuf> for LlzkBuild {
    fn from(path: PathBuf) -> Self {
        Self {
            path,
            cached_libraries: Default::default(),
        }
    }
}

/// Delete me.
const DUMMY_CXX: &str = "int main() {}";

/// Delete me.
fn dummy_cmakelist(path: &Path) -> String {
    let content = format!(
        r#"
cmake_minimum_required(VERSION 3.15)
project(Dummy)

list(APPEND CMAKE_PREFIX_PATH "{}")
find_package(LLZK REQUIRED)

add_executable(dummy dummy.cpp)
target_link_libraries(dummy PRIVATE LLZK::LLZKCAPI)
        "#,
        path.display()
    );
    eprintln!("CMakeLists.txt content: {content:?}");
    content
}

/// Delete me.
fn extract_libraries_from_dummy(path: &Path) -> Result<(Vec<PathBuf>, Vec<String>)> {
    let workdir = tempfile::tempdir()?;
    let dummy_cxx = workdir.path().join("dummy.cpp");
    let cmakelists = workdir.path().join("CMakeLists.txt");
    let build_dir = workdir.path().join("build");

    fs::create_dir_all(&build_dir)?;
    fs::write(dummy_cxx, DUMMY_CXX)?;
    fs::write(cmakelists, dummy_cmakelist(path))?;

    let mlir = MlirConfig::new(&[], &[], &[]);
    // Configure step
    Command::new("cmake")
        .current_dir(&build_dir)
        .arg("..")
        .args(mlir.cmake_flags()?)
        .status()
        .map_err(Into::into)
        .and_then(|status| {
            if status.success() {
                Ok(())
            } else {
                Err(anyhow!(
                    "Failed to run cmake while configuring the dummy project"
                ))
            }
        })?;
    let (dirs, libs): (Vec<Option<PathBuf>>, Vec<Option<String>>) = Command::new("cmake")
        .current_dir(&build_dir)
        .args(["--build", ".", "--target", "dummy", "--verbose"])
        .output()
        .map_err(Into::into)
        .and_then(|output| {
            eprintln!("output = {output:?}");
            let stderr = String::from_utf8_lossy(&output.stderr);
            if !output.status.success() {
                bail!("Failed to run cmake while building the dummy project. Stderr: {stderr}");
            }

            let stdout = String::from_utf8_lossy(&output.stdout);
            Ok(format!("{stdout}\n{stderr}"))
        })?
        .split_whitespace()
        .map(|token| {
            let lib_path = token.strip_prefix("-L").map(Path::new);
            let lib_name = token.strip_prefix("-l");
            // Assert that they are mutually exclusive
            assert!(
                (lib_path.is_none() && lib_name.is_none())
                    || lib_path.is_none() == lib_name.is_some()
            );
            let path = token.strip_suffix(".a").map(Path::new);
            (
                lib_path
                    .or_else(|| path.and_then(Path::parent))
                    .map(Path::to_path_buf),
                lib_name
                    .or_else(|| {
                        path.and_then(Path::file_name)
                            .and_then(OsStr::to_str)
                            .and_then(|s| s.strip_prefix("lib"))
                    })
                    .map(ToOwned::to_owned),
            )
        })
        .unzip();

    fn reduce<T: Hash + Eq + PartialEq>(i: impl IntoIterator<Item = Option<T>>) -> Vec<T> {
        i.into_iter()
            .flatten()
            .collect::<HashSet<T>>()
            .into_iter()
            .collect()
    }

    Ok((reduce(dirs), reduce(libs)))
}

impl LlzkBuild {
    fn libraries(
        &mut self,
    ) -> Result<(
        impl IntoIterator<Item = &Path>,
        impl IntoIterator<Item = &str>,
    )> {
        if self.cached_libraries.is_none() {
            self.cached_libraries = Some(extract_libraries_from_dummy(&self.path)?);
        }

        Ok(self
            .cached_libraries
            .as_ref()
            .map(|(p, s)| (p.iter().map(PathBuf::as_path), s.iter().map(|s| s.as_str())))
            .unwrap())
    }

    /// Returns the list of LLZK libraries that need to be linked.
    pub fn library_names(&mut self) -> Result<impl Iterator<Item = &str>> {
        Ok(self.libraries()?.1.into_iter())
    }

    /// Returns the list of paths where to find the libraries.
    pub fn link_paths(&mut self) -> Result<impl Iterator<Item = &Path>> {
        Ok(self.libraries()?.0.into_iter())
    }

    /// Build directory.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Builds `llzk-lib`'s CMake project using the given configuration.
    ///
    /// Emits cargo commands to avoid rerunning unless `llzk-lib` changes.
    pub fn build(cfgs: &[&dyn CMakeConfig]) -> Result<Self> {
        let mut cmake = Config::new(Self::src_path());
        for cfg in cfgs {
            cfg.apply(&mut cmake)?;
        }

        println!(
            "cargo::rerun-if-changed={}/include",
            Self::src_path().display()
        );
        println!("cargo::rerun-if-changed={}/lib", Self::src_path().display());
        Ok(cmake.build().join("build").into())
    }

    /// Returns the path, relative to `llzk-sys`'s build script, where LLZK is.
    pub fn src_path() -> &'static Path {
        Path::new("llzk-lib")
    }
}

impl BindgenConfig for LlzkBuild {
    fn apply(&self, bindgen: Builder) -> Result<Builder> {
        Ok(BindgenConfig::include_paths(
            self,
            bindgen,
            &[&self.path, Self::src_path()],
        ))
    }
}

impl CCConfig for LlzkBuild {
    fn apply(&self, cc: &mut Build) -> Result<()> {
        CCConfig::include_paths(self, cc, &[&self.path, Self::src_path()]);
        Ok(())
    }
}
