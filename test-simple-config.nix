# Test configuration for the Genossi NixOS module
# Run: nixos-rebuild build-vm -I nixos-config=./test-simple-config.nix
# Or use: ./run-vm.sh

{ config, pkgs, ... }:

{
  # Import the actual module to test it!
  imports = [ ./module.nix ];

  # Basic system
  boot.loader.systemd-boot.enable = true;
  boot.loader.efi.canTouchEfiVariables = true;

  # Test the module with a simple instance
  services.genossi.test = {
    enable = true;
    port = 3000;
    host = "127.0.0.1";
    #package = import ./default.nix { features = ["mock_auth"]; };
    logLevel = "debug";
    domain = "genossi-test.local";
    enableSSL = false; # Disable SSL for local testing
  };

  # Networking
  networking.hostName = "genossi-test";
  networking.firewall.enable = false;

  # Test user
  users.users.root.password = "test";
  services.getty.autologinUser = "root";

  # Tools
  environment.systemPackages = with pkgs; [ curl sqlite jq ];

  system.stateVersion = "24.05";
}