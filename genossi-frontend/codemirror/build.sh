#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")"

echo "Installing dependencies..."
npm install

echo "Building CodeMirror bundle..."
npx esbuild entry.js \
  --bundle \
  --format=iife \
  --minify \
  --external:codemirror-lang-typst \
  --outfile=../assets/codemirror-bundle.js

echo "Done! Files written to ../assets/"
