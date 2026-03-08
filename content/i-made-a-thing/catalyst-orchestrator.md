+++
title = "Catalyst: An Orchestrator That Stopped Asking and Started Deciding"
template = "page.html"
weight = 0
date = 2026-02-15
[extra]
desc = "A devlog tracing catalyst-orchestrator through three architectural revisions — from LLM-driven gate decisions to deterministic routing to daemon-managed fix loops."
keywords = "catalyst, orchestrator, workflow automation, AI agents, Go, beads, deterministic routing, devlog"
schema_type = "BlogPosting"
sitemap_priority = "0.8"
bluesky_tags = "dev, orchestration, AI, golang, devlog"
+++

<!-- OUTLINE: Intro — The Two Problems

1. The beads-orchestrator (version 0, link: https://git.sr.ht/~ninjapanzer/beads-orchestrator) had Claude
   doing everything — orchestration, decision-making, all of it. Claude Code crashed reading subagent chat
   strings. It forgot to keep iterating. Memory issues. Timing issues.

2. Durability. Internet drops, Claude Max windows expire, sessions die. Needed something that waits,
   recovers, and resumes without babysitting.

Goal: Create tasks across multiple projects (3 at once), let the daemon pick them up and complete them
overnight. The JetBrains beads manager (link to "keep-your-eyes-on..." article) feeds tasks in.
Optimize productivity while you sleep.

Hook/story: First real unattended run with the beads-orchestrator hit a concurrency bug — spun up so many
Sonnet sessions it burned through a Claude usage window in about 3 minutes.
-->

---

## DevLog

<div class="devlog-entry">

### 11 02 2026
#### v0.1 — The Haiku Decides

The first version of catalyst was built around a simple idea: let a cheap, fast LLM handle the coordination. The daemon watched beads molecules for ready steps, spawned agents (Sonnet for implementation, Haiku for review, Opus for merging), and when something interesting happened — a review finished, a step failed, an agent got stuck — it packaged that event as a gate and shipped it over a Unix socket to a Haiku orchestrator.

Haiku's job was to interpret the situation and respond. "Review passed — merge or MR?" Haiku would answer `merge`. "Step failed — retry, skip, or abort?" Haiku would decide. The daemon was deliberately reactive. It didn't interpret agent output. It didn't make routing decisions. It just watched molecules, ran agents, hit gates, and waited for Haiku to tell it what to do.

The architecture looked like this:

```
┌─────────────────┐     NDJSON/socket     ┌─────────────────┐
│  Haiku          │◄────────────────────►│  catalyst       │
│  orchestrator   │                       │  daemon         │
│                 │  gate_waiting:        │                 │
│  - clears gates │  "review passed,      │  - watches      │
│  - picks beads  │   merge or mr?"       │    molecules    │
│  - delegates    │  ◄──────────────────  │  - runs agents  │
│    merge to     │  "merge"              │  - emits events │
│    Opus         │  ──────────────────►  │  - handles      │
└─────────────────┘                       │    gates        │
                                          └─────────────────┘
```

Agent output was unstructured prose. The daemon didn't parse it — Haiku did. Formulas defined the full workflow explicitly: `implement → review → fix → merge`, with gates as decision points between steps. Every transition required Haiku's blessing.

This worked. But it had a cost. Haiku was interpreting free-form text to make routing decisions that were, in practice, completely predictable. "Review passed" always meant merge. "Review failed" always meant fix. The orchestrator was spending tokens to arrive at conclusions the daemon could have reached by parsing a status field.

</div>

<div class="devlog-entry">

### 13 02 2026
#### v0.2 — The Daemon Parses, The Daemon Routes

v0.2 was a philosophical inversion. Instead of the daemon asking Haiku what to do, agents were told to output a machine-parseable block, and the daemon was taught to read it.

The STEP-RESULT protocol replaced unstructured prose:

```
---STEP-RESULT---
STATUS: DONE
VERDICT: APPROVED
SUMMARY: Implementation meets all acceptance criteria
INSTRUCTIONS:
- Minor: consider adding a timeout to the HTTP client
---END-RESULT---
```

The daemon's new `StepResultParser` extracted structured fields. The `StatusRouter` made deterministic decisions based on what it found:

```
STATUS: DONE
  ├── Reviewer? Check VERDICT
  │   ├── APPROVED → enable merge step
  │   └── REJECTED → enable fix step, pass INSTRUCTIONS downstream
  └── Otherwise → advance DAG

STATUS: BLOCKED → mark bead blocked
STATUS: ERROR   → mark bead blocked
```

No LLM interpretation. No token spend on routing. The daemon read the result and knew where to go.

This version also introduced automatic retry with Opus escalation — if an agent produced malformed output (missing the STEP-RESULT block), the daemon retried up to 5 times with the original model, then escalated to Opus. If even Opus couldn't produce a parseable result, the bead was marked blocked. All retry events were logged to bead comments for auditability.

Agent prompts moved from hardcoded Go strings to an external TOML template file (`agent_prompts.toml`), with Go template variables (`{{.BeadID}}`, `{{.Description}}`, `{{.ReviewInstructions}}`) injected at runtime. Review instructions from the reviewer's INSTRUCTIONS field flowed downstream to the fixer and merge agents via bead comments — the reviewer could say "fix the nil pointer in auth.go:45" and the fixer would see that in its prompt.

The formula still defined `implement → review → fix → merge` as explicit steps. The fix step always existed in the workflow, even when the review approved and it was never needed.

</div>

<div class="devlog-entry">

### 14 02 2026
#### v0.3 — The Daemon Creates Steps at Runtime

v0.3 asked: if the daemon is already making the routing decisions, why does the fix step need to exist in the formula at all?

The answer was that it didn't. In v0.3, the fix step was removed from every formula. The workflow became `implement → review → merge`. When the reviewer output `VERDICT: REJECTED`, the daemon dynamically created a fix step in beads storage, spawned a fixer agent, and when the fix completed, reset the review step to `open` so it would re-run.

```
implement → review ←──────────────┐
              │                   │
    ┌─────────┴─────────┐        │
    │                   │        │
 APPROVED            REJECTED    │
    │                   │        │
    ▼                   ▼        │
  merge           create fix     │
                  (dynamic step) │
                        │        │
                   fix runs      │
                        │        │
                   fix DONE ─────┘
```

This loop could run up to 3 times. Iteration count was tracked via bead comments (`[daemon] FIX_ITERATION: N`). After 3 rejections, the bead was marked BLOCKED — the daemon decided the implementation couldn't be salvaged through automated fixes.

The key design insight was crash recovery. Fix steps weren't held in daemon memory — they were persisted as real beads issues. If the daemon crashed mid-fix, it would restart, scan for in-progress molecules, find the fix step by its title pattern (`Fix: <beadID>`), and resume processing. The existing step identification logic (`extractFormulaStepID()`) already recognized the "Fix:" prefix, so dynamically created steps were processed identically to formula-defined ones.

The review step stayed `in_progress` during the fix loop rather than being closed and reopened. This avoided a tricky state transition (beads didn't naturally support closed → open) and made conceptual sense — the review represented "evaluate this implementation," and that evaluation wasn't complete until the code either passed or exceeded the iteration limit.

This version also introduced `stub-claude` for deterministic end-to-end testing. Instead of running real Claude against real code, test scenarios defined expected agent sequences: "reviewer rejects once, then approves" should produce exactly 6 agent invocations (refine, implement, review, fix, review, merge). This made the implicit fix loop testable without burning API tokens.

</div>

---

## The Arc

<!-- OUTLINE: The Arc / Reflection — threads to weave together:

1. Proving Sonnet/Opus wrong about Haiku — They said Haiku couldn't orchestrate. It could, but its job
   was better expressed as a DAG with error routing to Opus. The insight wasn't that Haiku was bad — it
   was that the orchestration decisions were predictable enough to be code.

2. Moving things into code — Recurring theme across all 3 versions. The fix step was in the formula
   (v0.1, v0.2), then it wasn't (v0.3). You keep pulling decisions out of LLM interpretation and into
   deterministic logic. Not because LLMs can't do it, but because when the answer is predictable, code
   is cheaper and more reliable. An implementer always gets a review, a review might always need a fix.

3. The daemon as a patient worker — The whole point is durability. It waits for internet. It waits for
   Claude Max windows. It survives crashes. The fix loop persists in beads storage. This isn't about AI
   autonomy — it's about building something that doesn't need you awake to keep working.
-->

<!-- OUTLINE: What's Next

- Session optimization: Each step is a fresh Claude session — expensive. Agents should assess task
  complexity and choose Sonnet vs Opus accordingly.

- Inferred permissions: Move away from yolo mode. Infer project and task permissions, inject them per
  session to reduce dangerous hallucinations.

- DAG visualization: See the daemon's decision-making as it routes work. Let users flag items for manual
  review and make mid-stream changes for better control.
-->
