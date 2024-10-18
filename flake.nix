{
  description = "A very basic flake";

  inputs.nixpkgs.url = "flake:nixpkgs";
  inputs.flake-utils.url = "github:numtide/flake-utils";
  inputs.flake-parts.url = "github:hercules-ci/flake-parts";

  outputs = { self, ... } @ inputs:
    inputs.flake-parts.lib.mkFlake { inherit inputs; } {
      systems = [
        "x86_64-darwin"
        "x86_64-linux"
        "aarch64-darwin"
        "aarch64-linux"
      ];

      perSystem = { pkgs, ... }: {
        devShells.default = pkgs.mkShell {
        packages = with pkgs; [
          cargo
          cargo-audit
          cargo-bloat
          cargo-outdated
          cargo-nextest
          clippy
          rustc
          rust-analyzer
          libiconv
          rustfmt
          socat
          openssl
        ] ++ pkgs.lib.optionals pkgs.stdenv.isDarwin [
          darwin.apple_sdk.frameworks.CoreFoundation
          darwin.apple_sdk.frameworks.CoreServices
          darwin.apple_sdk.frameworks.IOKit
        ];
        };
      };
    };
}
