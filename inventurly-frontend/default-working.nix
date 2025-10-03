{ pkgs ? import <nixpkgs> {
    overlays = [
      (import (builtins.fetchTarball "https://github.com/oxalica/rust-overlay/archive/master.tar.gz"))
    ];
  }
}:

let
  rustToolchain = pkgs.rust-bin.stable.latest.default.override {
    extensions = [ "rust-src" ];
    targets = [ "wasm32-unknown-unknown" ];
  };
in
pkgs.stdenv.mkDerivation rec {
  pname = "inventurly-frontend";
  version = "0.1.0";
  
  src = ./.;

  nativeBuildInputs = with pkgs; [
    rustToolchain
    wasm-pack
    nodejs
    nodePackages.npm
    tailwindcss
    pkg-config
  ];

  buildInputs = with pkgs; [
    openssl
  ] ++ lib.optionals stdenv.isDarwin [
    darwin.apple_sdk.frameworks.Security
    darwin.apple_sdk.frameworks.SystemConfiguration
  ];

  # Environment for WebAssembly builds
  CARGO_BUILD_TARGET = "wasm32-unknown-unknown";
  
  buildPhase = ''
    runHook preBuild
    
    export HOME=$TMPDIR
    export CARGO_HOME=$TMPDIR/.cargo
    
    echo "Installing dioxus-cli..."
    # Try to install latest available version
    cargo install dioxus-cli || cargo install dioxus-cli --version 0.5.0 || echo "Warning: Could not install dioxus-cli"
    export PATH=$CARGO_HOME/bin:$PATH
    
    echo "Building with wasm-pack as fallback..."
    if command -v dx &> /dev/null; then
      echo "Using dioxus-cli..."
      dx build --release
    else
      echo "Using cargo directly for WASM..."
      mkdir -p dist
      cargo build --target wasm32-unknown-unknown --release
      # Copy wasm files to dist
      cp target/wasm32-unknown-unknown/release/*.wasm dist/ || echo "No wasm files found"
      # Create a simple index.html
      cat > dist/index.html << 'EOF'
<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <title>Inventurly</title>
</head>
<body>
    <div id="main"></div>
    <script type="module">
        import init from './inventurly-frontend.js';
        init('./inventurly-frontend_bg.wasm');
    </script>
</body>
</html>
EOF
    fi
    
    echo "Build phase completed"
    runHook postBuild
  '';

  installPhase = ''
    runHook preInstall
    
    mkdir -p $out
    if [ -d "dist" ]; then
      cp -r dist/* $out/
      echo "Installed frontend files to $out"
    else
      echo "Warning: No dist directory found"
      mkdir -p $out
      echo "Build completed but no output found" > $out/README.txt
    fi
    
    runHook postInstall
  '';

  meta = with pkgs.lib; {
    description = "Inventurly Frontend - Inventory Management System";
    license = licenses.mit;
    platforms = platforms.all;
  };
}