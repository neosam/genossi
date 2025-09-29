# NixOS module for Inventurly service
# 
# Usage in your NixOS configuration:
#   imports = [ /path/to/inventurly/module.nix ];
#   
#   services.inventurly.myinstance = {
#     enable = true;
#     port = 3000;
#     features = [ "mock_auth" ];
#   };

{ config, lib, pkgs, ... }:

with lib;

let
  cfg = config.services.inventurly;
  
  # Simple service creator without complex features for now
  mkInventurlyService = name: instanceCfg:
    let
      # Build the package with specified features
      inventurlyPackage = import ./default.nix { features = instanceCfg.features; };
      
      stateDir = "/var/lib/inventurly-${name}";
      dbPath = "${stateDir}/inventurly.db";
      
    in mkIf instanceCfg.enable {
      # Simple systemd service
      systemd.services."inventurly-${name}" = {
        description = "Inventurly Service (${name})";
        wantedBy = [ "multi-user.target" ];
        after = [ "network.target" ];
        
        environment = {
          DATABASE_URL = "sqlite:${dbPath}";
          SERVER_ADDRESS = "${instanceCfg.host}:${toString instanceCfg.port}";
          RUST_LOG = instanceCfg.logLevel;
        } // instanceCfg.extraEnvironment;
        
        serviceConfig = {
          Type = "simple";
          ExecStart = "${inventurlyPackage}/bin/inventurly";
          StateDirectory = "inventurly-${name}";
          WorkingDirectory = stateDir;
          Restart = "on-failure";
        };
        
        preStart = ''
          # Initialize database
          if [ ! -f ${dbPath} ]; then
            ${pkgs.sqlite}/bin/sqlite3 ${dbPath} "VACUUM;"
          fi
          
          # Copy and run migrations
          if [ ! -d ${stateDir}/migrations ]; then
            cp -r ${inventurlyPackage}/migrations ${stateDir}/
          fi
          cd ${stateDir}
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
    
    features = mkOption {
      type = types.listOf types.str;
      default = [ "mock_auth" ];
      description = "List of features to enable (mock_auth, oidc)";
    };
    
    logLevel = mkOption {
      type = types.str;
      default = "inventurly=debug,tower_http=debug";
      description = "Rust log level configuration";
    };
    
    extraEnvironment = mkOption {
      type = types.attrsOf types.str;
      default = {};
      description = "Additional environment variables";
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