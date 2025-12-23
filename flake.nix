{
  inputs = {
    llzk-pkgs.url = "github:Veridise/llzk-nix-pkgs";
    nixpkgs.follows = "llzk-pkgs/nixpkgs";
    flake-utils.follows = "llzk-pkgs/flake-utils";
    llzk-lib = {
      url = "github:Veridise/llzk-lib";
      inputs = {
        nixpkgs.follows = "llzk-pkgs/nixpkgs";
        flake-utils.follows = "llzk-pkgs/flake-utils";
        llzk-pkgs.follows = "llzk-pkgs";
      };
    };
    release-helpers.follows = "llzk-lib/release-helpers";
  };

  # Custom colored bash prompt
  nixConfig.bash-prompt = "\\[\\e[0;32m\\][llzk-rs]\\[\\e[m\\] \\[\\e[38;5;244m\\]\\w\\[\\e[m\\] % ";

  outputs =
    {
      self,
      nixpkgs,
      flake-utils,
      release-helpers,
      llzk-pkgs,
      llzk-lib,
    }:
    {
      # Overlay for downstream consumption
      overlays.default =
        final: prev:
        let
          # Assert version match between LLVM and MLIR
          mlirVersion = final.llzk-llvmPackages.mlir.version;
          _ =
            assert final.llzk-llvmPackages.libllvm.version == mlirVersion;
            null;

          # Create a merged LLVM + MLIR derivation so tools that use llvm-config (like mlir-sys)
          # can correctly discover information about both LLVM and MLIR libraries.
          mlir-with-llvm = final.symlinkJoin {
            name = "mlir-with-llvm-${mlirVersion}";
            paths = [
              final.llzk-llvmPackages.libllvm.dev
              final.llzk-llvmPackages.libllvm.lib
              final.llzk-llvmPackages.mlir.dev
              final.llzk-llvmPackages.mlir.lib
            ];
            nativeBuildInputs = final.lib.optionals final.stdenv.isDarwin [
              final.rcodesign
            ];
            postBuild = ''
              out="${placeholder "out"}"
              llvm_config="$out/bin/llvm-config"
              llvm_config_original="$out/bin/llvm-config-native"

              echo "Creating merged package: $out"

              # Move the original `llvm-config` to a new name so we can replace it with a wrapper script.
              # On Darwin, a straightforward `mv` will leave the binary unusable due to improper code
              # signing, so we use `cp -L` to copy the symlinked file to a new file and then delete the
              # original and sign the new file in place.
              cp -L "$llvm_config" "$llvm_config_original"
              rm "$llvm_config"
              ${final.lib.optionalString final.stdenv.isDarwin ''
                chmod +w "$llvm_config_original"
                rcodesign sign "$llvm_config_original"
              ''}

              # Create a wrapper script for `llvm-config` that adds MLIR support to the original tool.
              substitute ${./nix/llvm-config.sh.in} "$llvm_config" \
                --subst-var-by out "$out" \
                --subst-var-by originalTool "$llvm_config_original"
              chmod +x "$llvm_config"

              # Replace the MLIR dynamic library from the LLVM build with a dummy static library
              # to avoid duplicate symbol issues when linking with both LLVM and MLIR since the
              # MLIR build generated individual static libraries for each component.
              rm -f "$out/lib/libMLIR.${if final.stdenv.isDarwin then "dylib" else "so"}"
              ${final.stdenv.cc}/bin/ar -r "$out/lib/libMLIR.a"
            '';
          };

          # LLZK shared environment configuration
          llzkSharedEnvironment = {
            nativeBuildInputs = with final; [
              cmake
              rustc
              cargo
              llzk-llvmPackages.clang
            ];

            buildInputs = with final; [
              libxml2
              zlib
              zstd
              z3.lib
              llzk-llvmPackages.libclang.dev
            ];

            devBuildInputs =
              with final;
              [
                git
                rustfmt
                rustPackages.clippy
              ]
              ++ llzkSharedEnvironment.buildInputs;

            # Shared environment variables
            env = {
              CC = "clang";
              CXX = "clang++";
              MLIR_SYS_200_PREFIX = "${mlir-with-llvm}";
              TABLEGEN_200_PREFIX = "${mlir-with-llvm}";
              LIBCLANG_PATH = "${final.llzk-llvmPackages.libclang.lib}/lib";
              RUST_BACKTRACE = "1";
            };

            # Shared settings for packages
            pkgSettings = {
              RUSTFLAGS = "-lLLVM -L ${mlir-with-llvm}/lib";
              # For release packages, fix _FORTIFY_SOURCE warning on Linux
              # by ensuring build dependencies are optimized.
              CARGO_PROFILE_RELEASE_BUILD_OVERRIDE_OPT_LEVEL = "2";
              # Fix for GNU-like linkers on Linux to avoid removing symbols
              LLZK_SYS_ENABLE_WHOLE_ARCHIVE = "1";
              # Speed up cargo rebuilds
              CARGO_INCREMENTAL = "1";
            };

            # Shared settings for dev shells
            devSettings = {
              RUSTFLAGS = "-L ${mlir-with-llvm}/lib";
              RUST_SRC_PATH = final.rustPlatform.rustLibSrc;
              # Fix _FORTIFY_SOURCE warning on Linux. The same approach used in `pkgSettings` did not work
              # in the dev shell for some reason. In this case, just disable _FORTIFY_SOURCE altogether.
              NIX_CFLAGS_COMPILE = " -U_FORTIFY_SOURCE -D_FORTIFY_SOURCE=0";
            };
          };

          # Helper function for building LLZK Rust packages
          buildLlzkRustPackage =
            packageName:
            final.rustPlatform.buildRustPackage (
              rec {
                pname = "${packageName}-rs";
                version = (final.lib.importTOML (./. + "/${packageName}/Cargo.toml")).package.version;
                # Note: for this source to include the `llzk-lib` submodule, the nix command line
                # must use `.?submodules=1`. For example, `nix build '.?submodules=1#llzk-rs'`.
                src = ./.;

                nativeBuildInputs = final.llzkSharedEnvironment.nativeBuildInputs;
                buildInputs = final.llzkSharedEnvironment.buildInputs;

                cargoLock = {
                  lockFile = ./Cargo.lock;
                  allowBuiltinFetchGit = true;
                };

                cargoBuildFlags = [
                  "--package"
                  packageName
                ];
                cargoTestFlags = [
                  "--package"
                  packageName
                ];
              }
              // final.llzkSharedEnvironment.env
              // final.llzkSharedEnvironment.pkgSettings
            );
        in
        {
          inherit mlir-with-llvm llzkSharedEnvironment;

          # LLZK Rust packages
          llzk-sys-rs = buildLlzkRustPackage "llzk-sys";
          llzk-rs = buildLlzkRustPackage "llzk";
        };
    }
    // flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [
            self.overlays.default
            llzk-pkgs.overlays.default
            llzk-lib.overlays.default
            release-helpers.overlays.default
          ];
        };
      in
      {
        packages = flake-utils.lib.flattenTree {
          # Copy the packages from imported overlays.
          inherit (pkgs) llzk llzk-debug;
          inherit (pkgs) mlir mlir-debug;
          inherit (pkgs) changelogCreator;
          # Prevent use of libllvm and llvm from nixpkgs, which will have
          # different versions than the mlir from llzk-pkgs.
          inherit (pkgs.llzk-llvmPackages) libllvm llvm;
          # Add new packages created here
          inherit (pkgs) mlir-with-llvm llzk-rs llzk-sys-rs;
          default = pkgs.llzk-rs;
        };

        devShells = flake-utils.lib.flattenTree {
          default = pkgs.mkShell (
            {
              nativeBuildInputs = pkgs.llzkSharedEnvironment.nativeBuildInputs;
              buildInputs = pkgs.llzkSharedEnvironment.devBuildInputs;
            }
            // pkgs.llzkSharedEnvironment.env
            // pkgs.llzkSharedEnvironment.devSettings
          );
        };
      }
    );
}
