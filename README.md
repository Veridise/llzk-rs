# LLZK's Rust SDK

This repository is a collection of Rust crates that gives Rust developers access to [LLZK](https://veridise.github.io/llzk-lib/). 

> [!warning]
> These crates are under active development and things may change unexpectedly.

## Usage (pre v1 release)

To use the llzk bindings add the crates to your Cargo.toml:

```
llzk-sys = { git = "https://github.com/Veridise/llzk-rs" }
llzk = { git = "https://github.com/Veridise/llzk-rs" }
```

## Building tips

If you are using homebrew in macos you can access MLIR 20 by installing `llvm@20` with homebrew.
Setting the following environment variables configures the build system with the correct versions of MLIR and its dependencies. 
Depending on the version of your default C++ compiler you may need to set `CXX` and `CC` to a compiler that supports C++ 20.

```
export MLIR_SYS_200_PREFIX=/opt/homebrew/opt/llvm@20/
export TABLEGEN_200_PREFIX=/opt/homebrew/opt/llvm@20/
export CXX=clang++
export CC=clang
export RUSTFLAGS='-L /opt/homebrew/lib/'
```

See [`llzk-sys`'s README](llzk-sys/README.md) for more details on setting up the build environment.

If working on LLZK via the submodule you can enable dumping the compile commands when building with cargo. Assuming the current directory is where your editor will look for the compile commands you can link them setting the `LLZK_EMIT_COMPILE_COMMANDS` environment variable as follows.

```
LLZK_EMIT_COMPILE_COMMANDS=$(pwd) cargo build
```
