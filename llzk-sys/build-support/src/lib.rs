//! Helper modules for building and linking LLZK and generating the Rust bindings.

#![deny(rustdoc::broken_intra_doc_links)]
#![deny(missing_debug_implementations)]
#![deny(missing_docs)]

use std::{io::stdout, path::Path};

use crate::{
    compile_commands::CompileCommands,
    config_traits::cmake::CMakeConfig,
    llzk::{whole_archive_config, LlzkBuild},
};
use anyhow::Result;

pub mod compile_commands;
pub mod config_traits;
pub mod default;
pub mod llzk;
pub mod mlir;
pub mod wrap_static_fns;

/// Builds `llzk-lib` and emits the cargo instructions to link against it.
pub fn build_llzk<'a>(src: &'a Path, cfg: impl CMakeConfig) -> Result<LlzkBuild<'a>> {
    let compile_commands = CompileCommands::get();
    let llzk = LlzkBuild::new(src, cfg.then(compile_commands).build(src)?);
    if let Some(compile_commands) = compile_commands {
        compile_commands.link(&llzk)?;
    }
    llzk.emit_cargo_commands(stdout(), whole_archive_config())?;
    Ok(llzk)
}
