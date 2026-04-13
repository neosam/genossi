#!/usr/bin/env bash

set -euo pipefail

# Show commands being executed (for debugging)
set -x

BRANCH="${1:-main}"

# Extract current version from default.nix
CURRENT_VERSION=$(grep -oP 'version = "\K[^"]+' default.nix | head -1)

# Verify it ends with -dev
if [[ ! "$CURRENT_VERSION" =~ ^([0-9]+)\.([0-9]+)\.([0-9]+)-dev$ ]]; then
    echo "Error: Current version '$CURRENT_VERSION' does not match expected pattern X.Y.Z-dev"
    exit 1
fi

MAJOR="${BASH_REMATCH[1]}"
MINOR="${BASH_REMATCH[2]}"
PATCH="${BASH_REMATCH[3]}"

# Release version: strip -dev
NEW_VERSION="${MAJOR}.${MINOR}.${PATCH}"

# Next dev version: increment minor, reset patch to 0
NEXT_MINOR=$((MINOR + 1))
FOLLOWING_VERSION="${MAJOR}.${NEXT_MINOR}.0-dev"

echo "Current version: $CURRENT_VERSION"
echo "Release version: $NEW_VERSION"
echo "Next dev version: $FOLLOWING_VERSION"

./update_versions.sh "$NEW_VERSION"
cargo build
jj commit -m "Set version to $NEW_VERSION"
jj b m "$BRANCH" --to @-
jj git push
git tag -a "v$NEW_VERSION" "$BRANCH"
git push --tags

./update_versions.sh "$FOLLOWING_VERSION"
cargo build
jj commit -m "Set version to $FOLLOWING_VERSION"
jj b m "$BRANCH" --to @-
jj git push
