use anyhow::Result;
use bindgen::Builder;
use cc::Build;
use std::path::{Path, PathBuf};

use super::config::{bindgen::BindgenConfig, cc::CCConfig, cmake::CMakeConfig};

pub struct LlzkBuild<'a> {
    path: PathBuf,
    src_path: &'a Path,
}

impl<'a> LlzkBuild<'a> {
    fn new(src: &'a Path, dst: PathBuf) -> Self {
        Self {
            path: dst,
            src_path: src,
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn emit_cargo_instructions(&self) -> Result<()> {
        println!(
            "cargo:rerun-if-changed={}/include",
            self.src_path().display()
        );
        println!("cargo:rerun-if-changed={}/lib", self.src_path().display());

        let lib_path = self.path.join("lib64");
        println!("cargo:rustc-link-search=native={}", lib_path.display());
        for entry in lib_path.read_dir()? {
            let name = entry?
                .file_name()
                .into_string()
                .map_err(|orig| anyhow::anyhow!("Failed to convert {orig:?} into a String"))?;
            if let Some(lib) = name.strip_prefix("lib").and_then(|s| s.strip_suffix(".a")) {
                println!("cargo:rustc-link-lib=static={lib}");
            }
        }

        Ok(())
    }

    pub fn build(cfg: impl CMakeConfig, src: &'a Path) -> Result<Self> {
        Ok(Self::new(src, cfg.build(src)?))
    }

    pub fn src_path(&self) -> &'a Path {
        self.src_path
    }
}

impl BindgenConfig for LlzkBuild<'_> {
    fn apply(&self, bindgen: Builder) -> Result<Builder> {
        Ok(BindgenConfig::include_paths(
            self,
            bindgen,
            &[&self.path.join("build"), self.src_path],
        ))
    }
}

impl CCConfig for LlzkBuild<'_> {
    fn apply(&self, cc: &mut Build) -> Result<()> {
        CCConfig::include_paths(self, cc, &[&self.path.join("build"), self.src_path]);
        Ok(())
    }
}
