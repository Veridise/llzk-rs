{
  inputs = {
    # TODO: After llzk-nix-pkgs is updated, drop "?ref=th/cleanup"
    llzk-pkgs.url = "github:Veridise/llzk-nix-pkgs?ref=th/cleanup";

    nixpkgs = {
      url = "github:NixOS/nixpkgs";
      follows = "llzk-pkgs/nixpkgs";
    };

    flake-utils = {
      url = "github:numtide/flake-utils/v1.0.0";
      follows = "llzk-pkgs/flake-utils";
    };

    llzk = {
      # TODO: After llzk-lib is updated, drop "?ref=th/update_llzk_nix_pkgs"
      url = "github:Veridise/llzk-lib?ref=th/update_llzk_nix_pkgs";
      inputs = {
        nixpkgs.follows = "llzk-pkgs/nixpkgs";
        flake-utils.follows = "llzk-pkgs/flake-utils";
        llzk-pkgs.follows = "llzk-pkgs";
      };
    };

    release-helpers.follows = "llzk/release-helpers";
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
      llzk,
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [
            release-helpers.overlays.default
            llzk-pkgs.overlays.default
            llzk.overlays.default
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
            rm -f "$out"/lib/libMLIR.dylib
            ${pkgs.stdenv.cc}/bin/ar -r "$out"/lib/libMLIR.a
          '';
        };
        commonNativeBuildInputs = (
          with pkgs;
          [
            cmake
            rustc
            cargo
          ]
        );
        commonBuildInputs = (
          with pkgs;
          [
            libxml2
            zlib
            zstd
          ]
        );

        llzk-sys = pkgs.rustPlatform.buildRustPackage rec {
          pname = "llzk-sys";
          version = (pkgs.lib.importTOML ./llzk-sys/Cargo.toml).package.version;

          nativeBuildInputs = commonNativeBuildInputs;
          buildInputs = commonBuildInputs;

          # Cannot just use the local source because the submodule would not be included.
          src = pkgs.fetchFromGitHub {
            owner = "Veridise";
            repo = "llzk-rs";
            rev = "ed67aee0cb901d60945af74947e7378794298220";
            hash = "sha256-WOFl7d5QzUwGBmUsgYI5r3f3oP1Ufe1EEcnpif/NrQc=";
            fetchSubmodules = true;
          };

          cargoLock = {
            lockFile = ./Cargo.lock;
            allowBuiltinFetchGit = true;
          };

          # Build only the llzk-sys package from the workspace
          cargoBuildFlags = [
            "--package"
            "llzk-sys"
          ];
          cargoTestFlags = [
            "--package"
            "llzk-sys"
          ];

          CC = "clang";
          CXX = "clang++";
          MLIR_SYS_200_PREFIX = "${mlir-with-llvm}";
          TABLEGEN_200_PREFIX = "${mlir-with-llvm}";
          RUSTFLAGS = "-L ${mlir-with-llvm}/lib/";
          RUST_BACKTRACE = 1;
        };
      in
      {
        packages = flake-utils.lib.flattenTree {
          # Copy the packages from the overlay.
          inherit (pkgs) llzk llzk-debug;
          inherit (pkgs) mlir mlir-debug;
          inherit (pkgs) changelogCreator;
          # Prevent use of libllvm and llvm from nixpkgs, which will have
          # different versions than the mlir from llzk-pkgs.
          inherit (pkgs.llzk-llvmPackages) libllvm llvm;
          # Add new packages created here
          inherit mlir-with-llvm llzk-sys;
          default = llzk-sys;
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
