#!/usr/bin/env bash
# Verify script for commit-agent
# Checks that the commit was made with the correct author and removes lock file

set -euo pipefail

# Get repo root dynamically (verify script runs from repo root)
REPO_ROOT="$(git rev-parse --show-toplevel)"
LOCKS_DIR="${REPO_ROOT}/.kwike/locks"

CONTRACT=$(cat)
STATUS=$(echo "$CONTRACT" | jq -r '.status')
ORIGINAL_SHA=$(echo "$CONTRACT" | jq -r '.original_commit_sha')

# Always remove lock on completion — processing is done regardless of outcome
LOCK_FILE="${LOCKS_DIR}/${ORIGINAL_SHA}.lock"
if [[ -f "$LOCK_FILE" ]]; then
  rm "$LOCK_FILE"
  echo "Removed lock file: $LOCK_FILE"
else
  echo "Note: Lock file not found at $LOCK_FILE (may have been manually removed)"
fi

# Only verify author for approved commits (rejected = no commit made)
if [[ "$STATUS" != "approved" ]]; then
  echo "Status: $STATUS — no commit to verify"
  exit 0
fi

EXPECTED_AUTHOR="DevelopmehPublishRobot <robot@developmeh.com>"
CONTRACT_AUTHOR=$(echo "$CONTRACT" | jq -r '.commit_author')
CONTRACT_SHA=$(echo "$CONTRACT" | jq -r '.commit_sha')

# No-op case: doc-organizer made no changes, commit_sha == original_sha
if [[ "$CONTRACT_SHA" == "$ORIGINAL_SHA" ]]; then
  echo "No changes needed — doc-organizer verified content is correct"
  exit 0
fi

# Verify contract author matches expected
if [[ "$CONTRACT_AUTHOR" != "$EXPECTED_AUTHOR" ]]; then
  echo "ERROR: Contract commit_author '$CONTRACT_AUTHOR' does not match expected '$EXPECTED_AUTHOR'" >&2
  echo "You MUST commit with --author=\"$EXPECTED_AUTHOR\"" >&2
  exit 1
fi

# Verify the actual git commit author (already in repo root)
ACTUAL_AUTHOR=$(git log -1 --format='%an <%ae>' "$CONTRACT_SHA" 2>/dev/null || echo "COMMIT_NOT_FOUND")

if [[ "$ACTUAL_AUTHOR" == "COMMIT_NOT_FOUND" ]]; then
  echo "ERROR: Commit $CONTRACT_SHA not found in repository" >&2
  exit 1
fi

if [[ "$ACTUAL_AUTHOR" != "$EXPECTED_AUTHOR" ]]; then
  echo "ERROR: Actual commit author '$ACTUAL_AUTHOR' does not match expected '$EXPECTED_AUTHOR'" >&2
  echo "The commit was made with the wrong author. This will cause infinite workflow loops!" >&2
  exit 1
fi

echo "Verification passed: commit $CONTRACT_SHA by $ACTUAL_AUTHOR"
exit 0
