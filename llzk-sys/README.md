# llzk-sys

Rust bindings to the LLZK C API.

## Usage 

Simply add `llzk-sys` to your Cargo project.
LLZK is maintained as a submodule and built in place when building the crate. 

```
cargo add llzk-sys
```

The crate depends on `mlir-18-sys` whose build process uses `llvm-config` for detecting the location of LLVM and MLIR. Similarly to `mlir-sys`, you can use the `MLIR_SYS_180_PREFIX` environment variable for customizing the location where to find `llvm-config`.

