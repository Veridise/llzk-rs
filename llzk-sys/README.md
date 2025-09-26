# llzk-sys

Rust bindings to the LLZK C API.

## Usage 

Add `llzk-sys` to your Cargo project.
LLZK is maintained as a submodule and built in place when building the crate. 

```
cargo add llzk-sys
```

For building set the environment variable `MLIR_SYS_200_PREFIX` with the path to a distribution of LLVM 20. For example, to use LLVM 20 installed via Homebrew in macOS (`brew install llvm@20`) set it to `/opt/homebrew/opt/llvm@20/`. This is the only required variable. The recommened environment for building is as follows.

```
export MLIR_SYS_200_PREFIX=...
export TABLEGEN_200_PREFIX=<same as MLIR_SYS_200_PREFIX>
# Some system's default compilers are a bit old and you need a recent version of clang (+18) or gcc (+13).
export CXX=clang++
export CC=clang
# If MLIR's and LLVM's installation is not on standard paths set them here.
# For example, for a homebrew version of LLVM on macOS use this path.
export RUSTFLAGS='-L /opt/homebrew/lib/'
# This variable may need to be configured on macOS as well. If building fails try setting it.
export LIBCLANG_PATH=$MLIR_SYS_200_PREFIX/lib
```

Only Linux and macOS are planned to be supported.
