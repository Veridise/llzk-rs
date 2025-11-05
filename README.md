# Rust bindings for LLZK.

⚠️  These crates are under active development and things may change unexpectedly.

Rust bindings for [LLZK](https://veridise.github.io/llzk-lib/) over its C API. 
The bindings' API is meant to be more user friendly than the raw C API and more idiomatic. Its design is heavily inspired 
by [melior](https://github.com/mlir-rs/melior) and depends on it for handling the MLIR parts that are not 
specific to LLZK.

The primary supported use case of these bindings is creating IR and running passes on it. Support for other things, 
such as writing custom passes, is limited and not as ergonomic as it is in C++.

## Usage 

Run `cargo doc -p llzk` to generate the API documentation of the rust API and you can visit  
[LLZK's documentation](https://veridise.github.io/llzk-lib/) for more information about the IR itself.
For the high-level usage of the bindings you can check the examples in `llzk/examples`.

### Optional features 

We include some optional functionality guarded by feature flags. We currently have the following features:

- `bigint`: Allows creating constant values from [`num-bigint`'s Big integers](https://docs.rs/num-bigint/latest/num_bigint/struct.BigUint.html).

## Manual installation (pre v1 release)

Install LLVM 20 and note the installation path. While building your project the build scripts will look for LLVM using `llvm-config`.
If you don't have that tool in your `PATH` or it doesn't point to an LLVM 20 installation set the following environment variables 
to the path where LLVM is installed and the build scripts will then use `$MLIR_SYS_200_PREFIX/bin/llvm-config` instead.

```text
export MLIR_SYS_200_PREFIX=/path/to/llvm/20/
export TABLEGEN_200_PREFIX=/path/to/llvm/20/
```

In your rust project, add the crates to your Cargo.toml:

```text
llzk-sys = { git = "https://github.com/Veridise/llzk-rs" }
llzk = { git = "https://github.com/Veridise/llzk-rs" }
```

### Building tips

If you are using homebrew in macos you can access MLIR 20 by installing `llvm@20` with homebrew.
Setting the following environment variables configures the build system with the correct versions of MLIR and its dependencies. 
Depending on the version of your default C++ compiler you may need to set `CXX` and `CC` to a compiler that supports C++ 20.

```text
export MLIR_SYS_200_PREFIX=/opt/homebrew/opt/llvm@20/
export TABLEGEN_200_PREFIX=/opt/homebrew/opt/llvm@20/
export CXX=clang++
export CC=clang
export RUSTFLAGS='-L /opt/homebrew/lib/'
```

See [`llzk-sys`'s README](llzk-sys/README.md) for more details on setting up the build environment.

If working on LLZK via the submodule you can enable dumping the compile commands when building with cargo. Assuming the current directory is where your editor will look for the compile commands you can link them setting the `LLZK_EMIT_COMPILE_COMMANDS` environment variable as follows.

```text
LLZK_EMIT_COMPILE_COMMANDS=$(pwd) cargo build
```

## Nix installation 

We also include a nix flake that creates an environment with the right version of LLVM and MLIR. If you are already using nix this may be your prefered method.

You can use this flake for configuring your development environment.
For example, to work within a nix developer shell you can use the following command.

```text
nix develop 'github:Veridise/llzk-rs?submodules=1#llzk-rs'
```

Another alternative is to use [direnv](https://direnv.net/) with the following `.envrc` to automatically enter 
the developer environment when you enter your project's directory.

```text
if ! has nix_direnv_version || ! nix_direnv_version 3.0.4; then
  source_url "https://raw.githubusercontent.com/nix-community/nix-direnv/3.0.4/direnvrc" "sha256-DzlYZ33mWF/Gs8DDeyjr8mnVmQGx7ASYqA5WlxwvBG4="
fi

use flake 'github:Veridise/llzk-rs?submodules=1#llzk-rs'
```
