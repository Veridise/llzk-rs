#!/bin/sh 

export RUSTFLAGS="$RUSTFLAGS -Zsanitizer=address"
export ASAN_OPTIONS="debug=true:symbolize=1"
export LSAN_OPTIONS=""

supp=.lsan.supp
root=$(cargo locate-project --workspace --message-format plain | xargs dirname)
if [ -e "$root/$supp" ]; then 
  export LSAN_OPTIONS="$LSAN_OPTIONS:suppressions=$root/$supp"
fi

if [ $(uname) == "Darwin" ]; then 
  export ASAN_OPTIONS="$ASAN_OPTIONS:detect_leaks=1"
  export RUSTFLAGS="$RUSTFLAGS -Zexternal-clangrt -L $MLIR_SYS_200_PREFIX/lib/clang/20/lib/darwin/ -l clang_rt.asan_osx_dynamic"
fi
 
export ASAN_SYMBOLIZER_PATH="$MLIR_SYS_200_PREFIX/bin/llvm-symbolizer"

cargo +nightly test --target=aarch64-apple-darwin $@
