# Development shell for Genossi Frontend
{ pkgs ? import <nixpkgs> {
    overlays = [
      (import (builtins.fetchTarball "https://github.com/oxalica/rust-overlay/archive/master.tar.gz"))
    ];
  }
}:

pkgs.mkShell {
  buildInputs = with pkgs; [
    # Rust toolchain with WebAssembly support
    (rust-bin.stable.latest.default.override {
      extensions = [ "rust-src" "rust-analyzer" ];
      targets = [ "wasm32-unknown-unknown" ];
    })
    
    # WebAssembly tools
    wasm-pack
    wasm-bindgen-cli
    wasmtime
    
    # Dioxus CLI from nixpkgs
    dioxus-cli
    
    # Node.js for Tailwind and package management
    nodejs
    nodePackages.npm
    
    # Tailwind CSS
    tailwindcss
    
    # System dependencies
    pkg-config
    openssl
    
    # Development tools
    cargo-watch
  ];

  # Environment variables
  RUST_TARGET = "wasm32-unknown-unknown";
  CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_LINKER = "lld";
  
  # Shell hook with environment info
  shellHook = ''
    echo "🦀 Genossi Frontend Development Environment"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo "✅ dioxus-cli ($(dx --version)) is available"
    echo ""
    echo "🛠️  Available commands:"
    echo "  dx serve           - Start development server"
    echo "  dx build          - Build for production"
    echo "  cargo check       - Check code for errors"
    echo "  npm run build-css - Build Tailwind CSS"
    echo ""
    echo "🚀 Run 'dx serve' to start the development server"
  '';

  # Set up environment for better development experience
  RUST_SRC_PATH = "${pkgs.rust.packages.stable.rustPlatform.rustLibSrc}";
}