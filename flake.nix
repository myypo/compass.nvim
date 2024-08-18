{
  inputs = {
    nixpkgs.url = "nixpkgs/nixos-unstable";

    pre-commit-hooks.url = "github:cachix/pre-commit-hooks.nix";

    neovim-nightly-overlay.url = "github:nix-community/neovim-nightly-overlay";
    neovim-nightly-overlay.inputs.nixpkgs.follows = "nixpkgs";

    fenix.url = "github:nix-community/fenix";
    fenix.inputs.nixpkgs.follows = "nixpkgs";
  };

  outputs = inputs:
    with inputs; let
      forEachSupportedSystem = let
        supportedSystems = ["x86_64-linux" "aarch64-linux" "x86_64-darwin" "aarch64-darwin"];
      in (
        f:
          nixpkgs.lib.genAttrs supportedSystems (
            system:
              f
              (let
                overlays = [fenix.overlays.default neovim-nightly-overlay.overlays.default];
              in
                import nixpkgs {inherit overlays system;})
          )
      );
    in {
      checks.pre-commit-check = forEachSupportedSystem (pkgs:
        inputs.pre-commit-hooks.lib.${pkgs.system}.run {
          src = ./.;
          hooks = {
            alejandra.enable = true;

            rustfmt = {
              enable = true;
              packageOverrides.rustfmt = pkgs.fenix.complete.rustfmt;
            };
            clippy = {
              enable = true;
              packageOverrides.cargo = pkgs.fenix.complete.cargo;
              packageOverrides.clippy = pkgs.fenix.complete.clippy;
              settings.allFeatures = true;
              settings.denyWarnings = true;
            };
          };
        });

      packages = forEachSupportedSystem (pkgs:
        with pkgs; {
          default = rustPlatform.buildRustPackage {
            name = "compass";
            src = lib.cleanSource ./.;
            cargoLock = {
              lockFile = ./Cargo.lock;
            };

            doCheck = false;

            nativeBuildInputs = [
              pkg-config
              rustPlatform.bindgenHook
            ];
          };
        });

      devShells = forEachSupportedSystem (pkgs:
        with pkgs; {
          default =
            mkShell.override {
              stdenv = stdenvAdapters.useMoldLinker clangStdenv;
            }
            {
              inherit (self.checks.pre-commit-check.${pkgs.system}) shellHook;

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
        });
    };
}
