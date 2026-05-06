# SPDX-FileCopyrightText: 2026 Noyalib
# SPDX-License-Identifier: MIT OR Apache-2.0
#
# Nix flake for noyalib.
#
# Usage:
#   nix run github:sebastienrousseau/noyalib#noyafmt -- --check ci/*.yaml
#   nix run github:sebastienrousseau/noyalib#noyavalidate -- in.yaml
#   nix develop                                   # dev shell with rust 1.75 + xtask deps
#   nix build .#default                           # produces ./result/bin/{noyafmt,noyavalidate}
#
# After the GitHub Release lands, this same flake gets adapted into
# a `pkgs/development/tools/noyalib/default.nix` for upstream
# nixpkgs submission.

{
  description = "noyalib — pure-Rust YAML 1.2 with serde, zero unsafe";

  inputs = {
    nixpkgs.url      = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url  = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ rust-overlay.overlays.default ];
        };
        rustToolchain = pkgs.rust-bin.stable."1.75.0".default.override {
          extensions = [ "rust-src" ];
        };
      in {
        packages = rec {
          default = pkgs.rustPlatform.buildRustPackage {
            pname = "noyalib";
            version = "0.0.1";   # bumped by `release-binaries.yml`
            src = ../..;
            cargoLock = {
              lockFile = ../../Cargo.lock;
            };
            buildAndTestSubdir = "crates/noya-cli";
            cargoBuildFlags = [ "--bin" "noyafmt" "--bin" "noyavalidate" ];

            nativeBuildInputs = with pkgs; [ installShellFiles asciidoctor ];

            # Trigger build.rs codegen so man pages and completions
            # land at predictable OUT_DIR paths the postInstall hook
            # can pick up.
            NOYA_GEN_ASSETS = "1";

            postInstall = ''
              # Man pages emitted by build.rs land under
              # target/release/build/noya-cli-*/out/.
              local outdir
              outdir=$(find target/release/build -type d -name 'out' -path '*noya-cli*' | head -1)
              installManPage "$outdir/noyafmt.1" "$outdir/noyavalidate.1"
              installShellCompletion --bash \
                  --name noyafmt      "$outdir/noyafmt.bash" \
                  --name noyavalidate "$outdir/noyavalidate.bash"
              installShellCompletion --fish \
                  --name noyafmt.fish      "$outdir/noyafmt.fish" \
                  --name noyavalidate.fish "$outdir/noyavalidate.fish"
              installShellCompletion --zsh \
                  --name _noyafmt      "$outdir/_noyafmt" \
                  --name _noyavalidate "$outdir/_noyavalidate"
            '';

            meta = with pkgs.lib; {
              description = "Pure-Rust YAML 1.2 parser, formatter, and validator";
              homepage    = "https://github.com/sebastienrousseau/noyalib";
              license     = with licenses; [ mit asl20 ];
              maintainers = [ ];   # populated when noyalib lands in nixpkgs upstream
              platforms   = platforms.unix ++ platforms.windows;
            };
          };
          # Convenience aliases so `nix run .#noyafmt` works.
          noyafmt      = default;
          noyavalidate = default;
        };

        devShells.default = pkgs.mkShell {
          buildInputs = [
            rustToolchain
            pkgs.cargo-deb
            pkgs.cargo-generate-rpm
            pkgs.cargo-llvm-cov
            pkgs.asciidoctor
          ];
        };

        # `nix flake check` runs build + the workspace test suite.
        checks = {
          build = self.packages.${system}.default;
        };
      });
}
