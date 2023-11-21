with import <nixpkgs> {};

let
  fenix = import (fetchTarball "https://github.com/nix-community/fenix/archive/main.tar.gz") { };
in

mkShell {
  name = "multipars-shell";

  buildInputs = [
    cargo-asm
    cargo-criterion
    cargo-flamegraph
    clippy
    fenix.minimal.toolchain
    gnuplot
    (python3.withPackages (p: with p; [
      autopep8
      sympy
    ]))
    rust-analyzer
    rustfmt
  ];

  RUST_SRC_PATH = "${pkgs.rust.packages.stable.rustPlatform.rustLibSrc}";
}
