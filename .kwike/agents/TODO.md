# Developmeh Publish Workflow

Automated content organization and publishing for developmeh.com.

**Help:** `kwike doc consume` | `kwike doc patterns`

## Overview

This workflow automatically organizes content after user commits:

1. **Validates frontmatter** - ensures required fields (desc, keywords, dates)
2. **Updates home.md** - adds new devlogs/articles to homepage listings
3. **Commits changes** - uses robot author to prevent infinite loops

## Event Flow

```
git commit (user)
    │
    └─► post-commit hook
            │
            └─► dispatches publish.commit
                    │
                    └─► doc-organizer (creates lock file)
                            │
                            └─► publish.commit.done
                                    │
                                    └─► commit-agent
                                            │
                                            ├─► publish.commit.review.done (approved)
                                            │       └─► removes lock file
                                            │
                                            └─► publish.commit.review.rejected
                                                    │
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
- **Creates:** Lock file at `.kwike/locks/<event-id>.lock`
- **Does:**
  - Validates/fixes frontmatter (desc, keywords, updated date)
  - Updates content/landings/home.md devlogs/articles sections
  - NEVER touches: draft, discussion_number, discussion_url, actual content

### commit-agent
- **Triggered by:** `publish.commit.done`
- **Verifies:** Changes are correct, frontmatter is valid
- **Commits with:** `DevelopmehPublishRobot <robot@developmeh.com>`
- **Removes:** Lock file after successful commit
- **verify.sh:** Ensures commit author is correct (prevents infinite loops)

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

## Lock File Mechanism

- **Purpose:** Prevents git push while doc-organizer is working
- **Location:** `.kwike/locks/<event-id>.lock`
- **Created by:** doc-organizer at start
- **Removed by:** commit-agent after successful commit
- **pre-push hook:** Waits up to 5 minutes for locks to clear

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

## Frontmatter Requirements

```toml
+++
title = "Required"
template = "page.html"        # Added if missing
date = 2025-01-01             # Keep existing
updated = 2025-03-15          # Auto-updated to today
[extra]
desc = "Auto-generated"       # Generated from content if missing
keywords = "comma, separated" # Generated from content if missing
# NEVER TOUCHED:
draft = false
discussion_number = 123
discussion_url = "..."
+++
```

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
