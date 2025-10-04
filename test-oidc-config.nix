# Test configuration for OIDC-enabled Inventurly
{ config, lib, pkgs, ... }:

{
  imports = [ ./module.nix ];

  # Test OIDC configuration
  services.inventurly.test-oidc = {
    enable = true;
    domain = "inventurly.test.local";
    port = 3000;
    host = "127.0.0.1";
    
    oidc = {
      enable = true;
      issuer = "https://accounts.google.com";
      clientId = "test-client-id";
      clientSecretFile = /etc/inventurly/client_secret;
      # appUrl will be auto-derived from domain as "https://inventurly.test.local"
    };
    
    extraEnvironment = {
      RUST_LOG = "inventurly=debug,tower_http=debug";
    };
  };

  # Test mock auth configuration (legacy compatibility)
  services.inventurly.test-mock = {
    enable = true;
    domain = "inventurly-mock.test.local";
    port = 3001;
    host = "127.0.0.1";
    
    oidc.enable = false; # Explicitly disable OIDC for mock auth
  };
}