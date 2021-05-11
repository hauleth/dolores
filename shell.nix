{ pkgs ? import <nixpkgs> {}, ... }:

with pkgs;
with pkgs.beam.packages.erlangR23;

mkShell {
  buildInputs = [
    cargo
    cargo-bloat
    cargo-outdated
    rustc
    libiconv
    rustfmt
    socat
  ] ++ stdenv.lib.optionals stdenv.isDarwin [
    darwin.apple_sdk.frameworks.CoreFoundation
    darwin.apple_sdk.frameworks.CoreServices
  ];
}
