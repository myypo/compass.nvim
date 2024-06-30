{
  inputs = {
    nixpkgs.url = "nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";

    neovim-nightly-overlay.url = "github:nix-community/neovim-nightly-overlay";
    neovim-nightly-overlay.inputs.nixpkgs.follows = "nixpkgs";

    fenix.url = "github:nix-community/fenix";
    fenix.inputs.nixpkgs.follows = "nixpkgs";
  };

  outputs = inputs:
    with inputs;
      flake-utils.lib.eachDefaultSystem (
        system: let
          overlays = [fenix.overlays.default neovim-nightly-overlay.overlays.default];
          pkgs = import nixpkgs {
            inherit overlays system;
          };
        in {
          packages = with pkgs; {
            default = rustPlatform.buildRustPackage {
              name = "compass";
              src = lib.cleanSource ./.;
              cargoLock = {
                lockFile = ./Cargo.lock;
                allowBuiltinFetchGit = true;
              };

              doCheck = false;

              nativeBuildInputs = [
                pkg-config
                rustPlatform.bindgenHook
              ];
            };
          };

          devShells = with pkgs; {
            default =
              mkShell.override {
                stdenv = stdenvAdapters.useMoldLinker clangStdenv;
              }
              mkShell {
                packages = [
                  openssl
                  pkg-config

                  neovim

                  rust-analyzer-nightly

                  rustPlatform.bindgenHook

                  (fenix.complete.withComponents [
                    "cargo"
                    "clippy"
                    "rust-src"
                    "rust-std"
                    "rustc"
                    "rustfmt"
                  ])

                  cargo-watch
                ];
              };
          };
        }
      );
}
