+++
title = "Where the Edges of Claude Code Fray: Timing Blindness"
template = "page.html"
draft = true
weight = 0
date = 2026-02-15
[extra]
nav_title = "Claude Code Fray 2: Timing Blindness"
desc = "On Claude's consistent inability to reason about timing, race conditions, and sequencing across system boundaries — a pattern that repeats across projects and models."
keywords = "claude code, timing, race conditions, e2e testing, webrtc, sequencing, devex, AI limitations"
schema_type = "BlogPosting"
sitemap_priority = "0.8"
bluesky_tags = "dev, claude, testing, devex, timing"
+++

<!-- SERIES: "Where the Edges of Claude Code Fray"
Part 1: Context Exhaustion (content/devex/where-the-edges-of-claude-code-fray.md)
Part 2: Timing Blindness (this article)
Future: TBD — enumerating failure patterns, some with solutions, some without
-->

<!-- OUTLINE: Intro
A pattern that shows up across projects and model versions: Claude cannot reason about timing,
race conditions, and sequencing at the intersection of multiple systems. This isn't a context
window problem — it's a category of work where Claude consistently fails to deliver.

This feels like "cognitive overload" for the model — and is probably a good indicator of how
this architecture is not general intelligence.
-->

<!-- OUTLINE: The Pattern
- At the intersection of multiple items that need a concept of timing, race conditions, and
  e2e testing, Claude generally fails to deliver
- This is consistent regardless of agent or model version
- It's not about code generation ability — Claude can write the individual pieces. It's about
  understanding how those pieces interact across time and system boundaries
- Three manifestations of the same underlying gap:
  1. Protocol sequencing — can't order async handshake steps correctly (krappy-tunnel)
  2. Execution order — can't reason about chronological order of operations from logs (catalyst)
  3. Temporal variability — can't model that execution time varies and biases to wrong
     defaults (beads-orchestrator)
-->

<!-- OUTLINE: Case 1 — WebRTC Protocol Sequencing (krappy-tunnel)
- Project: krappy-tunnel (https://git.sr.ht/~ninjapanzer/krappy-tunnel)
  - A self-hosted STUN/TURN WebRTC implementation in WASM
  - Two participants establish a direct connection via NAT hole punching
  - Part of the larger Krappy Internet ecosystem (link: /projects/krappy-internet)
  - This component was a blocker for the whole ecosystem
- Reference implementation: https://blog.printf.net/articles/2013/05/17/webrtc-without-a-signaling-server/
- The protocol is well-documented, not novel — ICE/STUN/TURN handshake is textbook
- Claude was unable to establish the protocol with the right order
- This isn't a knowledge gap — WebRTC has extensive RFCs, docs, examples in training data
- The failure is in reasoning about the sequencing: which async operations must complete before
  the next can start, readiness signals, the state machine of the handshake
- Because this was a blocker, the downstream Krappy Internet work stalled on this failure
-->

<!-- OUTLINE: Case 2 — E2E Testing catalyst-orchestrator
- E2E tests for the daemon required careful orchestration and system configuration
- Multiple processes: daemon, stub-claude, beads CLI, BATS test runner
- Each has its own timing: daemon polls, agents run, beads state updates, watchers fire
- The daemon log is sequential; prompt debugging had chronological timestamps
- Plenty of hints about the chronological order of bead execution — from both logs and prompts
- Despite this, Claude would move check behavior in the router out of order, causing bugs
- It was clear the agent was struggling to understand the execution order
- The confusion compounded — out-of-order checks introduced further bugs and confusion
- This is distinct from the context exhaustion problem (Part 1): even with the information
  present, the ordering reasoning failed
- Link to catalyst article: /i-made-a-thing/catalyst-orchestrator
- Link to BATS article: /tech-dives/bats-testing-bash-like-you-mean-it
- Link to e2e env: https://git.sr.ht/~ninjapanzer/catalyst-orchestration-e2e-env
-->

<!-- OUTLINE: Case 3 — Polling Intervals (beads-orchestrator)
- Project: beads-orchestrator (the v0 predecessor to catalyst)
  - Link: https://git.sr.ht/~ninjapanzer/beads-orchestrator
  - The orchestrator agent managed subagents that each took variable amounts of time
- Author repeatedly prompted: poll every 15s, never longer than 30s
- Claude would consistently pick 10-minute intervals — completely slowing down workstreams
- Most agents completed in under 3 minutes
- Claude always biased toward longer and longer waits
- It didn't understand the concept of variability of time
- Similar to WebRTC: not a knowledge problem, but an inability to model that timing is
  variable and that short frequent checks are appropriate when execution time is unpredictable
-->

<!-- OUTLINE: Why This Is Different From Context Exhaustion
- Context exhaustion (Part 1) is a capacity problem — the tool runs out of room
- Timing blindness is a reasoning problem — Claude doesn't model temporal relationships
  between system boundaries even with plenty of context
- You can give Claude a fresh session with full documentation about timing requirements
  and it will still produce code that doesn't respect them
- This feels like "cognitive overload" — the model can handle individual pieces but can't
  hold the temporal relationships between them
- A good indicator that this architecture is not general intelligence: humans have an easier
  time letting go of details to assess time-based parts of operations that don't occur in
  consistent timelines
-->

<!-- OUTLINE: The Solution (and What It Says)
- The solution is to handle the ambiguous, time-dependent testing yourself
- Let Claude handle the "dumb parts" — building tooling, scaffolding, deterministic logic
- The human is better at reasoning about temporal variability and sequencing across system
  boundaries — letting go of details to assess the time-based behavior
- This redraws the line of what AI-assisted development can and can't do:
  - CAN: write individual components, build test infrastructure, generate deterministic logic
  - CAN'T: reason about how those components interact across time
- What categories of work this makes unreliable with current AI assistance
-->

---

<!-- REMARKS FOR NEXT SESSION — context for continuing this article:

## Key Details from Interview
- Pattern observed across multiple projects, not just catalyst
- WebRTC: timing between network calls ignored despite documentation
- E2E testing catalyst: multiple systems with their own timing (daemon, stub-claude, beads, BATS)
- Consistent across agent and model versions
- This is distinct from context exhaustion — it's a reasoning gap, not a capacity limit
- Author has solutions for some failure patterns in the series, not all — TBD which category
  this falls into

## Key Details from Interview (Round 2)
- krappy-tunnel: self-hosted STUN/TURN WebRTC via WASM, NAT hole punching
- Reference: https://blog.printf.net/articles/2013/05/17/webrtc-without-a-signaling-server/
- Part of Krappy Internet ecosystem — tunnel was a blocker for downstream work
- Protocol is well-known, well-documented — not a knowledge gap
- Claude couldn't get the signaling order right
- Catalyst e2e: daemon logs were sequential, prompt debug had timestamps — ordering info was
  available but Claude moved router checks out of order, introducing bugs
- The ordering confusion compounded into further bugs

## Key Details from Interview (Round 3)
- Author's framing: this feels like "cognitive overload" for the model — a good indicator
  that this architecture is not general intelligence
- The solution: handle the ambiguous time-based testing yourself, let Claude build tooling
- Humans have an easier time "letting go of details" to assess time-based operations that
  don't occur in consistent timelines — Claude can't do this
- Third example: beads-orchestrator polling intervals
  - Subagents took variable time, most under 3 minutes
  - Author prompted repeatedly: poll every 15s, never longer than 30s
  - Claude consistently picked 10-minute intervals
  - Biased toward longer and longer waits — didn't understand temporal variability
  - Similar root cause to the WebRTC sequencing failure

## Still Needs Interview
- How the krappy-tunnel situation was ultimately resolved (or if it's still blocked)
- Any additional detail the author wants to add when writing

## Series Context
- Part 1: Context Exhaustion (content/devex/where-the-edges-of-claude-code-fray.md)
- Part 2: Timing Blindness (this article)
- Future parts TBD
- Potential terms-and-afflictions article about using blog-interviewer agent for article ideation

## Article Style
- Section: devex (developer experience)
- Tone: builder's journal — personal, reflective on the experience
- Author voice: conversational, mixes personal narrative with technical depth
- NEVER edit or reword author content — only assist with structure, metadata, presentation
-->
