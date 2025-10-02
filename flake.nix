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
      overlays.default = final: prev: {
        inherit (self.packages.${final.system}) mlir-with-llvm llzk-rs llzk-sys-rs;
      };
    }
    // flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [
            release-helpers.overlays.default
            llzk-pkgs.overlays.default
            llzk-lib.overlays.default
          ];
        };

        # Assert version match between LLVM and MLIR
        mlirVersion = pkgs.llzk-llvmPackages.mlir.version;
        _ =
          assert pkgs.llzk-llvmPackages.libllvm.version == mlirVersion;
          null;

        # Create a merged LLVM + MLIR derivation so tools that use llvm-config (like mlir-sys)
        # can correctly discover information about both LLVM and MLIR libraries.
        mlir-with-llvm = pkgs.symlinkJoin {
          name = "mlir-with-llvm-${mlirVersion}";
          paths = [
            pkgs.llzk-llvmPackages.libllvm.dev
            pkgs.llzk-llvmPackages.libllvm.lib
            pkgs.llzk-llvmPackages.mlir.dev
            pkgs.llzk-llvmPackages.mlir.lib
          ];
          postBuild = ''
            echo "Creating merged package: $out"
            mv "$out"/bin/llvm-config "$out"/bin/llvm-config-native
            substitute ${./nix/llvm-config.sh.in} "$out"/bin/llvm-config \
              --subst-var-by out "${placeholder "out"}" \
              --subst-var-by cmakeBuildType "Release" \
              --subst-var-by version "${mlirVersion}" 
            chmod +x "$out"/bin/llvm-config
            # Replace the MLIR dynamic library from the LLVM build with a dummy static library
            # to avoid duplicate symbol issues when linking with both LLVM and MLIR since the
            # MLIR build generated individual static libraries for each component.
            rm -f "$out"/lib/libMLIR.${if pkgs.stdenv.isDarwin then "dylib" else "so"}
            ${pkgs.stdenv.cc}/bin/ar -r "$out"/lib/libMLIR.a
          '';
        };

        commonNativeBuildInputs = (
          with pkgs;
          [
            cmake
            rustc
            cargo
            clang
          ]
        );

        commonBuildInputs = (
          with pkgs;
          [
            libxml2
            zlib
            zstd
            z3.lib
            mlir-with-llvm
          ]
        );

        # Helper function to build Rust packages with common configuration
        buildLlzkRustPackage =
          packageName:
          pkgs.rustPlatform.buildRustPackage rec {
            pname = "${packageName}-rs";
            version = (pkgs.lib.importTOML ./${packageName}/Cargo.toml).package.version;
            # Note: for this source to include the `llzk-lib` submodule, the nix command line
            # must use `.?submodules=1`. For example, `nix build '.?submodules=1#llzk-rs'`.
            src = ./.;

            nativeBuildInputs = commonNativeBuildInputs;
            buildInputs = commonBuildInputs;

            cargoLock = {
              lockFile = ./Cargo.lock;
              allowBuiltinFetchGit = true;
            };

            # Build only the specified package from the workspace
            cargoBuildFlags = [
              "--package"
              packageName
            ];
            cargoTestFlags = [
              "--package"
              packageName
            ];

            CC = "clang";
            CXX = "clang++";
            MLIR_SYS_200_PREFIX = "${mlir-with-llvm}";
            TABLEGEN_200_PREFIX = "${mlir-with-llvm}";
            LIBCLANG_PATH = "${pkgs.llzk-llvmPackages.libclang.lib}/lib";
            RUSTFLAGS = "-lLLVM -L ${mlir-with-llvm}/lib/ -lz3 -L ${pkgs.z3.lib}/lib";
            RUST_BACKTRACE = 1;
            # Fix _FORTIFY_SOURCE warning on Linux by ensuring build dependencies are optimized
            CARGO_PROFILE_RELEASE_BUILD_OVERRIDE_OPT_LEVEL = 2;
            # Fix for GNU-like linkers on Linux to avoid removing symbols
            LLZK_SYS_ENABLE_WHOLE_ARCHIVE = 1;
          };

        llzk-sys-rs = buildLlzkRustPackage "llzk-sys";
        llzk-rs = buildLlzkRustPackage "llzk";
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
          inherit mlir-with-llvm llzk-rs llzk-sys-rs;
          default = llzk-rs;
        };

        devShells = flake-utils.lib.flattenTree {
          default = pkgs.mkShell {
            nativeBuildInputs = commonNativeBuildInputs;
            buildInputs =
              (with pkgs; [
                git
                rustfmt
                rustPackages.clippy
              ])
              ++ commonBuildInputs;

            CC = "clang";
            CXX = "clang++";
            MLIR_SYS_200_PREFIX = "${mlir-with-llvm}";
            TABLEGEN_200_PREFIX = "${mlir-with-llvm}";
            RUSTFLAGS = "-L ${mlir-with-llvm}/lib/";
            RUST_SRC_PATH = pkgs.rustPlatform.rustLibSrc;
            CARGO_INCREMENTAL = 1; # speed up rebuilds
            RUST_BACKTRACE = 1; # enable backtraces
          };
        };
      }
    );
}
