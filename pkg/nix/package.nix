# SPDX-FileCopyrightText: 2026 Noyalib
# SPDX-License-Identifier: MIT OR Apache-2.0
#
# Stand-alone nixpkgs derivation for noyalib. This file is the
# eventual target for inclusion under
# `pkgs/development/tools/noyalib/default.nix` in the
# `NixOS/nixpkgs` repository.
#
# Until that PR lands, this derivation can be consumed directly:
#
#   nix-build pkg/nix/package.nix
#   ./result/bin/noyafmt --version

{ lib
, rustPlatform
, fetchFromGitHub
, installShellFiles
, asciidoctor
, stdenv
}:

rustPlatform.buildRustPackage rec {
  pname = "noyalib";
  version = "0.0.1";   # bumped by release pipeline

  src = fetchFromGitHub {
    owner  = "sebastienrousseau";
    repo   = "noyalib";
    rev    = "v${version}";
    sha256 = lib.fakeSha256;   # rewritten on every release bump
  };

  cargoLock.lockFile = ../../Cargo.lock;
  buildAndTestSubdir = "crates/noya-cli";
  cargoBuildFlags = [ "--bin" "noyafmt" "--bin" "noyavalidate" ];

  nativeBuildInputs = [ installShellFiles asciidoctor ];

  NOYA_GEN_ASSETS = "1";

  postInstall = ''
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

  meta = with lib; {
    description = "Pure-Rust YAML 1.2 parser, formatter, and validator (zero unsafe)";
    homepage    = "https://github.com/sebastienrousseau/noyalib";
    license     = [ licenses.mit licenses.asl20 ];
    maintainers = [ ];   # populated when noyalib lands in nixpkgs upstream
    platforms   = platforms.unix ++ platforms.windows;
  };
}
