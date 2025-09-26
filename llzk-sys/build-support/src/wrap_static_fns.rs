//! Configuration for building static functions found by [`bindgen`].

use super::config::{bindgen::BindgenConfig, cc::CCConfig};
use anyhow::{bail, Result};
use bindgen::Builder;
use cc::Build;
use std::{
    env,
    path::{Path, PathBuf},
};

/// Configuration for building the library that include the implementation of static functions in
/// LLZK's CAPI.
pub struct WrapStaticFns {
    dst: PathBuf,
}

impl WrapStaticFns {
    /// Creates a new configuration.
    pub fn new(out_dir: &Path) -> Self {
        Self {
            dst: out_dir.join("bindgen_wrap"),
        }
    }

    /// Returns the name of the C source file.
    pub fn source_file(&self) -> PathBuf {
        let mut copy = self.dst.clone();
        copy.set_extension("c");
        copy
    }
}

impl BindgenConfig for WrapStaticFns {
    fn apply(&self, bindgen: Builder) -> Result<Builder> {
        Ok(bindgen
            .wrap_static_fns(true)
            .wrap_static_fns_path(&self.dst))
    }
}

impl CCConfig for WrapStaticFns {
    fn apply(&self, cc: &mut Build) -> Result<()> {
        if !self.source_file().is_file() {
            bail!("Source file not found! {}", self.source_file().display());
        }

        cc.file(self.source_file())
            .include(env::var("CARGO_MANIFEST_DIR")?);
        Ok(())
    }
}
