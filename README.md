# LLZK's Rust SDK

This repository is a collection of Rust crates that gives Rust developers access to [LLZK](https://veridise.github.io/llzk-lib/). 

> [!warning]
> These crates are under active development and things may change unexpectedly.

## Building tips

If you are using homebrew in macos you can access MLIR 18 by installing `llvm@18` with homebrew.
Setting the following environment variables configures the build system with the correct versions of LMLIR and its dependencies.

```
export MLIR_SYS_180_PREFIX=/opt/homebrew/opt/llvm@18/
export RUSTFLAGS='-L /opt/homebrew/lib/'
```
