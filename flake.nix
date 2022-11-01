{
  description = "A very basic flake";

  inputs.nixpkgs.url = "flake:nixpkgs";
  inputs.flake-utils.url = "github:numtide/flake-utils";

  outputs = { self, nixpkgs, flake-utils }: flake-utils.lib.eachDefaultSystem (system:
    let
      pkgs = nixpkgs.legacyPackages.${system};
    in {
      inherit self nixpkgs;

      devShell = pkgs.mkShell {
        buildInputs = [
          pkgs.cargo
          pkgs.cargo-bloat
          pkgs.cargo-outdated
          pkgs.clippy
          pkgs.rustc
          pkgs.rust-analyzer
          pkgs.libiconv
          pkgs.rustfmt
          pkgs.socat
          pkgs.openssl
        ] ++ pkgs.lib.optionals pkgs.stdenv.isDarwin [
          pkgs.darwin.apple_sdk.frameworks.CoreFoundation
          pkgs.darwin.apple_sdk.frameworks.CoreServices
        ];
      };
    });
}
