#!/usr/bin/env bash
#
# Publish gate for the commit-agent.
#
# The commit-agent used to decide whether the doc-organizer's edits were valid
# by reading a diff and forming an opinion. It had no Bash access beyond git, so
# "verify frontmatter is valid TOML" and "links in home.md are not broken" were
# aspirations rather than checks. This script makes them checks, which is the
# rules-before-models move: the model decides nothing a script can decide.
#
# Four gates, all run, all reported, so one invocation names everything that is
# wrong rather than only the first thing:
#
#   1. zola build  - malformed TOML, undeclared taxonomies, template breakage.
#                    This is the failure that took the live build down when
#                    tagged content shipped ahead of the config declaring the
#                    taxonomy.
#   2. link audit  - every internal href in the pages the organizer edits must
#                    resolve in the build output. Zola does NOT check plain
#                    "/path/" links, only "@/file.md" ones, so home.md's link
#                    list is otherwise unverified. This is the organizer's
#                    highest-risk edit.
#   3. bats        - the discoverability regression suite. Every assertion in it
#                    corresponds to something that was once broken.
#   4. zola check  - anchors and external links, advisory only.
#
# Runs against the WORKING TREE, before the robot commit, so a failing batch is
# never committed in the first place.
#
#   .kwike/agents/commit-agent/gate.sh            # all gates
#   .kwike/agents/commit-agent/gate.sh --quick    # skip bats and zola check
#
# Env:
#   GATE_OUTPUT_DIR   build output dir, default ./_site (matches CI and the
#                     bats suite's SITE default)
#   GATE_LINK_PAGES   space-separated built pages for gate 2, default is the
#                     homepage output
#
# Exit codes:
#   0  all blocking gates passed
#   1  a blocking gate failed; stdout is suitable for verbatim use as rejection
#      feedback
#   2  the gate could not run (missing tooling). Inconclusive, not a rejection.

set -uo pipefail   # deliberately not -e: every gate runs even after a failure

REPO_ROOT="$(git rev-parse --show-toplevel)" || exit 2
cd "$REPO_ROOT" || exit 2

OUT="${GATE_OUTPUT_DIR:-./_site}"
export SITE="$OUT"          # the bats suite reads SITE
QUICK=0
[ "${1:-}" = "--quick" ] && QUICK=1

# ---------------------------------------------------------------------------
# Tooling. The root dev shell pulls deploy/flake.nix in through inputsFrom, so
# zola, bats, jq and python3 are already on PATH when kwike-start ran inside
# `nix develop`. Fall back to entering the deploy shell for a consumer started
# outside it.
# ---------------------------------------------------------------------------
if command -v zola >/dev/null 2>&1; then
  RUN=()
elif command -v nix >/dev/null 2>&1; then
  RUN=(nix develop ./deploy --command)
else
  echo "GATE INCONCLUSIVE: neither zola nor nix is on PATH."
  echo "Start the consumers from inside 'nix develop' so the toolchain is available."
  exit 2
fi

run() {
  if [ "${#RUN[@]}" -eq 0 ]; then "$@"; else "${RUN[@]}" "$@"; fi
}

failed=()
report=""
note() { report+="$1"$'\n'; }

# ---------------------------------------------------------------------------
# Gate 1: build
# ---------------------------------------------------------------------------
# --force is required: zola refuses to build into an existing --output-dir, so
# without it the gate passes once and then fails on every subsequent run, which
# would reject every batch after the first.
log="$(mktemp)"
if run zola build --force --output-dir "$OUT" >"$log" 2>&1; then
  note "PASS  zola build"
else
  note "FAIL  zola build"
  note "$(tail -n 30 "$log" | sed 's/^/        /')"
  rm -f "$log"
  printf '%s\n' "$report"
  echo "GATE FAILED: the site does not build. Nothing else was run."
  exit 1
fi
rm -f "$log"

# ---------------------------------------------------------------------------
# Gate 2: internal link audit on the organizer-edited pages
#
# Zola validates "@/file.md" links at build time but leaves literal "/path/"
# hrefs completely unchecked, and every link the doc-organizer writes into
# home.md is the literal kind. A wrong slug here produces a 404 that no other
# gate notices.
# ---------------------------------------------------------------------------
link_pages="${GATE_LINK_PAGES:-}"
if [ -z "$link_pages" ]; then
  for candidate in "$OUT/home/index.html" "$OUT/index.html"; do
    [ -f "$candidate" ] && link_pages+="$candidate "
  done
fi

bad_links=""
for page in $link_pages; do
  [ -f "$page" ] || continue
  while IFS= read -r href; do
    # Strip query and fragment, then resolve against the build output.
    path="${href%%#*}"; path="${path%%\?*}"; path="${path%/}"
    [ -z "$path" ] && continue
    if [ ! -e "$OUT$path" ] && [ ! -e "$OUT$path/index.html" ] && [ ! -d "$OUT$path" ]; then
      bad_links+="        $(basename "$(dirname "$page")")/ -> $href"$'\n'
    fi
  done < <(grep -o 'href="/[^"]*"' "$page" | sed 's/href="//; s/"$//' | sort -u)
done

if [ -n "$bad_links" ]; then
  failed+=("links")
  note "FAIL  internal link audit"
  note "$(printf '%s' "$bad_links" | head -n 25)"
else
  note "PASS  internal link audit ($(echo $link_pages | wc -w) page(s))"
fi

# ---------------------------------------------------------------------------
# Gate 3: discoverability suite
# ---------------------------------------------------------------------------
if [ "$QUICK" -eq 1 ]; then
  note "SKIP  bats (--quick)"
elif [ ! -f tests/discoverability.bats ]; then
  note "SKIP  bats (tests/discoverability.bats not found)"
else
  log="$(mktemp)"
  if run bats tests/discoverability.bats >"$log" 2>&1; then
    note "PASS  bats tests/discoverability.bats"
  else
    failed+=("bats")
    note "FAIL  bats tests/discoverability.bats"
    note "$(grep -E '^not ok|^#' "$log" | head -n 40 | sed 's/^/        /')"
  fi
  rm -f "$log"
fi

# ---------------------------------------------------------------------------
# Gate 4: zola check, advisory
#
# Covers anchors and external URLs. External hosts rate-limit and return 418,
# and config.toml's [link_checker] skip_prefixes only covers the known ones, so
# a third-party host having a bad afternoon must not block the author's
# metadata. Reported, never blocking.
# ---------------------------------------------------------------------------
if [ "$QUICK" -eq 1 ]; then
  note "SKIP  zola check (--quick)"
else
  log="$(mktemp)"
  if run zola check >"$log" 2>&1; then
    note "PASS  zola check"
  else
    note "WARN  zola check reported failures, not blocking"
    note "$(tail -n 12 "$log" | sed 's/^/        /')"
  fi
  rm -f "$log"
fi

printf '%s\n' "$report"

if [ "${#failed[@]}" -gt 0 ]; then
  echo "GATE FAILED: ${failed[*]}"
  exit 1
fi

echo "GATE PASSED"
exit 0
