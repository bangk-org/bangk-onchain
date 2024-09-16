{
  description = "Bangk On-Chain programs";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";

    crane = {
      url = "github:ipetkov/crane";
    };

    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs = {
        nixpkgs.follows = "nixpkgs";
      };
    };

    solana = {
      url = "github:VincentBerthier/solana";
      inputs = {
        nixpkgs.follows = "nixpkgs";
        flake-utils.follows = "flake-utils";
        crane.follows = "crane";
      };
    };

    flake-utils.url = "github:numtide/flake-utils";

    advisory-db = {
      url = "github:rustsec/advisory-db";
      flake = false;
    };
  };

  outputs = {
    self,
    nixpkgs,
    crane,
    rust-overlay,
    flake-utils,
    advisory-db,
    solana,
    ...
  }:
    flake-utils.lib.eachDefaultSystem (system: let
      pkgs = import nixpkgs {
        inherit system;
        config.allowUnfree = true;
        overlays = [(import rust-overlay)];
      };
      rustOverlay = pkgs.rust-bin.stable."1.81.0".default.override {
        extensions = ["rust-analyzer" "rust-src" "rust-docs" "llvm-tools"];
      };
      # rustPretty = pkgs.rust-bin.stable."1.78.0".default; # llvm-cov-pretty doesn’t compile with a more recent version

      inherit (pkgs) lib;
      craneLib = (crane.mkLib pkgs).overrideToolchain rustOverlay;
      # craneLibPretty = (crane.mkLib pkgs).overrideToolchain rustPretty;

      src = lib.cleanSourceWith {
        src = craneLib.path ./.;
        filter = path: type:
          (lib.hasSuffix "\.dic" path) # just to get the spellcheck config
          || (craneLib.filterCargoSources path type);
      };

      # Common arguments can be set here to avoid repeating them later
      commonArgs = {
        inherit src;
        pname = "bangk-onchain";
        version = "0.1.0";
        strictDeps = true;

        buildInputs = with pkgs;
          [
            stdenv
            mold
          ]
          ++ lib.optionals pkgs.stdenv.isDarwin [
            # Additional darwin specific inputs can be set here
            pkgs.libiconv
          ];

        nativeBuildInputs = with pkgs; [
          mold
        ];

        # Additional environment variables can be set directly
      };

      # Build *just* the cargo dependencies, so we can reuse
      # all of that work (e.g. via cachix) when running in CI
      cargoArtifacts = craneLib.buildDepsOnly commonArgs;

      ######################################################
      ###               Coverage & Doc                   ###
      ######################################################
      bangk-docs = craneLib.cargoDoc (commonArgs
        // {
          pnameSuffix = "-docs";
          inherit cargoArtifacts;
        });

      llvm-cov-pretty = craneLib.buildPackage (commonArgs
        // {
          pname = "llvm-cov-pretty";
          version = "0.1.10";
          cargoArtifacts = null;

          src = pkgs.fetchFromGitHub {
            owner = "dnaka91";
            repo = "llvm-cov-pretty";
            rev = "v0.1.10";
            sha256 = "sha256-3QtDAQGVcqRDfjgl4Lq3Ue/6/yH61YPkM/JXdQJdoNo=";
            # sha256 = lib.fakeHash;
            fetchSubmodules = true;
          };

          cargoBuildCommand = "pnpm run build && cargo build --profile release";
          doCheck = true;
          cargoTestExtraArgs = "-- --skip version";
          nativeBuildInputs = commonArgs.nativeBuildInputs ++ [pkgs.tailwindcss pkgs.nodePackages_latest.pnpm];
        });

      bangk-coverage = craneLib.mkCargoDerivation (commonArgs
        // {
          inherit src;

          pnameSuffix = "-coverage";
          BANGK_MODE = "TESTING";
          cargoArtifacts = null;

          buildPhaseCargoCommand = ''
            cargo llvm-cov --no-report --locked --all-features nextest
          '';
          doInstallCargoArtifacts = false;
          installPhase = ''
            mkdir -p $out
            cargo llvm-cov report --doctests --ignore-filename-regex="(^\\\/nix\\\/store\\\/*|tests-utilities\\\/*|(entrypoint|errors).rs)" --json | ${llvm-cov-pretty}/bin/llvm-cov-pretty --theme dracula --output-dir $out
            cargo llvm-cov report --doctests --ignore-filename-regex="(^\\\/nix\\\/store\\\/*|tests-utilities\\\/*|(entrypoint|errors).rs)" --lcov --output-path $out/lcov.info
          '';
          nativeBuildInputs = commonArgs.nativeBuildInputs ++ [pkgs.cargo-llvm-cov pkgs.cargo-nextest];
        });

      ######################################################
      ###             On-chain programs                  ###
      ######################################################
      bangk-prg-testing = pkgs.stdenv.mkDerivation {
        inherit system src;

        name = "bangk-prg-testing";
        buildPhase = ''
          mkdir -p target && chmod -R 777 target/
          # ${pkgs.docker}/bin/docker run --rm -e "BANGK_MODE=TESTING" -v $(pwd):/mnt/code/ solana /bin/bash build-main
          ${pkgs.docker}/bin/docker run --rm -e "BANGK_MODE=TESTING" -v $(pwd):/mnt/code/ solana /bin/bash build-ico
        '';
        installPhase = ''
          mkdir $out
          # cp target/build-sbf/deploy/bangk.so $out/bangk-main-testing.so
          cp target/build-sbf/deploy/bangk_ico.so $out/bangk-ico-testing.so
        '';
      };

      bangk-prg-devnet = pkgs.stdenv.mkDerivation {
        inherit system src;

        name = "bangk-prg-devnet";
        buildPhase = ''
          mkdir -p target && chmod -R 777 target/
          # ${pkgs.docker}/bin/docker run --rm -e "BANGK_MODE=DEVNET" -v $(pwd):/mnt/code/ solana /bin/bash build-main
          ${pkgs.docker}/bin/docker run --rm -e "BANGK_MODE=DEVNET" -v $(pwd):/mnt/code/ solana /bin/bash build-ico
        '';
        installPhase = ''
          mkdir $out
          # cp target/build-sbf/deploy/bangk.so $out/bangk-main-devnet.so
          cp target/build-sbf/deploy/bangk_ico.so $out/bangk-ico-devnet.so
        '';
      };

      bangk-prg-mainnet = pkgs.stdenv.mkDerivation {
        inherit system src;

        name = "bangk-prg-mainnet";
        buildPhase = ''
          mkdir -p target && chmod -R 777 target/
          # ${pkgs.docker}/bin/docker run --rm -e "BANGK_MODE=MAINNET" -v $(pwd):/mnt/code/ solana /bin/bash build-main
          ${pkgs.docker}/bin/docker run --rm -e "BANGK_MODE=MAINNET" -v $(pwd):/mnt/code/ solana /bin/bash build-ico
        '';
        installPhase = ''
          mkdir $out
          # cp target/build-sbf/deploy/bangk.so $out/bangk-main-mainnet.so
          cp target/build-sbf/deploy/bangk_ico.so $out/bangk-ico-mainnet.so
        '';
      };

      bangk-prg-all = pkgs.stdenv.mkDerivation {
        inherit system src;

        name = "bangk-prg-all";
        installPhase = ''
          mkdir $out
          cp ${bangk-prg-testing}/*.so $out/
          cp ${bangk-prg-devnet}/*.so $out/
          cp ${bangk-prg-mainnet}/*.so $out/
        '';
      };

      alias-coverage = ''
        BANGK_MODE='TESTING' cargo llvm-cov nextest \
          --ignore-filename-regex='(entrypoint).rs' \
          --locked --all-features \
          --json | llvm-cov-pretty --theme dracula \
          && echo 'Rapport Code Coverage généré' | cowsay | lolcat \
      '';
      aliases = ''
        alias coverage=\"${alias-coverage}\" \
      '';
    in {
      checks = {
        # Build the crate as part of `nix flake check` for convenience
        inherit bangk-prg-testing;

        ######################################################
        ###               Nix flake checks                 ###
        ######################################################
        # Run clippy (and deny all warnings) on the crate source,
        # again, resuing the dependency artifacts from above.
        #
        # Note that this is done as a separate derivation so that
        # we can block the CI if there are issues here, but not
        # prevent downstream consumers from building our crate by itself.
        bangk-clippy = craneLib.cargoClippy (commonArgs
          // {
            inherit cargoArtifacts;
            pnameSuffix = "-clippy";

            cargoClippyExtraArgs = "--all-features --all-targets -- --deny warnings";
          });

        # Check formatting
        bangk-fmt = craneLib.cargoFmt (commonArgs
          // {
            pnameSuffix = "-fmt";
            inherit src;
          });

        # Audit dependencies
        bangk-audit = craneLib.cargoAudit (commonArgs
          // {
            pnameSuffix = "-audit";
            inherit src advisory-db;
          });

        # Audit licenses
        bangk-deny = craneLib.cargoDeny (commonArgs
          // {
            pnameSuffix = "-deny";
            inherit src;
          });

        # Check the spelling
        bangk-spellcheck = craneLib.mkCargoDerivation (commonArgs
          // {
            inherit src cargoArtifacts;

            pnameSuffix = "-spellcheck";
            buildPhaseCargoCommand = "HOME=./ cargo spellcheck check -m 1";
            nativeBuildInputs = (commonArgs.buildInputs or []) ++ [pkgs.cargo-spellcheck];
          });

        # Run tests with cargo-nextest
        # Consider setting `doCheck = false` on `bangk` if you do not want
        # the tests to run twice
        bangk-nextest = craneLib.cargoNextest (commonArgs
          // {
            pnameSuffix = "-tests";
            inherit cargoArtifacts;

            checkPhaseCargoCommand = "cargo nextest run";
            partitions = 1;
            partitionType = "count";
            BANGK_MODE = "TESTING";
          });
      };

      ######################################################
      ###                 Build packages                 ###
      ######################################################
      packages = {
        default = bangk-prg-testing;

        coverage = bangk-coverage;
        docs = bangk-docs;

        # Programs
        prg-testing = bangk-prg-testing;
        prg-devnet = bangk-prg-devnet;
        prg-mainnet = bangk-prg-mainnet;
        prg-all = bangk-prg-all;
      };

      ######################################################
      ###                   Dev’ shell                   ###
      ######################################################
      devShells.default = craneLib.devShell {
        name = "devshell";

        # Inherit inputs from checks.
        checks = self.checks.${system};

        # Additional dev-shell environment variables can be set directly
        # CARGO_BUILD_JOBS = 8;
        # LD_LIBRARY_PATH = "${pkgs.openssl.out}/lib;${pkgs.bzip2.out}/lib";
        PATH = "${pkgs.mold}/bin/mold";
        BANGK_MODE = "testing";

        shellHook = ''
          export PATH="$HOME/.cargo/bin:$PATH"
          echo "Environnement $(basename $(pwd)) chargé" | cowsay | lolcat

          exec $SHELL -C "${aliases}"
        '';

        # Extra inputs can be added here; cargo and rustc are provided by default.
        packages = with pkgs; [
          # Compilation
          mold # rust linker
          protobuf

          # Solana from flake
          solana.packages.${system}.default

          # Utils
          cowsay
          gitmoji-cli # Use gitmojis to commit
          lolcat
          tokei # file lines count

          # Cargo utilities
          cargo-bloat # check binaries size (which is fun but not terriby useful?)
          cargo-cache # cargo cache -a
          cargo-deny
          cargo-audit
          cargo-expand # for macro expension
          cargo-spellcheck # Spellcheck documentation
          # cargo-wizard
        ];
      };
    });
}
