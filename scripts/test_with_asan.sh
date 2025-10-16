#!/bin/sh 

export RUSTFLAGS="$RUSTFLAGS -Zsanitizer=address"

if [ $(uname) == "Darwin" ]; then 
  export RUSTFLAGS="$RUSTFLAGS -Zexternal-clangrt -L $MLIR_SYS_200_PREFIX/lib/clang/20/lib/darwin/ -l clang_rt.asan_osx_dynamic"
fi
 
export ASAN_OPTIONS="debug=true:detect_leaks=1:symbolize=1"
export ASAN_SYMBOLIZER_PATH="$MLIR_SYS_200_PREFIX/llvm-symbolizer"

cargo +nightly test --target=aarch64-apple-darwin $@
