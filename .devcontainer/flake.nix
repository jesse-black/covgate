{
  description = "Devcontainer Home Manager configuration";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    home-manager = {
      url = "github:nix-community/home-manager";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { nixpkgs, home-manager, ... }:
    let
      system = "x86_64-linux";
      pkgs = import nixpkgs {
        inherit system;
      };
      localModule = ./. + "/local.nix";
      localModules =
        if builtins.pathExists localModule
        then [ localModule ]
        else [ ];
    in {
      homeConfigurations.vscode = home-manager.lib.homeManagerConfiguration {
        inherit pkgs;
        modules = [
          ./home.nix
          ({ pkgs, ... }: {
            home.packages = with pkgs; [
              # Core CLI tools
              curl
              git
              jq
              yq-go
              ripgrep
              fd
              eza
              gh
              zip
              unzip
              file
              which
              less
              tree
              python3

              # Shell/script tooling
              shellcheck
              shfmt

              # Native fixture/build support
              gnumake
              pkg-config
              cmake
              ninja
              clang
              llvmPackages.llvm

              # Rust tooling
              rustup
              cargo-llvm-cov
              cargo-deny
              cargo-machete
              cargo-binstall
            ];
          })
        ] ++ localModules;
      };
    };
}
