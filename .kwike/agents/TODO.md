# Developmeh Publish Workflow

Automated content organization and publishing for developmeh.com.

**Help:** `kwike doc consume` | `kwike doc patterns`

## Overview

This workflow organizes content after user commits:

1. **Validates front matter** - required fields, plus the `[taxonomies]` block
   that puts a page on the topic hubs
2. **Updates home.md** - adds new devlogs/articles to homepage listings, and
   migrates links when files move
3. **Gates on a real build** - `zola build`, an internal link audit, and the
   discoverability suite must pass before anything is committed
4. **Commits changes** - uses robot author to prevent infinite loops

## Event Flow

```
git commit (user)
    │
    └─► post-commit hook
            │
            ├─► creates lock file
            └─► dispatches publish.commit (with commit_date)
                    │
                    └─► doc-organizer
                            │
                            └─► publish.commit.done
                                    │
                                    └─► commit-agent
                                            │
                                            ├─► gate.sh (build + links + bats)
                                            │
                                            ├─► publish.commit.review.done (approved)
                                            │       └─► verify.sh removes lock file
                                            │
                                            └─► publish.commit.review.rejected
                                                    │       (lock RETAINED)
                                                    └─► doc-organizer (RESUMES)
                                                            │
                                                            └─► retry...

git push (user)
    │
    └─► pre-push hook
            │
            └─► waits for lock file removal
                    │
                    └─► push proceeds
```

## Agents

### doc-organizer

- **Triggered by:** `publish.commit` (new), `publish.commit.review.rejected` (resume)
- **Model:** sonnet
- **Does:**
  - Validates/fixes front matter (`desc`, `keywords`, `updated`, `sitemap_priority`)
  - Validates/fixes `[taxonomies] topics` against the closed 21-value vocabulary
    in `config.toml`'s `[extra.topic_descriptions]`
  - Updates `content/landings/home.md` devlogs/articles sections
  - Migrates links across all of `content/` when files are renamed
  - Reports `config_requests` and `faq_suggested` for a human to action
- **NEVER touches:** prose, `draft`, `discussion_number`, `discussion_url`,
  `slug`, `path`, `aliases`, `date`, `[[extra.faq]]`, `schema_type`, or any file
  outside `content/`

### commit-agent

- **Triggered by:** `publish.commit.done`
- **Model:** haiku, because `gate.sh` makes the pass/fail decision rather than
  the model
- **Runs:** `gate.sh`, and rejects on any non-zero exit
- **Commits with:** `DevelopmehPublishRobot <robot@developmeh.com>`
- **verify.sh:** enforces the robot author and `gate_result == "passed"`, then
  releases the push lock

## The Gate

`.kwike/agents/commit-agent/gate.sh` runs against the working tree, before the
robot commit, so a failing batch is never committed.

| Gate | Catches | Blocking |
|---|---|---|
| `zola build` | malformed TOML, undeclared taxonomies, template breakage | yes |
| internal link audit | wrong slugs in home.md links, which Zola does not check | yes |
| `bats tests/discoverability.bats` | JSON-LD, llms.txt, robots.txt, sitemap, chip URLs, DOM order | yes |
| `zola check` | anchors and external URLs | no, advisory |

Run it by hand any time:

```bash
.kwike/agents/commit-agent/gate.sh           # everything
.kwike/agents/commit-agent/gate.sh --quick   # build + links only
```

Exit 2 means the toolchain is missing, usually because the consumers were
started outside `nix develop`. That is inconclusive rather than a failure, and
the commit-agent treats it as a rejection rather than approving blind.

Note that `zola check` does **not** validate literal `/path/` links, only
`@/file.md` ones. Every link the doc-organizer writes into home.md is the
literal kind, which is why the link audit exists as a separate gate.

## Setup

```bash
# Enter dev shell
nix develop

# Install git hooks (once)
kwike-setup

# Start daemon and consumers
kwike-start

# Check status
kwike-status

# Stop everything
kwike-stop
```

The consumers must be started from inside `nix develop` so `zola`, `bats`, `jq`
and `python3` are on their PATH. The root dev shell pulls them in from
`deploy/flake.nix` via `inputsFrom`.

## Lock File Mechanism

- **Purpose:** prevents git push while the agents are working
- **Location:** `.kwike/locks/<commit-sha>.lock`
- **Created by:** the post-commit hook
- **Removed by:** `verify.sh`, on approval only
- **Retained on rejection**, because a rejection sends the doc-organizer back
  around for another pass and a push must not race the retry
- **pre-push hook:** waits up to 5 minutes for locks to clear, then tells you
  how to force

## Content Classification

Documents are classified as:

### Devlog-only

- Small header, immediately goes to `## DevLog`
- Contains `<div class="devlog-entry">` blocks
- Added to home.md devlogs section only

### Article-only

- Substantial content, no devlog section
- Added to home.md articles section only

### Hybrid

- Has content AND devlog section
- Added to BOTH sections

## Front Matter Requirements

```toml
+++
title = "Required"
template = "page.html"        # Added if missing
date = 2025-01-01             # Keep existing
updated = 2025-03-15          # Set from the payload's commit_date

[taxonomies]
topics = ["Agentic AI", "Testing"]   # 2-4, from the closed vocabulary

[extra]
desc = "Auto-generated"       # Generated from content if missing
keywords = "comma, separated" # Generated from content if missing
sitemap_priority = "0.8"      # Added for pages over 800 words

# NEVER TOUCHED:
draft = false
slug = "explicit-url"
discussion_number = 123
discussion_url = "..."
schema_type = "TechArticle"   # derived from section, override only
[[extra.faq]]                 # author's voice, suggested but never written
+++
```

### Topic Vocabulary

Closed set, 21 values. The keys of `[extra.topic_descriptions]` in `config.toml`
are the source of truth; this list is a convenience copy.

```
Agent Orchestration     Agentic AI              AWS
Capability Security     CI/CD                   Context Engineering
Creativity              Decentralized Web       Developer Experience
Distributed Systems     Engineering Leadership  Event Streaming
Go                      Nix                     Ruby
Rust                    Shell Testing           Software Architecture
Software Ethics         Testing                 WebAssembly
```

Adding a value here is a two-step change: add the `topic_descriptions` entry
first, then tag content with it. Tagging content ahead of the config is what
took the live build down once already.

## Advisory Channels

Two things the agents surface but never do, because they need a human or the
author's voice:

- **`config_requests`**, a wanted topic that has no `topic_descriptions` entry,
  a new article worth adding to `llms_featured`, or a content section missing
  from the `*_sections` lists in `config.toml`.
- **`faq_suggested`**, pages over 1000 words with no `[[extra.faq]]` block.
  FAQ pairs are extracted by answer engines out of proportion to their size, so
  they are worth writing, but the answers are prose.

Both are echoed to the consumer log by `verify.sh` and carried in the event
payload.

## Manual Testing

```bash
# Start kwike
kwike-start

# Make a test commit
echo "test" >> content/test.md
git add content/test.md
git commit -m "Test commit"

# Watch the magic happen
kwike-status

# View events
kwike events --daemon "unix://.kwike/daemon.sock"
```

To test the gate in isolation, without involving an agent:

```bash
.kwike/agents/commit-agent/gate.sh; echo "exit=$?"
```
