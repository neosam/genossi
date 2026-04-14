#!/usr/bin/env bash

set -euo pipefail

# Show commands being executed (for debugging)
set -x

BRANCH="${1:-main}"

# Calculate date-based version components
YEAR=$(date +%Y)
DOY=$(date +%j)

# Determine increment number N from existing git tags
EXISTING_TAGS=$(git tag -l "v${YEAR}.${DOY}.*")
if [[ -z "$EXISTING_TAGS" ]]; then
    N=0
else
    LAST_N=$(echo "$EXISTING_TAGS" | sed "s/v${YEAR}\.${DOY}\.//" | sort -n | tail -1)
    N=$((LAST_N + 1))
fi

# Construct version strings
NEW_VERSION="${YEAR}.${DOY}.${N}"
NEXT_N=$((N + 1))
FOLLOWING_VERSION="${YEAR}.${DOY}.${NEXT_N}-dev"

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
