use anyhow::{Context, Result};
use bindgen::Builder;
use cc::Build;
use std::path::{Path, PathBuf};

use super::config::{bindgen::BindgenConfig, cc::CCConfig, cmake::CMakeConfig};

pub const LIBDIR: &'static str = "lib";

pub struct LlzkBuild<'a> {
    dst_path: PathBuf,
    src_path: &'a Path,
}

impl<'a> LlzkBuild<'a> {
    fn new(src: &'a Path, dst: PathBuf) -> Self {
        Self {
            dst_path: dst,
            src_path: src,
        }
    }

    pub fn dst_path(&self) -> &Path {
        &self.dst_path
    }

    pub fn lib_path(&self) -> PathBuf {
        self.dst_path.join(LIBDIR)
    }

    pub fn build_path(&self) -> PathBuf {
        self.dst_path.join("build")
    }

    pub fn emit_cargo_instructions(&self) -> Result<()> {
        println!(
            "cargo:rerun-if-changed={}/include",
            self.src_path().display()
        );
        println!("cargo:rerun-if-changed={}/lib", self.src_path().display());

        let lib_path = self.lib_path();

        println!("cargo:rustc-link-search=native={}", lib_path.display());
        // Adding the whole archive modifier is optional since only seems to be required for some GNU-like linkers.
        let modifier = if std::env::var("LLZK_SYS_ENABLE_WHOLE_ARCHIVE").is_ok_and(|var| var == "1")
        {
            ":+whole-archive"
        } else {
            ""
        };
        for entry in lib_path
            .read_dir()
            .with_context(|| format!("Failed to read directory {}", lib_path.display()))?
        {
            let name = entry
                .context("Failed to read entry in directory")?
                .file_name()
                .into_string()
                .map_err(|orig| anyhow::anyhow!("Failed to convert {orig:?} into a String"))?;
            if let Some(lib) = name.strip_prefix("lib").and_then(|s| s.strip_suffix(".a")) {
                println!("cargo:rustc-link-lib=static{modifier}={lib}");
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
            &[&self.build_path(), self.src_path],
        ))
    }
}

impl CCConfig for LlzkBuild<'_> {
    fn apply(&self, cc: &mut Build) -> Result<()> {
        CCConfig::include_paths(self, cc, &[&self.build_path(), self.src_path]);
        Ok(())
    }
}
