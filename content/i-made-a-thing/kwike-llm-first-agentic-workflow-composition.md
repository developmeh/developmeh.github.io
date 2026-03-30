+++
title = "kwike: LLM-First Agentic Workflow Composition"
template = "page.html"
weight = 0
date = 2026-03-07
updated = 2026-03-29
[extra]
desc = "An LLM-first tool for composing agentic workflows using Unix primitives - pipes, append-only logs, and event subscriptions instead of SDKs and harnesses."
keywords = "event-driven, reactive flows, LLM orchestration, kwike, eventing, CLI, Go, durable events, Unix philosophy, agentic workflows, claude-code"
sitemap_priority = "0.8"
discussion_number = 51
discussion_url = "https://github.com/orgs/developmeh/discussions/51"
+++

I've been building [kwike](https://git.sr.ht/~ninjapanzer/kwike), an LLM-first tool for composing agentic workflows using Unix primitives - pipes, append-only logs, and event subscriptions instead of SDKs and harnesses.

The primary use case is agentic workflows for specific repeatable actions. What I often call drudgery. Version upgrades, dependency management, maintaining documentation, syncing sequence diagrams when HTTP endpoints change, resolving test failures with context.

This isn't going to run your business or make you the next 50MM-in-12hr guy, but it does allow you to get boring stuff done while you work on the stuff you like. A technical solution for technical problems.

---

## LLM First

One of the features kwike focuses on is training an LLM in its use. While it has a CLI you can use for non-agentic tooling, building a workflow is telling your LLM of choice - in my case, Claude - "You have access to kwike, an agentic workflow orchestration tool. Read `kwike --help` and `kwike docs`. I want to build a workflow that looks something like X. Let me know when you are ready to discuss."

Much of what comes next can be constructed without any experience with the tool. It includes debugging tools and static analysis to validate configuration and the workflow DAG.

While setting up a new workflow isn't immediate, rather complex behavior can be recorded and committed with your projects.

## Core Primitives

Deep down this is an agent-to-agent communication protocol through event dispatch and subscription. The tool exposes three primitives:

- **dispatch** - emit events to an event store
- **consume** - pull events you've subscribed to
- **daemon** - event store owner, can mesh across network boundaries

The daemon persists events to an append-only JSON-lines file. Producers dispatch events through stdin or a watch command that polls arbitrary commands on an interval. Consumers pull events, render Go templates against the event payload, and execute LLM subprocesses with the rendered prompt.

```
kwike daemon --http :4444
kwike watch "git diff HEAD" --type ci.diff --interval 30s
kwike dispatch --type review.requested < payload.json
kwike consume --config reviewer.yaml --once
```

## Uniforms and Contracts

Each workflow step is called a **uniform** and is assigned to a dedicated consumer process. A uniform is a prompt template passed to the underlying tool the consumer executes.

The goal is to force the LLM to consume and produce a consistent contract so agents and scripts can exchange data. This protocol also provides some guarantee that agentic processes complete their work by requiring a JSON schema which validates the agent output.

The consumer watches for one or more message types, defines its own partition, and maintains its own read cursor. Think of it as allowing system events or another LLM's tool use to spawn a dedicated workflow.

In many cases this becomes a context exchange between agents - reminding working directories or fan-out/fan-in statuses. This context exchange is generally analogous to an email thread. A consumer can subscribe to many sources and can force a clean context or continue a session:

```yaml
source:
  types:
    - "task.implement"      # subscribed events
    - "task.review.rejected"
    - "task.review.approved"

session:
  fresh_types: [task.implement]        # these create new sessions
  resume_types: [task.review.rejected] # these resume existing sessions
  # task.review.approved is neither → defaults to fresh
```

Each tool call or agent can map lifecycle events so the output contract can inform on `.done` and `.failed`. When these mappings exist a successful tool or agent completion results in a new event dispatched as a threaded reply, allowing other consumers to participate in the work with their own uniform defining a task.

## The Durability Guarantee

I can't guarantee the LLM performs correctly. It's all based on prompt tuning and sometimes luck. But I can guarantee that if the action should happen, the agent gets the message.

There's always an execution, always a log, always feedback. And I can identify whether the LLM performed the expected operation and retry if it didn't. The durability is in the pipeline, not the output.

The mental model is a newsgroup server, not a message broker. Messages are public. Consumers pull what they want. Copy locally for later. The server doesn't care if you read or not. The messages are there.

## The Unix Thing

This wasn't accidental. I explicitly followed Eric Raymond's 17 Unix Rules from *The Art of Unix Programming* and Mike Gancarz's Unix Philosophy tenets.

JSON-lines is "store data in flat text files." The four subcommands are "make each program do one thing well." Piping dispatch from stdin is "make every program a filter." The daemon over Unix sockets is "write transparent programs." `--dry-run` is "write programs which fail in a way that is easy to diagnose."

Small is beautiful. Build modular programs. Use composition. Avoid unnecessary output.

These aren't principles I admire from a distance. They're the architecture document.

## Claude-Code Integration

Much of this has been built around claude-code and there are some limitations towards other agents right now. Claude-code supports session resume, which permits retries and replies to maintain a conversationally bound session to speed things up.

The consumer config can specify how different event types interact with sessions - some start fresh, some resume existing context. This matters for workflows where rejected reviews should continue the existing conversation rather than starting over.

## Where It Runs

This isn't a developer tool. It's infrastructure.

Consider a GitLab CI pipeline where module-specific consumers with tailored prompts live *in the codebase* alongside the code they're about. Not a generic "AI, update the docs." A specific consumer config for *that* module, with *that* prompt, that knows how to handle *that* class of file:

```yaml
dispatch:
  stage: collect
  script:
    - kwike dispatch --type "lint.result" < lint.json
    - kwike dispatch --type "test.result" < test.json
  artifacts:
    paths:
      - events.jsonl

review:
  stage: analyze
  needs: [dispatch]
  script:
    - kwike consume --config llm-review.yaml --once
```

`--once` means process events and exit. That's CI-native behavior.

Or deployed in a Lambda, spinning up instanced, routing based on the specific PagerDuty issue, delivering the right prompt for the right class of incident. Or as a k8s pod watching Prometheus for alarm conditions. Or as a sidecar to any application, watching logs for specific conditions.

It runs where the events are, not where the developer is.

## Quick Example

Watch a git repo, dispatch changes, consume with an agent. Requires a running kwike daemon.

```bash
# watch a git repo, dispatch changes, consume with an agent
kwike watch "git diff --stat HEAD~1" --type repo.change --interval 60s
kwike consume --config ./agents/writer/consumer.yaml
```

## Longer Example: Document Processing Pipeline

Here's something that has nothing to do with code. A small business receives documents - invoices, contracts, correspondence - dropped into a shared folder. The workflow: watch for new files, classify them, extract structured data based on type, and produce a weekly digest.

```
                         +------------------+
                         |   doc.received   |  <-- watch command
                         +--------+---------+
                                  |
                                  v
                         +------------------+
                         |    classifier    |
                         +--------+---------+
                                  |
                    +-------------+-------------+
                    |                           |
                    v                           v
           +----------------+          +-----------------+
           |  doc.classified |         | doc.classify    |  [TERMINAL]
           +-------+--------+          | .failed         |
                   |                   +-----------------+
                   |
     +-------------+-------------+-------------+
     |             |             |             |
     | filter:     | filter:     | filter:     |
     | invoice     | contract    | corresp.    |
     v             v             v             |
+----------+ +----------+ +-----------+        |
| invoice  | | contract | | corresp.  |        |
| extractor| | extractor| | extractor |        |
+----+-----+ +----+-----+ +-----+-----+        |
     |            |             |              |
     |    +-------+-------+     |              |
     |    |               |     |              |
     v    v               v     v              |
+-----------+          +-----------+           |
| .extracted|          | .extract  | [TERMINAL]|
| events    |          | .failed   |           |
+----+------+          +-----------+           |
     |                                         |
     +-------------+-------------+             |
                   |                           |
                   v                           |
          +------------------+                 |
          |    [TERMINAL]    |                 |
          |  markdown output |                 |
          +------------------+                 |
```

```bash
# daemon running somewhere
kwike daemon

# watch a folder for new PDFs
kwike watch "find /shared/inbox -name '*.pdf' -mmin -5" \
  --type doc.received --interval 5m
```

The classifier consumer subscribes to `doc.received` and outputs a structured classification:

```yaml
# classifier/consumer.yaml
source:
  types: ["doc.received"]

uniform:
  prompt: |
    You are a document classifier. Given the following document,
    classify it as one of: invoice, contract, correspondence, other.

    Document: {{.payload.path}}

    Output JSON: {"type": "...", "confidence": 0.0-1.0, "summary": "..."}

output:
  schema:
    type: object
    required: [type, confidence, summary]
    properties:
      type: {enum: [invoice, contract, correspondence, other]}
      confidence: {type: number}
      summary: {type: string}

lifecycle:
  done: "doc.classified"
  failed: "doc.classify.failed"
```

Now three more consumers, each subscribing to `doc.classified` but filtering on type:

```yaml
# extractors/invoice/consumer.yaml
source:
  types: ["doc.classified"]
  filter: '.payload.type == "invoice"'

uniform:
  prompt: |
    Extract invoice data from this document.

    Document: {{.payload.path}}

    Output JSON with: vendor, amount, due_date, line_items

output:
  schema:
    type: object
    required: [vendor, amount, due_date]
    properties:
      vendor: {type: string}
      amount: {type: number}
      due_date: {type: string}
      line_items: {type: array}

lifecycle:
  done: "invoice.extracted"
```

Fan-out at classification. Fan-in at digest. Each step validates its output against a schema. If the invoice extractor fails schema validation, it retries. If it keeps failing, `invoice.extract.failed` gets dispatched and a human can intervene. The documents themselves never leave the folder - only structured data flows through the pipeline.

Plenty of examples in the repo: [https://git.sr.ht/~ninjapanzer/kwike](https://git.sr.ht/~ninjapanzer/kwike)

There's plenty left to do but I'd love to hear feedback on the architecture and assumptions around contract constraints as a mode to make LLMs behave. The crash-only approach, and the message durability that mirrors Kafka but acts more like a newsgroup server and mail clients.

---

## DevLog

<div class="devlog-entry">

### 29 03 2026
#### v0.0.15 - Refining Options

Renamed llm config to tool since this can run any command with the caveat that resume_types are limited to claude exections. Probably need some way to create a validation for this.

</div>

<div class="devlog-entry">

### 15 03 2026
#### v0.0.5 - Making tools for robots

Kwike is not really designed to be used by humans, its rather complicated. Since its been designed as a tool robots use its cli is intended to instruct process and carry a lot of documentation. Its an interesting problem and the question is, does something like this inform on how to build for humans too.

</div>
