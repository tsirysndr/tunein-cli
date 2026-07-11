{
  description = "TuneIn CLI - Browse and listen to thousands of radio stations across the globe right from your terminal 🌎 📻 🎵✨";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

    crane.url = "github:ipetkov/crane";

    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.rust-analyzer-src.follows = "";
    };

    flake-utils.url = "github:numtide/flake-utils";

    advisory-db = {
      url = "github:rustsec/advisory-db";
      flake = false;
    };
  };

  outputs = { self, nixpkgs, crane, fenix, flake-utils, advisory-db, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
        };

        inherit (pkgs) lib;

        craneLib = crane.mkLib pkgs;

        protoFilter = path: _type: builtins.match ".*proto$" path != null;
        serviceFilter = path: _type: builtins.match ".*service$" path != null;
        protoOrCargo = path: type:
          (protoFilter path type) || (serviceFilter path type)
          || (craneLib.filterCargoSources path type);

        src = lib.cleanSourceWith {
          src = craneLib.path ./.; # The original, unfiltered source
          filter = protoOrCargo;
        };

        webuiNodeModules = pkgs.stdenv.mkDerivation {
          pname = "tunein-webui-node-modules";
          version = "0.7.1";

          src = lib.fileset.toSource {
            root = ./web;
            fileset = lib.fileset.unions [
              ./web/package.json
              ./web/bun.lock
            ];
          };

          nativeBuildInputs = [ pkgs.bun ];

          dontConfigure = true;

          buildPhase = ''
            runHook preBuild
            export HOME=$(mktemp -d)
            bun install --frozen-lockfile --no-progress
            runHook postBuild
          '';

          installPhase = ''
            runHook preInstall
            mv node_modules $out
            runHook postInstall
          '';

          dontFixup = true;

          outputHashMode = "recursive";
          outputHashAlgo = "sha256";
          outputHash = {
            x86_64-linux = "sha256-yeZqlyo/0FJkfPNNB4ri9aYcyciOZ0ACLvZELzb40Ng=";
            aarch64-linux = lib.fakeHash;
            aarch64-darwin = "sha256-6MAEYjk01JbCJC4m6q9rf2iePtxKMTClBB5YAuvubYA=";
            x86_64-darwin = lib.fakeHash;
          }.${system};
        };

        # Build the React SPA. The resulting `dist/` is embedded into the
        # tunein binary at compile time via rust-embed.
        webui = pkgs.stdenv.mkDerivation {
          pname = "tunein-webui";
          version = "0.7.1";

          src = ./web;

          nativeBuildInputs = [ pkgs.bun pkgs.nodejs ];

          configurePhase = ''
            runHook preConfigure
            cp -r ${webuiNodeModules} node_modules
            chmod -R u+w node_modules
            patchShebangs node_modules
            export HOME=$(mktemp -d)
            runHook postConfigure
          '';

          buildPhase = ''
            runHook preBuild
            bun run build
            runHook postBuild
          '';

          installPhase = ''
            runHook preInstall
            cp -r dist $out
            runHook postInstall
          '';
        };

        # Common arguments can be set here to avoid repeating them later
        commonArgs = {
          inherit src;

          pname = "tunein";
          version = "0.7.1";

          nativeBuildInputs = lib.optionals pkgs.stdenv.isDarwin [
            # coreaudio-sys generates its CoreAudio bindings with bindgen at
            # build time; bindgenHook provides libclang (LIBCLANG_PATH) and
            # points clang at the Nix Apple SDK headers.
            pkgs.rustPlatform.bindgenHook
          ];

          buildInputs = [
            # Add additional build inputs here
            pkgs.openssl
            pkgs.openssl.dev
            pkgs.pkg-config
            pkgs.gnumake
            pkgs.perl
            pkgs.protobuf
            pkgs.dbus
          ] ++ lib.optionals pkgs.stdenv.isLinux [
            pkgs.alsa-lib.dev
          ] ++ lib.optionals pkgs.stdenv.isDarwin [
            # Additional darwin specific inputs can be set here
            pkgs.libiconv
          ];

          # rust-embed reads web/dist at compile time. The source filter
          # strips the web directory, so we drop the pre-built SPA back
          # in before cargo runs.
          preBuild = ''
            mkdir -p web
            cp -r ${webui} web/dist
            chmod -R u+w web/dist
          '';

          # Additional environment variables can be set directly
          # MY_CUSTOM_VAR = "some value";
        };

        craneLibLLvmTools = craneLib.overrideToolchain
          (fenix.packages.${system}.complete.withComponents [
            "cargo"
            "llvm-tools"
            "rustc"
          ]);

        # Build *just* the cargo dependencies, so we can reuse
        # all of that work (e.g. via cachix) when running in CI
        cargoArtifacts = craneLib.buildDepsOnly commonArgs;

        # Build the actual crate itself, reusing the dependency
        # artifacts from above.
        tunein = craneLib.buildPackage (commonArgs // {
          inherit cargoArtifacts;
        });

      in
      {
        checks = {
          # Build the crate as part of `nix flake check` for convenience
          inherit tunein;

          # Run clippy (and deny all warnings) on the crate source,
          # again, resuing the dependency artifacts from above.
          #
          # Note that this is done as a separate derivation so that
          # we can block the CI if there are issues here, but not
          # prevent downstream consumers from building our crate by itself.
          tunein-clippy = craneLib.cargoClippy (commonArgs // {
            inherit cargoArtifacts;
            cargoClippyExtraArgs = "--all-targets -- --deny warnings";
          });

          tunein-doc = craneLib.cargoDoc (commonArgs // {
            inherit cargoArtifacts;
          });

          # Check formatting
          tunein-fmt = craneLib.cargoFmt {
            inherit src;
          };

          # Audit dependencies
          tunein-audit = craneLib.cargoAudit {
            inherit src advisory-db;
          };

          # Run tests with cargo-nextest
          # Consider setting `doCheck = false` on `tunein` if you do not want
          # the tests to run twice
          tunein-nextest = craneLib.cargoNextest (commonArgs // {
            inherit cargoArtifacts;
            partitions = 1;
            partitionType = "count";
          });
        } // lib.optionalAttrs (system == "x86_64-linux") {
          # NB: cargo-tarpaulin only supports x86_64 systems
          # Check code coverage (note: this will not upload coverage anywhere)
          tunein-coverage = craneLib.cargoTarpaulin (commonArgs // {
            inherit cargoArtifacts;
          });
        };

        packages = {
          default = tunein;
          inherit webui;
          tunein-llvm-coverage = craneLibLLvmTools.cargoLlvmCov (commonArgs // {
            inherit cargoArtifacts;
          });
        };

        apps.default = flake-utils.lib.mkApp {
          drv = tunein;
        };

        devShells.default = pkgs.mkShell {
          inputsFrom = builtins.attrValues self.checks.${system};

          # Additional dev-shell environment variables can be set directly
          # MY_CUSTOM_DEVELOPMENT_VAR = "something else";

          # Extra inputs can be added here
          nativeBuildInputs = with pkgs; [
            cargo
            rustc
          ];
        };
      });
}
