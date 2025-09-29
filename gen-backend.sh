#!/usr/bin/env bash

TEMPLATE_PATH="./default-template.nix"
OUTPUT_PATH="./default.nix"

echo "Finding cargo hash for Inventurly backend"

# Create initial version with empty cargo hash to trigger build error
sed -e "s/__CARGO_HASH__//g" $TEMPLATE_PATH > $OUTPUT_PATH

echo "Attempting build to discover cargo hash..."
output=$(nix-build $OUTPUT_PATH 2>&1)

# Extract the cargo hash from the error message
sha_line=$(echo "$output" | grep "got:\s*sha256-")

echo "Found hash line: $sha_line"

cargoHash=$(echo "$sha_line" | sed 's/\s*got:\s*//')

echo "Cargo hash: '$cargoHash'"

# Update the template with the discovered hash
sed -e "s|__CARGO_HASH__|$cargoHash|g" $TEMPLATE_PATH > $OUTPUT_PATH

echo "Successfully generated default.nix with cargo hash"
echo "You can now build with:"
echo "  ./build-backend.sh      # Build with mock_auth (default)"
echo "  ./build-oidc-backend.sh # Build with OIDC features"