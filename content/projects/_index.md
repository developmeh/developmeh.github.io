+++
title = "Projects"

description = "Interactive project demos and documentation"

draft = false
sort_by = "weight"
weight = 1
template = "section.html"
paginate_by = 0
insert_anchor_links = "none"
in_search_index = true
render = true
transparent = false
generate_feeds = false

[extra]
desc = "Interactive demos and documentation for my open source projects"
keywords = "kwike, cando, beamlet, wavelet, agentic workflows, LLM agents, event-driven, Unix philosophy, WebAssembly, capability security, multi-agent coordination"
+++

Interactive sites and documentation for projects I'm building.

---

<div class="project-card">

## [Kwike](/kwike/)

**Composable Event-Driven Agent Orchestration**

An LLM-first tool for composing agentic workflows using Unix primitives - pipes, append-only logs, and event subscriptions instead of SDKs and harnesses.

- Event-driven architecture with durable message delivery
- Four Unix-style primitives: `daemon`, `dispatch`, `watch`, `consume`
- Uniform contracts with JSON schema validation
- Session resume and crash recovery built-in

[View Interactive Site →](/kwike/) | [Source](https://git.sr.ht/~ninjapanzer/kwike)

</div>

---

<div class="project-card">

## [Cando](/cando/)

**Task-Oriented AI Agent**

A task-oriented LLM agent built in Crystal. Spawned for a task, does the work, exits.

- Runs Gemma 4 locally via Ollama with native tool calling
- Bash execution and file reading with structural awareness
- Context tracking and compaction across turns
- Not a chatbot - process-oriented task completion

[View Interactive Site →](/cando/) | [Source](https://git.sr.ht/~ninjapanzer/cando)

</div>

---

<div class="project-card">

## [Beamlet](/projects/beamlet/)

**A Minimum Viable OTP over WebAssembly**

Supervised, capability-sandboxed processes on wazero, in Go. The substrate for a self-rewriting LLM harness.

- OTP supervision semantics — restart policies, intensity windows, hot code loading
- Capability isolation as well as fault isolation: the zero-value grant is pure compute
- A harness where the model has five meta-tools and must author every other ability as sandboxed WASM
- Dependencies are a permission — `GOPROXY=off` builds plus a per-tool import policy

[Read more →](/projects/beamlet/) | [Source](https://git.sr.ht/~ninjapanzer/beamlet)

</div>

---

<div class="project-card">

## [Wavelet](/projects/wavelet/)

**Google Wave's Data Model as Multi-Agent Coordination**

A conversation store that is naturally forked, replayable, and cheap to assemble LLM context from.

- wave → wavelet → blip, without operational transformation or character-level sync
- Private forks as a data model rather than an orchestration script
- Per-call context assembly with rolling compaction for deep forks
- Provenance for free: which agent asserted what, when, in response to what

[Read more →](/projects/wavelet/) | [Source](https://git.sr.ht/~ninjapanzer/wavelet)

</div>
