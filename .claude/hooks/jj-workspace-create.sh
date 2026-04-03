#!/usr/bin/env bash
set -euo pipefail

# Read hook input from stdin (JSON with cwd, name, session_id, etc.)
INPUT=$(cat)
CWD=$(echo "$INPUT" | python3 -c "import sys,json; print(json.load(sys.stdin)['cwd'])")
NAME=$(echo "$INPUT" | python3 -c "import sys,json; print(json.load(sys.stdin)['name'])")

WORKTREE_PATH="$CWD/.claude/worktrees/$NAME"

# Create jj workspace at the specified path
cd "$CWD"
jj workspace add "$WORKTREE_PATH" --name "$NAME"

# Output the created path (required by WorktreeCreate contract)
echo "$WORKTREE_PATH"
