{ pkgs ? import <nixpkgs> {} }:

pkgs.stdenv.mkDerivation rec {
  pname = "inventurly-frontend";
  version = "0.1.0";

  src = ./.;

  nativeBuildInputs = with pkgs; [
    # Rust toolchain with WebAssembly support
    (rust-bin.stable.latest.default.override {
      extensions = [ "rust-src" ];
      targets = [ "wasm32-unknown-unknown" ];
    })
    
    # Required for building and bundling
    wasm-pack
    nodejs
    nodePackages.npm
    
    # For Tailwind CSS
    tailwindcss
    
    # System dependencies
    pkg-config
    openssl
  ];

  buildInputs = with pkgs; [
    openssl
  ] ++ lib.optionals stdenv.isDarwin [
    darwin.apple_sdk.frameworks.Security
    darwin.apple_sdk.frameworks.SystemConfiguration
  ];

  # Install dioxus-cli in a separate build step
  buildPhase = ''
    export HOME=$TMPDIR
    export CARGO_HOME=$TMPDIR/.cargo
    
    echo "Installing dioxus-cli..."
    cargo install dioxus-cli --version 0.6.1
    export PATH=$CARGO_HOME/bin:$PATH
    
    echo "Building Tailwind CSS..."
    npx tailwindcss -i ./input.css -o ./assets/tailwind.css --minify
    
    echo "Building Dioxus frontend..."
    dx build --release
  '';

  installPhase = ''
    echo "Installing built frontend..."
    mkdir -p $out
    cp -r dist/* $out/
    
    # Create a simple index file if needed
    if [ ! -f "$out/index.html" ]; then
      echo "Creating index.html..."
      cat > $out/index.html << 'EOF'
<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <title>Inventurly</title>
    <link rel="stylesheet" href="/tailwind.css">
</head>
<body>
    <div id="main"></div>
    <script type="module" src="/assets/inventurly-frontend.js"></script>
</body>
</html>
EOF
    fi
  '';

  meta = with pkgs.lib; {
    description = "Inventurly Frontend - Inventory Management System";
    license = licenses.mit;
    platforms = platforms.all;
  };
}