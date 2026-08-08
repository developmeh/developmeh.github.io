+++
title = "Where the Edges of Claude Code Fray: Context Exhaustion"
template = "page.html"
draft = true
weight = 0
date = 2026-02-15
[extra]
nav_title = "Claude Code Fray 1: Context Exhaustion"
desc = "On the experience of e2e testing catalyst-orchestrator with Claude Code, where context exhaustion and compaction break assumptions and the tool stops holding the full picture."
keywords = "claude code, e2e testing, context window, compaction, catalyst, beads, orchestration, devex, OOM"
schema_type = "BlogPosting"
sitemap_priority = "0.8"
bluesky_tags = "dev, claude, testing, devex"
+++

<!-- SERIES: "Where the Edges of Claude Code Fray"
Part 1: Context Exhaustion (this article)
Part 2: Timing Blindness (content/devex/where-the-edges-of-claude-code-fray-timing.md)
Future: TBD — enumerating failure patterns, some with solutions, some without
-->

<!-- OUTLINE: Intro
The problem: using Claude Code to build and e2e test catalyst-orchestrator. Two manifestations of
the same underlying issue — the tool runs out of room to think.
-->

<!-- OUTLINE: The Concurrency Motivation
- The original orchestration (written by Claude) had no flow control — no concurrency limits,
  no context cancellations
- Failures retried at full speed. In Go, running shell commands is blazing fast — unchecked
  goroutines blasted out
- First real unattended run burned through a Claude Max usage window in ~3 minutes from
  runaway concurrency
- Needed to assert the router and daemon would correctly react to success and failure before v0.3
- Wanted to use BATS for this (link: /tech-dives/bats-testing-bash-like-you-mean-it)
-->

<!-- OUTLINE: Context Exhaustion During E2E Testing
- The amount of logging and number of operations for a single basic e2e test — calling a stub
  claude binary and watching the beads — exhausted Opus's context before the job was completed
- This was a single basic test, not a complex scenario
- Regardless of how the author informed on filter commands for watchers, it wasn't enough
- Spent 3 Max sessions trying to get it working
- E2E tests require careful orchestration and system configuration — the kind of multi-system
  coordination that generates enormous context even for simple cases
-->

<!-- OUTLINE: Claude Code Subagent OOM
- Claude Code is a JS/Bun binary
- When using Claude's built-in agent delegation, it reads the conversation of the subagents back
- This caused OOM — the subagent conversations were too large to read back into the parent context
- A different manifestation of the same capacity problem: not just context window compaction,
  but literal memory exhaustion from trying to hold agent output
-->

<!-- OUTLINE: The 6-Phase Structure (v0.3)
- The v0.3 implementation was broken into 6 explicit phases
- This was NOT primarily a Claude Code workaround — it was designed to facilitate a generally
  useful flow for correct implementation of software
- But the phasing did help keep individual work units within Claude Code's effective context window
- Design docs in docs/v0.3.0-implicit-fix/ for each phase
-->

<!-- OUTLINE: The Resolution
- After 3 Max sessions, the author ran the tests themselves
- Claude shifted to a support role: building the e2e test environment setup rather than driving
  the testing
- E2E env as a separate repo: https://git.sr.ht/~ninjapanzer/catalyst-orchestration-e2e-env
- The tool is useful when scoped appropriately — but e2e testing of a multi-process daemon
  exceeded what it could hold
-->

<!-- OUTLINE: Reflection
- Context exhaustion is a hard ceiling, not a quality problem
- The gap between "Claude can write code" and "Claude can hold a complex system in its head
  long enough to test it end-to-end"
- The OOM problem suggests the architecture of Claude Code itself has capacity limits beyond
  just the model's context window
- Link to catalyst article: /i-made-a-thing/catalyst-orchestrator
- This is Part 1 of a series — other failure patterns exist (see Part 2: Timing Blindness)
-->

---

<!-- REMARKS FOR NEXT SESSION — context for continuing this article:

## Project Context
- catalyst-orchestrator: Go CLI daemon that orchestrates autonomous workflows for beads issues
- Repo: /home/paulscoder/repos/catalyst-orchestrator
- 3 versions: v0.1 (Haiku gate orchestrator), v0.2 (STEP-RESULT deterministic routing),
  v0.3 (implicit fix loop with dynamic step creation)
- E2E test env: https://git.sr.ht/~ninjapanzer/catalyst-orchestration-e2e-env

## Key Details from Interview
- Original orchestration had no flow control — goroutines blasted on failure, burned a Max
  window in ~3 minutes
- Single basic e2e test (stub-claude + bead watching) exhausted Opus context
- 3 Max sessions spent trying to get e2e tests working with Claude driving
- Filter commands for watchers didn't help enough
- Claude Code subagent delegation OOMed — JS/Bun binary reading subagent conversations back
- The 6-phase v0.3 structure was primarily for good software implementation flow, not a
  Claude Code workaround (though it helped with context)
- Resolution: author ran tests themselves, Claude built the env setup in support role

## Series Context
- Part 1: Context Exhaustion (this article)
- Part 2: Timing Blindness (content/devex/where-the-edges-of-claude-code-fray-timing.md)
- Future parts TBD — enumerating failure patterns
- Potential terms-and-afflictions article about using blog-interviewer agent for article ideation

## Article Style
- Section: devex (developer experience)
- Tone: builder's journal — personal, reflective on the experience
- Related catalyst article: /i-made-a-thing/catalyst-orchestrator
- BATS article: /tech-dives/bats-testing-bash-like-you-mean-it

## Blog Format
- Frontmatter: TOML with title, template, weight, date, [extra] with desc, keywords, schema_type
- DevLog entries (if used): ### DD MM YYYY headings, #### subtitles, wrapped in
  <div class="devlog-entry"> blocks
- Author voice: conversational, mixes personal narrative with technical depth
- NEVER edit or reword author content — only assist with structure, metadata, presentation
-->
