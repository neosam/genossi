# Fixed NixOS module for Genossi service
{ config, lib, pkgs, ... }:

with lib;

let
  cfg = config.services.genossi;
  
  mkGenossiService = name: instanceCfg:
    mkIf instanceCfg.enable {
      systemd.services."genossi-${name}" = {
        description = "Genossi Service (${name})";
        wantedBy = [ "multi-user.target" ];
        after = [ "network.target" ];
        
        environment = {
          DATABASE_URL = "sqlite:/var/lib/genossi-${name}/genossi.db";
          SERVER_ADDRESS = "${instanceCfg.host}:${toString instanceCfg.port}";
          RUST_LOG = instanceCfg.logLevel;
        } // instanceCfg.extraEnvironment;
        
        serviceConfig = {
          Type = "simple";
          ExecStart = "${pkgs.rustPlatform.buildRustPackage {
            pname = "genossi-service";
            version = "0.1.0";
            src = ./.;
            nativeBuildInputs = with pkgs; [curl pkg-config openssl];
            buildInputs = with pkgs; [sqlite openssl];
            buildFeatures = instanceCfg.features;
            buildNoDefaultFeatures = true;
            SQLX_OFFLINE = "true";
            postInstall = ''
              cp -r $src/migrations $out/
            '';
            cargoHash = "sha256-HD8zvbAEqDkeIuWgkWheSOW5UyXxU/qZWSCae5jOFfk=";
          }}/bin/genossi";
          
          StateDirectory = "genossi-${name}";
          WorkingDirectory = "/var/lib/genossi-${name}";
          Restart = "on-failure";
        };
        
        preStart = ''
          # Initialize database
          if [ ! -f /var/lib/genossi-${name}/genossi.db ]; then
            ${pkgs.sqlite}/bin/sqlite3 /var/lib/genossi-${name}/genossi.db "VACUUM;"
          fi
          
          # Copy and run migrations
          if [ ! -d /var/lib/genossi-${name}/migrations ]; then
            cp -r ${pkgs.rustPlatform.buildRustPackage {
              pname = "genossi-service";
              version = "0.1.0";
              src = ./.;
              nativeBuildInputs = with pkgs; [curl pkg-config openssl];
              buildInputs = with pkgs; [sqlite openssl];
              buildFeatures = instanceCfg.features;
              buildNoDefaultFeatures = true;
              SQLX_OFFLINE = "true";
              postInstall = ''
                cp -r $src/migrations $out/
              '';
              cargoHash = "sha256-HD8zvbAEqDkeIuWgkWheSOW5UyXxU/qZWSCae5jOFfk=";
            }}/migrations /var/lib/genossi-${name}/
          fi
          cd /var/lib/genossi-${name}
          ${pkgs.sqlx-cli}/bin/sqlx database setup --source ./migrations/sqlite || true
        '';
      };
    };
  
  instanceOptions = {
    enable = mkOption {
      type = types.bool;
      default = false;
      description = "Enable this Genossi instance";
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
      default = "genossi=debug,tower_http=debug";
      description = "Rust log level configuration";
    };
    
    extraEnvironment = mkOption {
      type = types.attrsOf types.str;
      default = {};
      description = "Additional environment variables";
    };
  };
  
in {
  options.services.genossi = mkOption {
    type = types.attrsOf (types.submodule { options = instanceOptions; });
    default = {};
    description = "Genossi service instances";
  };
  
  config = mkMerge (mapAttrsToList mkGenossiService cfg);
}