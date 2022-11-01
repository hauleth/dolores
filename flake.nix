{
  description = "A very basic flake";

  inputs.nixpkgs.url = "flake:nixpkgs";
  inputs.flake-utils.url = "github:numtide/flake-utils";
  inputs.rust-overlay.url = "github:oxalica/rust-overlay";

  outputs = { self, nixpkgs, rust-overlay, flake-utils }: flake-utils.lib.eachDefaultSystem (system:
    let
      pkgs = import nixpkgs {
        inherit system;
        overlays = [
          rust-overlay.overlay
          (self: super: {
            # Because rust-overlay bundles multiple rust packages into one
            # derivation, specify that mega-bundle here, so that crate2nix
            # will use them automatically.
            rustc = self.rust-bin.stable.latest.default;
            cargo = self.rust-bin.stable.latest.default;
          })
        ];
      };

    in
    {
      inherit self nixpkgs;

      devShell = pkgs.mkShell {
        buildInputs = with pkgs; [
          cargo
          cargo-bloat
          cargo-outdated
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
    });
}
