#!/usr/bin/env bash

ZIP_URL="https://github.com/neosam/genossi/archive/$1.zip"

echo "Fetching SHA256 for $ZIP_URL ..."
SHA256=$(nix-prefetch-url --unpack "$ZIP_URL" 2>/dev/null)
echo "SHA256: $SHA256"

nix build "$ZIP_URL#backend-oidc"
nix-copy-closure --to shifty.nebenan-unverpackt.de result
nix build "$ZIP_URL#frontend"
nix-copy-closure --to shifty.nebenan-unverpackt.de result
