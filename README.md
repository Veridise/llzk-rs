# LLZK's Rust SDK

This repository is a collection of Rust crates that gives Rust developers access to [LLZK](https://veridise.github.io/llzk-lib/). 

> [!warning]
> These crates are under active development and things may change unexpectedly.

## Usage 

To use the llzk bindings add the crates to your Cargo.toml:

```
llzk-sys = { git = "https://github.com/Veridise/llzk-rs" }
llzk = { git = "https://github.com/Veridise/llzk-rs" }
```

## Building tips

If you are using homebrew in macos you can access MLIR 18 by installing `llvm@18` with homebrew.
Setting the following environment variables configures the build system with the correct versions of MLIR and its dependencies.

```
export MLIR_SYS_180_PREFIX=/opt/homebrew/opt/llvm@18/
export TABLEGEN_180_PREFIX=/opt/homebrew/opt/llvm@18/
export LIBCLANG_PATH=/opt/homebrew/opt/llvm@18/lib
export RUSTFLAGS='-L /opt/homebrew/lib/'
```

If working on LLZK via the submodule you can enable dumping the compile commands when building with cargo. Assuming the current directory is where your editor will look for the compile commands you can link them setting the `LLZK_EMIT_COMPILE_COMMANDS` environment variable as follows.

```
LLZK_EMIT_COMPILE_COMMANDS=$(pwd) cargo build
```
