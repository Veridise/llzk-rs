use super::{config_traits::CMakeConfig, llzk::LlzkBuild};
use anyhow::bail;
use anyhow::Result;
use cmake::Config;
#[cfg(not(target_os = "windows"))]
use std::os::unix::fs::symlink;
#[cfg(target_os = "windows")]
use std::os::windows::fs::symlink_file as symlink;
use std::{fs, path::PathBuf};

pub struct CompileCommands {
    dst: PathBuf,
}

impl CompileCommands {
    pub fn get() -> Option<Self> {
        if let Some(s) = option_env!("LLZK_EMIT_COMPILE_COMMANDS") {
            let mut dst = PathBuf::from(s);
            if !dst.exists() {
                return None;
            }
            if dst.is_dir() {
                dst = dst.join("compile_commands.json")
            }
            return Some(CompileCommands { dst });
        }
        None
    }

    pub fn link(&self, llzk: &LlzkBuild) -> Result<()> {
        let src = llzk.path().join("compile_commands.json");
        if !src.is_file() {
            bail!(
                "Compile commands requested but it was not found or is not a file: {}",
                src.display()
            );
        }

        if self.dst.is_symlink() || self.dst.exists() {
            fs::remove_file(&self.dst)?;
        }
        symlink(src, &self.dst)?;

        Ok(())
    }
}

impl CMakeConfig for CompileCommands {
    fn apply(&self, cmake: &mut Config) -> Result<()> {
        cmake.define("CMAKE_EXPORT_COMPILE_COMMANDS", "1");
        Ok(())
    }
}
