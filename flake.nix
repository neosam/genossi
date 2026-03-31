{
  description = "Genossi - Inventory Management System";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    openspec.url = "github:Fission-AI/OpenSpec";

    # Frontend als Sub-Flake
    genossi-frontend = {
      url = "path:./genossi-frontend";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, flake-utils, genossi-frontend, openspec }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
        frontendPkg = genossi-frontend.packages.${system}.default;
      in
      {
        packages = {
          # Backend mit mock_auth (default)
          default = pkgs.callPackage ./default.nix { 
            inherit pkgs;
            features = [ "mock_auth" ];
          };
          
          # Backend mit mock_auth
          backend-mock = pkgs.callPackage ./default.nix {
            inherit pkgs;
            features = [ "mock_auth" ];
          };
          
          # Backend mit OIDC
          backend-oidc = pkgs.callPackage ./default.nix {
            inherit pkgs;
            features = [ "oidc" ];
          };
          
          # Frontend
          frontend = frontendPkg;
        };

        # Development shell
        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            rustc
            cargo
            rust-analyzer
            sqlx-cli
            sqlite
            nodejs
            pkg-config
            openspec.packages.${system}.default
            # Weitere Tools die du brauchst
          ];
        };
      }
    ) // {
      # NixOS Module (system-unabhängig)
      nixosModules.default = import ./module.nix;
    };
}
