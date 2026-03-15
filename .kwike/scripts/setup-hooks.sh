#!/usr/bin/env bash
# Setup git hooks for developmeh.com kwike integration
# Run this once to install the hooks

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
KWIKE_DIR="$(dirname "$SCRIPT_DIR")"
REPO_ROOT="$(dirname "$KWIKE_DIR")"
GIT_HOOKS_DIR="${REPO_ROOT}/.git/hooks"

echo "Setting up kwike git hooks..."

# Create symlinks to our hooks
for hook in post-commit pre-push; do
  SOURCE="${KWIKE_DIR}/hooks/${hook}"
  TARGET="${GIT_HOOKS_DIR}/${hook}"

  if [[ -f "$TARGET" ]] && [[ ! -L "$TARGET" ]]; then
    echo "WARNING: $hook hook already exists and is not a symlink"
    echo "  Backing up to ${TARGET}.backup"
    mv "$TARGET" "${TARGET}.backup"
  fi

  if [[ -L "$TARGET" ]]; then
    rm "$TARGET"
  fi

  ln -s "$SOURCE" "$TARGET"
  echo "Installed $hook hook"
done

echo ""
echo "Git hooks installed successfully!"
echo ""
echo "The hooks will:"
echo "  post-commit: Dispatch publish.commit events to kwike daemon"
echo "  pre-push: Wait for doc-organizer to complete before pushing"
echo ""
echo "To start the kwike daemon and consumers, run:"
echo "  .kwike/scripts/start-kwike.sh"
