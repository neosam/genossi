#!/usr/bin/env bash
set -euo pipefail

# Read hook input from stdin
INPUT=$(cat)
WORKTREE_PATH=$(echo "$INPUT" | jq -r '.worktree_path')
WORKSPACE_NAME=$(basename "$WORKTREE_PATH")

# Forget the workspace in jj
jj workspace forget "$WORKSPACE_NAME" 2>/dev/null || true

# Clean up the directory
rm -rf "$WORKTREE_PATH"
