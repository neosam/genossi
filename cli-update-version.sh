#!/usr/bin/env bash

set -euo pipefail

# Show commands being executed (for debugging)
set -x

TAG_MESSAGE=""
BRANCH="main"

while getopts "m:b:" opt; do
    case $opt in
        m) TAG_MESSAGE="$OPTARG" ;;
        b) BRANCH="$OPTARG" ;;
        *) echo "Usage: $0 [-m tag-message] [-b branch]" >&2; exit 1 ;;
    esac
done

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
if [[ -n "$TAG_MESSAGE" ]]; then
    git tag -a "v$NEW_VERSION" -m "$TAG_MESSAGE" "$BRANCH"
else
    git tag -a "v$NEW_VERSION" "$BRANCH"
fi
git push --tags

./update_versions.sh "$FOLLOWING_VERSION"
cargo build
jj commit -m "Set version to $FOLLOWING_VERSION"
jj b m "$BRANCH" --to @-
jj git push

echo "Released version: $NEW_VERSION"
