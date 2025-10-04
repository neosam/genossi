{ features ? ["mock_auth"], ... }:
let
  specificPkgs = import (builtins.fetchTarball {
    url = "https://github.com/NixOS/nixpkgs/archive/b024ced1aac25639f8ca8fdfc2f8c4fbd66c48ef.tar.gz";
    sha256 = "sha256:09dahi81cn02gnzsc8a00n945dxc18656ar0ffx5vgxjj1nhgsvy";
  }) {};
  src = ./.;
  rustPlatform = specificPkgs.rustPlatform;
in
  rustPlatform.buildRustPackage {
    pname = "inventurly-service";
    version = "0.1.0";
    src = src;
    nativeBuildInputs = with specificPkgs; [curl pkg-config openssl];
    buildInputs = with specificPkgs; [sqlite openssl];
    buildFeatures = features;
    buildNoDefaultFeatures = true;
    SQLX_OFFLINE = "true";

    postInstall = ''
  cp -r $src/migrations $out/

  # Create the start script
  echo "#!${specificPkgs.bash}/bin/bash" > $out/bin/start.sh
  echo "set +a" >> $out/bin/start.sh
  echo "${specificPkgs.sqlx-cli}/bin/sqlx db setup --source $out/migrations/sqlite" >> $out/bin/start.sh
  echo "$out/bin/inventurly" >> $out/bin/start.sh
  chmod a+x $out/bin/start.sh
  '';

    cargoHash = "sha256-EBb5gbo9S5KB+93IpCWTuOXFy7jcEm1NZLsRvxEY0gg=";
  }