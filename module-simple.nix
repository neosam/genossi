# Simple working NixOS module for Inventurly service
{ config, lib, pkgs, ... }:

with lib;

let
  cfg = config.services.inventurly;
  
  # Pre-built package
  inventurlyPkg = import ./default.nix { features = ["mock_auth"]; };
  
  mkInventurlyService = name: instanceCfg:
    mkIf instanceCfg.enable {
      systemd.services."inventurly-${name}" = {
        description = "Inventurly Service (${name})";
        wantedBy = [ "multi-user.target" ];
        after = [ "network.target" ];
        
        environment = {
          DATABASE_URL = "sqlite:/var/lib/inventurly-${name}/inventurly.db";
          SERVER_ADDRESS = "${instanceCfg.host}:${toString instanceCfg.port}";
          RUST_LOG = instanceCfg.logLevel;
        };
        
        serviceConfig = {
          Type = "simple";
          ExecStart = "${inventurlyPkg}/bin/inventurly";
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
            cp -r ${inventurlyPkg}/migrations /var/lib/inventurly-${name}/
          fi
          cd /var/lib/inventurly-${name}
          ${pkgs.sqlx-cli}/bin/sqlx database setup --source ./migrations/sqlite || true
        '';
      };
    };
  
  instanceOptions = {
    enable = mkOption {
      type = types.bool;
      default = false;
      description = "Enable this Inventurly instance";
    };
    
    port = mkOption {
      type = types.port;
      default = 3000;
      description = "Port to listen on";
    };
    
    host = mkOption {
      type = types.str;
      default = "127.0.0.1";
      description = "Host to bind to";
    };
    
    logLevel = mkOption {
      type = types.str;
      default = "inventurly=debug,tower_http=debug";
      description = "Rust log level configuration";
    };
  };
  
in {
  options.services.inventurly = mkOption {
    type = types.attrsOf (types.submodule { options = instanceOptions; });
    default = {};
    description = "Inventurly service instances";
  };
  
  config = mkMerge (mapAttrsToList mkInventurlyService cfg);
}