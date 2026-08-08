+++
title = "Kwike"
template = "page.html"
weight = 1
date = 2026-03-26
[extra]
desc = "Composable event-driven agent orchestration using Unix primitives"
keywords = "kwike, event-driven, agent orchestration, Unix philosophy, LLM, Claude, automation"
discussion_number = 53
discussion_url = "https://github.com/orgs/developmeh/discussions/53"

[taxonomies]
topics = ["Agentic AI", "Agent Orchestration", "Event Streaming", "Go"]
+++

**[View Interactive Site →](/kwike/)**

Kwike is an LLM-first tool for composing agentic workflows using Unix primitives - pipes, append-only logs, and event subscriptions instead of SDKs and harnesses.

## Core Primitives

- **daemon** - event store owner, persists to append-only JSON-lines
- **dispatch** - emit events to the store
- **watch** - poll commands on interval, dispatch on change
- **consume** - pull subscribed events, render templates, execute tools

## Key Features

- **Uniforms** - prompt templates with JSON schema validation
- **Session management** - fresh vs resume based on event type
- **Crash recovery** - cursor-based replay, idempotent processing
- **Fan-out/fan-in** - collector pattern for parallel workflows

## Links

- [Interactive Architecture Site](/kwike/)
- [Source Code](https://git.sr.ht/~ninjapanzer/kwike)
- [Devlog](/i-made-a-thing/kwike-llm-first-agentic-workflow-composition/)
