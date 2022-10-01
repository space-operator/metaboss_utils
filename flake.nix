{
  inputs = {
    nixpkgs = {
      url = "github:NixOS/nixpkgs/nixos-22.05";
    };
    flake-utils = {
      url = "github:numtide/flake-utils";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.flake-utils.follows = "flake-utils";
    };
    flake-compat = {
      url = "github:edolstra/flake-compat";
      flake = false;
    };
  };

  outputs = {
    self,
      nixpkgs,
      flake-utils,
      rust-overlay,
      ...
  }:
    flake-utils.lib.eachDefaultSystem (system: let
      overlays = [(import rust-overlay)];
      pkgs = import nixpkgs {inherit system overlays;};
      rust = pkgs.rust-bin.nightly.latest.default;
      platform = pkgs.makeRustPlatform {
        cargo = rust;
        rustc = rust;
      };
      shellInputs = with pkgs; [
        (rust.override {
          extensions = ["rust-src"];
          targets = ["wasm32-wasi" "wasm32-unknown-unknown"];
        })
      ];
      appBuildInputs = with pkgs; [
        openssl
        binutils-unwrapped
      ];
      appNativeBuildInputs = with pkgs; [
        pkg-config
      ];
    in {
      packages = {
      };
      devShell = pkgs.mkShell {
        buildInputs = shellInputs ++ appBuildInputs;
        nativeBuildInputs = appNativeBuildInputs;
        shellHook = ''export LD_LIBRARY_PATH="$LD_LIBRARY_PATH:${pkgs.lib.makeLibraryPath appBuildInputs}"'';
      };
    });
}
