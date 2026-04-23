#!/usr/bin/env bash
# PostToolUse hook: run `cargo fmt` when a .rs file is edited.
# Reads tool-call JSON from stdin; no-op for non-.rs edits.
set -euo pipefail

input="$(cat)"
file_path="$(printf '%s' "$input" | sed -n 's/.*"file_path"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' | head -n1)"

case "$file_path" in
  *.rs) cargo fmt --quiet 2>/dev/null || true ;;
  *) : ;;
esac
