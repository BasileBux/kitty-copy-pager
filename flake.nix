{
  description = "Kitty pager (Rust)";
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };
  outputs =
    {
      self,
      nixpkgs,
      flake-utils,
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
        kitty-copy-pager = pkgs.rustPlatform.buildRustPackage {
          pname = "kitty-copy-pager";
          version = "0.1.0";
          src = ./.;
          cargoLock.lockFile = ./Cargo.lock;
          postInstall = ''
            strip $out/bin/kitty-copy-pager
          '';
        };
      in
      {
        packages.default = kitty-copy-pager;
        apps.default = flake-utils.lib.mkApp { drv = kitty-copy-pager; };
      }
    );
}
