# Simple test configuration - just testing Inventurly without the module
# Run: nixos-rebuild build-vm -I nixos-config=./test-simple-config.nix

{ config, pkgs, ... }:

let
  # Build Inventurly package
  inventurly = import ./default.nix { features = ["mock_auth"]; };
in
{
  # Basic system
  boot.loader.systemd-boot.enable = true;
  boot.loader.efi.canTouchEfiVariables = true;

  # Simple service without the complex module
  systemd.services.inventurly-simple = {
    description = "Inventurly Service (Simple Test)";
    wantedBy = [ "multi-user.target" ];
    after = [ "network.target" ];
    
    environment = {
      DATABASE_URL = "sqlite:/tmp/inventurly.db";
      SERVER_ADDRESS = "0.0.0.0:3000";
      RUST_LOG = "debug";
    };
    
    serviceConfig = {
      Type = "simple";
      ExecStart = "${inventurly}/bin/inventurly";
      WorkingDirectory = "/tmp";
      Restart = "on-failure";
    };
    
    preStart = ''
      # Create database
      ${pkgs.sqlite}/bin/sqlite3 /tmp/inventurly.db "VACUUM;" || true
      
      # Copy migrations if they exist
      if [ -d ${inventurly}/migrations ]; then
        cp -r ${inventurly}/migrations /tmp/ || true
        cd /tmp
        ${pkgs.sqlx-cli}/bin/sqlx database setup --source ./migrations/sqlite || true
      fi
    '';
  };

  # Networking
  networking.hostName = "inventurly-test";
  networking.firewall.enable = false;

  # Test user
  users.users.root.password = "test";
  services.getty.autologinUser = "root";

  # Tools
  environment.systemPackages = with pkgs; [ curl sqlite jq ];

  system.stateVersion = "24.05";
}