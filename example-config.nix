# Example NixOS configuration showing different Inventurly deployment options
# This file demonstrates how to use the module in production

{ config, pkgs, ... }:

{
  imports = [ ./module.nix ];

  # Example 1: Development instance (direct access, no reverse proxy)
  services.inventurly.dev = {
    enable = true;
    port = 3000;
    host = "0.0.0.0";  # Accessible from network
    features = [ "mock_auth" ];
    logLevel = "debug";
  };

  # Example 2: Production with domain and SSL (automatic nginx + Let's Encrypt)
  services.inventurly.prod = {
    enable = true;
    domain = "inventurly.example.com";  # This enables nginx reverse proxy
    # ssl = true;       # Default: true - Uses Let's Encrypt
    # forceSSL = true;  # Default: true - Redirects HTTP to HTTPS
    features = [ "oidc" ];
    logLevel = "info";
    
    extraEnvironment = {
      OIDC_ISSUER = "https://auth.example.com";
      OIDC_CLIENT_ID = "inventurly";
      # OIDC_CLIENT_SECRET should be in a separate secure file
    };
  };

  # Example 3: Staging with domain but no SSL (for testing)
  services.inventurly.staging = {
    enable = true;
    port = 3001;  # Different port to avoid conflicts
    domain = "staging.local";
    ssl = false;  # HTTP only, no certificates
    features = [ "mock_auth" ];
  };

  # Example 4: Multiple production instances on different subdomains
  services.inventurly = {
    eu = {
      enable = true;
      domain = "eu.inventurly.com";
      features = [ "oidc" ];
    };
    
    us = {
      enable = true;
      port = 3002;  # Different internal port
      domain = "us.inventurly.com";
      features = [ "oidc" ];
    };
  };

  # Required: Set ACME email for Let's Encrypt (if not set per-domain)
  security.acme.defaults.email = "admin@example.com";
}