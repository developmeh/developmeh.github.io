+++
title = "Wavelet"
template = "page.html"
weight = 4
date = 2026-08-07
updated = 2026-08-07
[extra]
schema_type = "TechArticle"
desc = "Google Wave's data model repurposed as a coordination substrate for multiple chat agents - forked conversations, cursor-based wake semantics, and deterministic targeted delivery that keeps an agent's context about what concerns it"
keywords = "wavelet, Google Wave, multi-agent coordination, agent orchestration, MCP, Model Context Protocol, targeted delivery, attention layer, context engineering, wake semantics, turn semantics, conversation store, blackboard architecture, provenance, forked conversation, Go, tmux, Claude Code"
categories = "Projects"
discussion_number = 56
discussion_url = "https://github.com/orgs/developmeh/discussions/56"

[taxonomies]
topics = ["Agentic AI", "Agent Orchestration", "Context Engineering", "Go"]
+++

Wavelet takes Google Wave's data model and repurposes it as a coordination substrate for multiple chat agents.

It is not a Wave clone. It keeps the structural model — **wave → wavelet → blip** — and discards what made Wave hard: operational transformation, character-level sync, and the everything-is-live UX. What remains is a conversation store that is naturally forked, replayable, and cheap to assemble LLM context from.

## The data model

A conversation involving multiple agents is a forked graph, not a linear transcript.

- **Wave** — the shared world-state for a task or session. The durable artifact.
- **Wavelet** — a scoped sub-conversation with its own participant list. Two agents hashing out a subproblem fork a wavelet; only the conclusion surfaces to the parent. Private wavelets are invisible to non-participants — the "subagent scratch context" pattern expressed as a *data model* rather than an orchestration script.
- **Blip** — an individual message or assertion, with authorship and history. Blip-level append/replace only; no mid-keystroke merging.

Rosters start at fork time and mutate by `invite`/`leave` — replayable log events with system blips for provenance. Leaving stops deliveries mechanically rather than by convention.

## Channels: binding a session to a wave

A **channel** is an MCP-over-stdio server that turns a Claude CLI session into a wave participant. It polls the daemon, delivers other participants' blips as channel notifications that wake the session, and exposes the participant tools — `wavelet_post`, `wavelet_read`, `wavelet_list`, `wavelet_fork`, `wavelet_surface`, `wavelet_invite`, `wavelet_leave`, `wavelet_ping`.

The important detail is that **the session's identity is bound by the channel, not by its history.** The instructions open with "You are `<participant>`" and claim prior blips under that name as the session's own. That is what makes wrapping a `--resume`d session safe: a resumed operator session picks up where it left off instead of replaying history.

The whole wire surface is hand-rolled stdlib Go — newline-delimited JSON-RPC 2.0, no MCP SDK, no HTTP router. One binary, flat `package main`, `vendorHash = null`. The cost is owning protocol conformance; the benefit is that the entire surface is auditable in one file and there is no dependency churn.

## Non-channel transport: waking without notifications

Sessions on third-party providers — Bedrock, for instance — have no development-channel surface, so they cannot be woken by MCP notifications at all. They *can* be re-invoked when a background process completes, so `wavelet watch` becomes that process: it blocks until at least one new blip lands, prints everything since the cursor, emits an eval-able `after=` re-arm line, and exits. **Completion is the wake.**

`-hook` mode adapts the same loop to a Claude Code `asyncRewake` hook, which re-arms on every turn end and has no caller to thread a cursor through. Three things change:

- Position is read from and committed to the channel's *own* state file. Because both transports share one file, a session that falls back from channels to the hook resumes exactly where it left off — no replay, no gap.
- A lock beside that file keeps concurrent arms from racing; a second arm exits 0 immediately.
- The exit code carries the signal: `2` means wake the session, `0` means nothing happened. A daemon unreachable for 30s also exits 2, because a hook that dies quietly stops delivery until the next turn — which on an idle session is never.

The cursor advances when blips are *emitted*, not on proof the session read them; nothing acks a hook. A session killed between wake and read loses that batch. Holding the cursor back instead would re-deliver the same blips on every arm, forever.

## Wake semantics and the turn budget

LLM participants are request/response. Wave had no turns. Something has to decide when a wavelet change triggers which participant's inference — and whatever you pick is also the token-burn control.

The current answer is **wake on any blip**: every non-self blip in a subscribed wavelet is delivered, bounded by three independent guards.

1. **Prompt contract** — turn budget, never post merely to acknowledge, finish with one `DONE:` blip and then silence.
2. **Channel instructions** — never reply to `system` or `surface` blips.
3. **A mechanical per-wavelet delivery cap** — one exhaustion notice, then silent cursor advance. A runaway pair is bounded at roughly 2× the cap regardless of what the models do.

Echo suppression is stateless: blips carry `author`, and the channel drops any blip whose author is the participant itself. Position is one rule everywhere — deliver blips with `id > cursor`, UUIDv7 so lexicographic order is time order. No ACKs. A failed push keeps the cursor, so blips retry rather than vanish.

The cursor is tracked **per wavelet**, not as a single global position. Because UUIDv7 ids sort by time, one global cursor advanced by a busy wavelet would step straight over older-but-unseen blips in a quiet one.

## Targeted dispatch: rules before models

Wake-on-any-blip injects every participant's full prose into every rostered session, and live fleet runs showed the cost compounding: role dilution as other agents' output dominates a session's context, compaction summarizing away the session's *own* role first, mid-task interruptions, degraded classification.

The reframing that fixes it: each participant is really an **assistant** — the session doing work — plus an **attention layer** deciding what reaches it.

A model-based attendant was designed first and rejected. Once the policy is "digest whatever isn't addressed, from the operator, or already distilled," no judgment call remains that justifies a subprocess, API cost, latency, and a failure-fallback path. So the attention layer is **deterministic rules in the channel**, opt-in per participant:

- **Full delivery** when the blip mentions the participant (word-boundary or `[[name]]`), is authored by the operator, or is `kind:"surface"`. Addressed, human, and already-distilled traffic is never summarized.
- **Everything else digests** — one frame per wavelet batch, one line per blip, blip ids in the frame meta. Nothing is silently dropped: the digest line is the hook that lets an agent pull full text and object, so peer refutation survives in attenuated form.
- Frames, not blips, count toward the turn cap. A failed batch leaves the cursor and retries whole.

The trade is explicit: a targeted participant's context grows with what concerns it rather than with roster chatter, at the cost that unaddressed brilliance arrives as one line instead of prose. Operators choose per agent — reviewers on the full feed, specialists targeted.

Addressing thereby becomes a *meaningful convention* — instructions now say to name the participant whose attention you need — without any wire or store change.

Every routing decision, and every read-after-digest miss, is logged to a client-local JSONL. That is deliberately the training corpus for a future learned attendant, which plugs in behind an external command as a **rescue-only, fallback-open** hook: any error, timeout, or bad verdict leaves the rule outcome standing.

## The operator is in-band

A failure mode the turn guards don't cover: the human types a follow-up directly into an agent's terminal. That input is out-of-band — not a blip, invisible to other participants, and unaddressed by the agent's prompt contract. Observed results included agents over-applying `DONE:` silence and one forking an unrequested wavelet.

Two rules resolve it. **Out-of-band input is operator control, never conversation** — agents answer it locally and never post, fork, or surface in response unless explicitly asked to relay, and then as one blip prefixed `operator asks:` so provenance stays visible. And **the operator's in-band voice is a wrapped session**: `wavelet claude` launches or resumes a session with the channel MCP server injected as inline config, so questions to the *group* arrive as ordinary `human` blips that wake everyone.

Interjection becomes symmetric with participation. Everything the group should react to exists as an attributable, replayable blip; anything typed into a single pane is explicitly private steering.

## Spawning a fleet

Spawning participants is a *terminal* concern, not a daemon concern. `wavelet spawn` materializes a wavelet's roster into tmux — one window per wavelet, one pane per participant — and tmux owns supervision: panes are visible, attachable, killable, and die with the session. The daemon never gains process management. `fork -spawn` composes fork-and-staff into one motion.

Cross-machine fleets spawn locally against a remote daemon, so panes live where the operator's tmux lives — which is where they can be watched.

## Operator console

`wavelet daemon -ui 127.0.0.1:8087` serves an embedded single-file console: wave tree, live transcripts over SSE, and the operator's in-band powers through the same participant API the agents use. Off unless the flag is given, loopback by default. Woken participants show as *working* until they post or a TTL lapses — ephemeral telemetry, never written to the log.

## Design commitments

**Storage stays a tree; cross-links are inferred.** Forking is the only structural primitive. General cross-references are never stored, because they can always be inferred at read time.

**Context assembly is cheap, so do it per call.** An agent's prompt context is assembled on demand by walking the fork's ancestor path. No pre-materialized contexts.

**Compacted context for long forks.** Each fork keeps a rolling summary, so a call's context is compacted ancestor chain plus the live tail. Deep forks stay cheap.

**Agents are robots.** Wave's robot protocol was already an agent protocol. There is no agent-to-agent RPC; everything is mediated through the shared conversation artifact — a structured, replayable blackboard.

**Provenance for free.** Every blip has an author and the wave has full history, so it is always answerable which agent asserted what, when, and in response to what.

## Open questions

Subscription semantics — what does the declaration language look like? Turn semantics beyond wake-on-any-blip: debouncing, quiescence detection, explicit hand-off markers. Surfacing rules: explicit op by a participant, or inferred? And compaction policy — when does it run, which model performs it, and how do you verify a compaction didn't drop a load-bearing fact?

## Links

- [Source Code](https://git.sr.ht/~ninjapanzer/wavelet)
- Reference ships as scdoc man pages — `wavelet(1)`, per-subcommand pages, and `wavelet(7)` for concepts
- Design decisions recorded as ADRs in `docs/adr/`
- Related: [kwike](/projects/kwike/) — a wave is a natural durable home for a shared agent session
