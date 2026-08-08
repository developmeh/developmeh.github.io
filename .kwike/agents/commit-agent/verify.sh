#!/usr/bin/env bash
# Verify script for commit-agent
#
# Two jobs: enforce the robot author (a wrong author re-triggers the post-commit
# hook and loops forever), and manage the push lock.
#
# Lock semantics changed deliberately. This used to remove the lock on ANY
# terminal status with the comment "processing is done regardless of outcome",
# but that is not true of a rejection: `rejected` maps to
# publish.commit.review.rejected, which the doc-organizer consumes as a resume
# and then round-trips back here. Removing the lock there let a push through
# while the retry round was still rewriting content. The lock now survives a
# rejection and clears only on approval, with the pre-push hook's 5-minute
# timeout as the backstop for a loop that never converges.

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
LOCKS_DIR="${REPO_ROOT}/.kwike/locks"

CONTRACT=$(cat)
STATUS=$(echo "$CONTRACT" | jq -r '.status')
ORIGINAL_SHA=$(echo "$CONTRACT" | jq -r '.original_commit_sha')
GATE=$(echo "$CONTRACT" | jq -r '.gate_result // "absent"')
LOCK_FILE="${LOCKS_DIR}/${ORIGINAL_SHA}.lock"

# Rejected: the doc-organizer is about to resume. Keep the lock so a push cannot
# race the retry.
if [[ "$STATUS" != "approved" ]]; then
  echo "Status: $STATUS — no commit to verify, lock retained for the retry round"
  if [[ -f "$LOCK_FILE" ]]; then
    echo "Lock still held: $LOCK_FILE"
  fi
  exit 0
fi

# An approval must be backed by a green gate. The schema also asserts this, but
# the script is the layer that actually cannot be talked out of it.
if [[ "$GATE" != "passed" ]]; then
  echo "ERROR: approved with gate_result='${GATE}'. Approval requires gate_result='passed'." >&2
  echo "Run .kwike/agents/commit-agent/gate.sh and reject if it exits non-zero." >&2
  exit 1
fi

EXPECTED_AUTHOR="DevelopmehPublishRobot <robot@developmeh.com>"
CONTRACT_AUTHOR=$(echo "$CONTRACT" | jq -r '.commit_author')
CONTRACT_SHA=$(echo "$CONTRACT" | jq -r '.commit_sha')

# Verify contract author matches expected
if [[ "$CONTRACT_AUTHOR" != "$EXPECTED_AUTHOR" ]]; then
  echo "ERROR: Contract commit_author '$CONTRACT_AUTHOR' does not match expected '$EXPECTED_AUTHOR'" >&2
  echo "You MUST commit with --author=\"$EXPECTED_AUTHOR\"" >&2
  exit 1
fi

# No-op case: doc-organizer made no changes, commit_sha == original_sha.
if [[ "$CONTRACT_SHA" == "$ORIGINAL_SHA" ]]; then
  echo "No changes needed — doc-organizer verified content is correct"
else
  # Verify the actual git commit author.
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
fi

# Approved and verified. Release the push lock.
if [[ -f "$LOCK_FILE" ]]; then
  rm "$LOCK_FILE"
  echo "Removed lock file: $LOCK_FILE"
else
  echo "Note: Lock file not found at $LOCK_FILE (may have been manually removed)"
fi

# Surface any advisory requests so they are visible in the consumer log rather
# than only buried in the event payload.
REQS=$(echo "$CONTRACT" | jq -r '(.config_requests // [])[]' 2>/dev/null || true)
if [[ -n "$REQS" ]]; then
  echo "--- config_requests (needs a human) ---"
  printf '%s\n' "$REQS"
fi
FAQS=$(echo "$CONTRACT" | jq -r '(.faq_suggested // [])[]' 2>/dev/null || true)
if [[ -n "$FAQS" ]]; then
  echo "--- faq_suggested (needs the author's voice) ---"
  printf '%s\n' "$FAQS"
fi

exit 0
