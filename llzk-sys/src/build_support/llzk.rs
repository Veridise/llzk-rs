use anyhow::Result;
use bindgen::Builder;
use cc::Build;
use cmake::Config;
use glob::{glob, GlobError, Paths};
use std::{
    collections::HashSet,
    ffi::OsStr,
    path::{Path, PathBuf},
};

use super::config_traits::{BindgenConfig, CCConfig, CMakeConfig};

pub struct LlzkBuild {
    path: PathBuf,
}

impl From<PathBuf> for LlzkBuild {
    fn from(path: PathBuf) -> Self {
        Self { path }
    }
}

impl LlzkBuild {
    fn libraries(&self) -> Result<Paths> {
        Ok(glob(self.path.join("**/*.a").to_str().unwrap())?)
    }

    pub fn library_names(&self) -> Result<Vec<String>> {
        let libs = self.libraries()?;
        libs.map(archive_name_from_path).collect()
    }

    pub fn link_paths(&self) -> Result<HashSet<PathBuf>> {
        self.libraries()?.map(parent_of_lib_path).collect()
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

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

fn parse_archive_name(name: &str) -> Option<&str> {
    if let Some(name) = name.strip_prefix("lib") {
        name.strip_suffix(".a")
    } else {
        None
    }
}

fn archive_name_from_path(path: Result<PathBuf, GlobError>) -> Result<String> {
    let path = path?;
    path.file_name()
        .and_then(OsStr::to_str)
        .and_then(parse_archive_name)
        .map(|s| s.to_string())
        .ok_or(anyhow::anyhow!(
            "Failed to parse archive name of {}",
            path.display()
        ))
}

fn parent_of_lib_path(path: Result<PathBuf, GlobError>) -> Result<PathBuf> {
    let path = path?;
    path.parent()
        .map(|p| p.to_path_buf())
        .ok_or(anyhow::anyhow!(
            "Failed to get parent from {}",
            path.display()
        ))
}
