# Test configuration for OIDC-enabled Genossi
{ config, lib, pkgs, ... }:

{
  imports = [ ./module.nix ];

  # Test OIDC configuration
  services.genossi.test-oidc = {
    enable = true;
    domain = "genossi.test.local";
    port = 3000;
    host = "127.0.0.1";
    
    oidc = {
      enable = true;
      issuer = "https://accounts.google.com";
      clientId = "test-client-id";
      clientSecretFile = /etc/genossi/client_secret;
      # appUrl will be auto-derived from domain as "https://genossi.test.local"
    };
    
    extraEnvironment = {
      RUST_LOG = "genossi=debug,tower_http=debug";
    };
  };

  # Test mock auth configuration (legacy compatibility)
  services.genossi.test-mock = {
    enable = true;
    domain = "genossi-mock.test.local";
    port = 3001;
    host = "127.0.0.1";
    
    oidc.enable = false; # Explicitly disable OIDC for mock auth
  };
}