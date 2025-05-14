# mlir-18-sys

Rust bindings to [the MLIR C API](https://mlir.llvm.org/docs/CAPI/) targeting LLVM and MLIR version 18.x.

## Usage 

Add `mlir-18-sys` to your Cargo project.

```
cargo add mlir-18-sys
```

The build process uses `llvm-config` for detecting the location of LLVM and MLIR. Similarly to `mlir-sys`, you can use the `MLIR_SYS_180_PREFIX` environment variable for customizing the location where to find `llvm-config`.

## mlir-sys conflict

This crate will also generate bindings for MLIR and will link against LLVM and MLIR. This is because LLZK uses MLIR 18 while [`mlir-sys`](https://crates.io/crates/mlir-sys) uses LLVM 20. If you use this crate don't use `mlir-sys`.

## License

While not a fork of `mlir-sys` this crate is heavily influenced by it. For that reason this crate in particular is dual licensed with MIT and Apache2.
