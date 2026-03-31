# Example NixOS configuration showing different Genossi deployment options
# This file demonstrates how to use the module in production

{ config, pkgs, ... }:

{
  imports = [ ./module.nix ];

  # Example 1: Development instance (direct access, no reverse proxy)
  services.genossi.dev = {
    enable = true;
    port = 3000;
    host = "0.0.0.0";  # Accessible from network
    features = [ "mock_auth" ];
    logLevel = "debug";
  };

  # Example 2: Production with domain and SSL (automatic nginx + Let's Encrypt)
  services.genossi.prod = {
    enable = true;
    domain = "genossi.example.com";  # This enables nginx reverse proxy
    # ssl = true;       # Default: true - Uses Let's Encrypt
    # forceSSL = true;  # Default: true - Redirects HTTP to HTTPS
    features = [ "oidc" ];
    logLevel = "info";
    
    extraEnvironment = {
      OIDC_ISSUER = "https://auth.example.com";
      OIDC_CLIENT_ID = "genossi";
      # OIDC_CLIENT_SECRET should be in a separate secure file
    };
  };

  # Example 3: Staging with domain but no SSL (for testing)
  services.genossi.staging = {
    enable = true;
    port = 3001;  # Different port to avoid conflicts
    domain = "staging.local";
    ssl = false;  # HTTP only, no certificates
    features = [ "mock_auth" ];
  };

  # Example 4: Multiple production instances on different subdomains
  services.genossi = {
    eu = {
      enable = true;
      domain = "eu.genossi.com";
      features = [ "oidc" ];
    };
    
    us = {
      enable = true;
      port = 3002;  # Different internal port
      domain = "us.genossi.com";
      features = [ "oidc" ];
    };
  };

  # Required: Set ACME email for Let's Encrypt (if not set per-domain)
  security.acme.defaults.email = "admin@example.com";
}