{
  description = "Inventurly - Inventory Management System";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";

    # Frontend als Sub-Flake
    inventurly-frontend = {
      url = "path:./inventurly-frontend";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, flake-utils, inventurly-frontend }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
        frontendPkg = inventurly-frontend.packages.${system}.default;
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
            # Weitere Tools die du brauchst
          ];
        };
      }
    ) // {
      # NixOS Module (system-unabhängig)
      nixosModules.default = import ./module.nix;
    };
}