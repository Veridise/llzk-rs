#!/bin/sh 

RUSTFLAGS="$RUSTFLAGS -Zsanitizer=address" \
  ASAN_OPTIONS="debug=true:detect_leaks=1:symbolize=1" \
  ASAN_SYMBOLIZER_PATH="$MLIR_SYS_200_PREFIX/llvm-symbolizer" \
  CFLAGS="-fsanitize=address" \
  CXXFLAGS="-fsanitize=address" \
  LDFLAGS="-fsanitize=address -static-libasan" \
  cargo +nightly test --target=aarch64-apple-darwin $@
