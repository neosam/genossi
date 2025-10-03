{
  description = "Inventurly Frontend - Inventory Management System";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, rust-overlay, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };
        
        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" "rust-analyzer" ];
          targets = [ "wasm32-unknown-unknown" ];
        };
      in
      {
        packages.default = pkgs.rustPlatform.buildRustPackage rec {
          pname = "inventurly-frontend";
          version = "0.1.0";
          
          src = ./.;
          
          cargoLock = {
            lockFile = ./Cargo.lock;
          };

          nativeBuildInputs = with pkgs; [
            rustToolchain
            wasm-pack
            wasm-bindgen-cli
            dioxus-cli
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

          CARGO_BUILD_TARGET = "wasm32-unknown-unknown";
          
          buildPhase = ''
            runHook preBuild
            
            export HOME=$TMPDIR
            export CARGO_HOME=$TMPDIR/.cargo
            
            # Disable wasm-opt to avoid parsing errors
            export DIOXUS_WASM_OPT_DISABLE=1
            
            echo "Building Tailwind CSS..."
            if [ -f "./input.css" ]; then
              tailwindcss -i ./input.css -o ./assets/tailwind.css --minify
            fi
            
            echo "Building Dioxus frontend..."
            # Try building with cargo directly to avoid wasm-opt issues
            mkdir -p dist
            cargo build --target wasm32-unknown-unknown --release
            
            # Copy the wasm file and create basic HTML
            if [ -f "target/wasm32-unknown-unknown/release/inventurly-frontend.wasm" ]; then
              cp target/wasm32-unknown-unknown/release/inventurly-frontend.wasm dist/
              
              # Generate JS bindings with wasm-bindgen
              wasm-bindgen --out-dir dist --target web target/wasm32-unknown-unknown/release/inventurly-frontend.wasm
              
              # Create index.html
              cat > dist/index.html << 'EOF'
<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <title>Inventurly</title>
    <link rel="stylesheet" href="tailwind.css">
</head>
<body>
    <div id="main"></div>
    <script type="module">
        import init from './inventurly-frontend.js';
        init();
    </script>
</body>
</html>
EOF
              
              # Copy CSS if it exists
              if [ -f "assets/tailwind.css" ]; then
                cp assets/tailwind.css dist/
              fi
            else
              echo "Error: WASM file not found"
              exit 1
            fi
            
            runHook postBuild
          '';

          installPhase = ''
            runHook preInstall
            
            mkdir -p $out
            if [ -d "dist" ]; then
              cp -r dist/* $out/
            else
              echo "Warning: dist directory not found"
              mkdir -p $out
              echo "Build failed - no dist output" > $out/error.txt
            fi
            
            runHook postInstall
          '';

          dontCargoCheck = true;
          dontCargoBuild = true;

          meta = with pkgs.lib; {
            description = "Inventurly Frontend - Inventory Management System";
            license = licenses.mit;
            platforms = platforms.all;
          };
        };

        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            rustToolchain
            wasm-pack
            wasm-bindgen-cli
            wasmtime
            dioxus-cli
            nodejs
            nodePackages.npm
            tailwindcss
            pkg-config
            openssl
            cargo-watch
          ];

          RUST_TARGET = "wasm32-unknown-unknown";
          CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_LINKER = "lld";
          
          shellHook = ''
            echo "🦀 Inventurly Frontend Development Environment"
            echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
            echo "✅ dioxus-cli ($(dx --version)) is available"
            echo ""
            echo "🛠️  Available commands:"
            echo "  dx serve           - Start development server"
            echo "  dx build          - Build for production"
            echo "  nix build         - Build with Nix"
            echo "  cargo check       - Check code for errors"
            echo ""
          '';

          RUST_SRC_PATH = "${pkgs.rust.packages.stable.rustPlatform.rustLibSrc}";
        };
      }
    );
}