# NixOS module for Inventurly service
{ config, lib, pkgs, ... }:

let
  cfg = config.services.inventurly;
in
{
  options.services.inventurly = lib.mkOption {
    type = lib.types.attrsOf (lib.types.submodule {
      options = {
        enable = lib.mkOption {
          type = lib.types.bool;
          default = false;
          description = "Enable this Inventurly instance";
        };
        
        package = lib.mkOption {
          type = lib.types.package;
          description = "Inventurly package to use";
          default = pkgs.callPackage (./default.nix) { inherit pkgs; features = ["oidc"]; };
        };
        
        frontendPackage = lib.mkOption {
          type = lib.types.package;
          description = "Inventurly frontend package to use";
          default = pkgs.callPackage (./inventurly-frontend/default.nix) { inherit pkgs; };
        };
        
        port = lib.mkOption {
          type = lib.types.port;
          default = 3000;
          description = "Port to listen on";
        };
        
        host = lib.mkOption {
          type = lib.types.str;
          default = "127.0.0.1";
          description = "Host to bind to";
        };
        
        logLevel = lib.mkOption {
          type = lib.types.str;
          default = "inventurly=debug,tower_http=debug";
          description = "Rust log level configuration";
        };
        
        domain = lib.mkOption {
          type = lib.types.nullOr lib.types.str;
          default = null;
          description = "Domain name for nginx reverse proxy. If set, enables nginx reverse proxy.";
        };
        
        enableSSL = lib.mkOption {
          type = lib.types.bool;
          default = true;
          description = "Enable SSL/TLS with Let's Encrypt (only used when domain is set)";
        };
        
        extraEnvironment = lib.mkOption {
          type = lib.types.attrsOf lib.types.str;
          default = {};
          description = "Additional environment variables";
        };
      };
    });
    default = {};
    description = "Inventurly service instances";
  };
  
  config = lib.mkMerge [
    # Systemd services
    {
      systemd.services = lib.mapAttrs' (name: instanceCfg:
        lib.nameValuePair "inventurly-${name}" (lib.mkIf instanceCfg.enable {
          description = "Inventurly Service (${name})";
          wantedBy = [ "multi-user.target" ];
          after = [ "network.target" ];
          
          environment = {
            DATABASE_URL = "sqlite:/var/lib/inventurly-${name}/inventurly.db";
            SERVER_ADDRESS = "${instanceCfg.host}:${toString instanceCfg.port}";
            RUST_LOG = instanceCfg.logLevel;
          } // instanceCfg.extraEnvironment;
          
          serviceConfig = {
            Type = "simple";
            ExecStart = "${instanceCfg.package}/bin/inventurly";
            StateDirectory = "inventurly-${name}";
            WorkingDirectory = "/var/lib/inventurly-${name}";
            Restart = "on-failure";
          };
          
          preStart = ''
            # Initialize database
            if [ ! -f /var/lib/inventurly-${name}/inventurly.db ]; then
              ${pkgs.sqlite}/bin/sqlite3 /var/lib/inventurly-${name}/inventurly.db "VACUUM;"
            fi
            
            # Copy and run migrations
            if [ ! -d /var/lib/inventurly-${name}/migrations ]; then
              cp -r ${instanceCfg.package}/migrations /var/lib/inventurly-${name}/
            fi
            
            # Run migrations
            cd /var/lib/inventurly-${name}
            ${pkgs.sqlx-cli}/bin/sqlx database setup --source ./migrations/sqlite || true
          '';
        })
      ) cfg;

    }

    {
      # Create etc directories
      environment.etc = lib.mapAttrs' (name: instanceCfg: 
        lib.nameValuePair "inventurly-${name}/config.json" {
          text = lib.mkIf instanceCfg.enable ''
            {
              "backend": "http://localhost:${toString instanceCfg.port}/api"
            }
          '';
        }) cfg;
    }

    # Nginx configuration for instances with domains
    (lib.mkIf (lib.any (instanceCfg: instanceCfg.enable && instanceCfg.domain != null) (lib.attrValues cfg)) {
      services.nginx = {
        enable = lib.mkDefault true;
        recommendedGzipSettings = lib.mkDefault true;
        recommendedOptimisation = lib.mkDefault true;
        recommendedProxySettings = lib.mkDefault true;
        recommendedTlsSettings = lib.mkDefault true;
        
        virtualHosts = lib.mapAttrs' (name: instanceCfg:
          lib.nameValuePair instanceCfg.domain {
            forceSSL = instanceCfg.enableSSL;
            enableACME = instanceCfg.enableSSL;

            locations."= /authenticate" = {
              proxyPass = "http://127.0.0.1:${toString instanceCfg.port}";
              priority = 100;
            };

            locations."= /api-docs" = {
              proxyPass = "http://127.0.0.1:${toString instanceCfg.port}";
              priority = 100;
            };

            locations."= /api/" = {
              proxyPass = "http://127.0.0.1:${toString instanceCfg.port}";
              priority = 100;
              extraConfig = ''
                rewrite ^/api/(.*)$ /$1 break;
              '';
            };
            locations."= /config.json" = {
              alias = "/etc/inventurly-${name}/config.json";
              extraConfig = "add_header ContentType application/json;";
              priority = 200;
            };
            locations."= /assets/config.json" = {
              alias = "/etc/inventurly-${name}/config.json";
              extraConfig = "add_header ContentType application/json;";
              priority = 200;
            };
            locations."/" = {
              root = instanceCfg.frontendPackage;
              priority = 300;
              tryFiles = "$uri /index.html =200";
            };
          }
        ) (lib.filterAttrs (_: instanceCfg: instanceCfg.enable && instanceCfg.domain != null) cfg);
      };
    })
    
    # ACME configuration for SSL
    (lib.mkIf (lib.any (instanceCfg: instanceCfg.enable && instanceCfg.domain != null && instanceCfg.enableSSL) (lib.attrValues cfg)) {
      security.acme = {
        acceptTerms = lib.mkDefault true;
        #defaults.email = lib.mkDefault "admin@example.com"; # Users should override this
      };
    })
  ];
}