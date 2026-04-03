#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")"

echo "Installing dependencies..."
npm install

echo "Building CodeMirror bundle..."
node build.mjs

echo "Done! Files written to ../assets/"
