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
keywords = "kwike, cando, agentic workflows, LLM agents, event-driven, Unix philosophy"
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
