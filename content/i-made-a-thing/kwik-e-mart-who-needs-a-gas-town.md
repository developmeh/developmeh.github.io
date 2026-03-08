+++
title = "Kwik-E-Mart: Who Needs a Gas Town When a Gas Station Will Do"
template = "page.html"
weight = 0
date = 2026-03-07
[extra]
desc = "Comparing tools for building event-driven pipelines — NATS, Redis Streams, shell pipes, and Kwik-E-Mart — with a focus on what it takes to get durable, fan-out reactive flows feeding LLMs."
keywords = "event-driven, reactive flows, LLM orchestration, NATS, JetStream, Redis Streams, Kwik-E-Mart, eventing, CLI, Go, durable events, Gas Town, beads, Unix philosophy"
sitemap_priority = "0.8"
+++

<!-- INTERVIEW REMARKS:

## Key Details from Interview

- Origin: Inspired by beads' mail command which "doesn't really do anything" — imagined agents
  sending specific prompts to each other through mailboxes
- Gas Town (Steve Yegge's multi-agent orchestrator) was too big — "I don't need a town, a gas
  station is the right size for most of the agentic work I do"
- Targets small deterministic workflows on low throughput operations — frontier LLMs at high
  concurrent volume are expensive
- Expected beads to offer a mailbox that tracks agent sessions, allowing retrigger/resume with
  a new prompt and await response — that's what Kwik-E-Mart is
- Mental model: newsgroup server, not message broker — messages are public, consumers pull what
  they want, copy locally for later
- The problem: LLM supervision orchestration causes degradation — context length over time
  screws up attention, agents stop working accurately
- Design guarantee: if the action should happen, the agent gets the message. Success isn't
  guaranteed but execution, logging, and feedback always happen
- Can identify if LLM performed expected operation and retry if not
- CI use case: module-specific consumers with tailored prompts kept IN THE CODEBASE alongside
  the code they're about — not generic "AI update docs" but specific prompts per module/file class
- Use cases: keeping documentation up to date (sequence diagrams, API docs synced when endpoint
  code changes), intercepting PagerDuty webhooks for log research, resolving/explaining test
  failures
- Deployment: lambda, k8s pod, sidecar, CI runner — runs where the events are, not where the
  developer is. These things don't run on developer systems.
- Sidecar pattern: deployed next to any application watching for specific log conditions
- Scale argument: NATS/Kafka/Redis aren't wrong, they're overscaled. Too much setup and
  resources for problems that need something compact
- Explicitly followed Eric Raymond's 17 Unix Rules and Mike Gancarz's Unix Philosophy tenets
- The emotional core: "We are in this very stupid cycle of talking to LLMs for all activities.
  We should be delegating and automating. Repeatability gives us productivity and reduces drudgery."
- The thesis: "Let the unsleeping robot do it 85% good enough. We can always clean it up but
  otherwise we get nothing."
- Documentation example: "just having sequence diagrams updated or API docs synced when http
  endpoint code changes is like magic"

## Manson Essayist Structure

Inverse thesis — DON'T open with the comparison or the tool. Open tangential.

Thread 1: The drudgery — boring tasks don't get done, 85% > 0%
Thread 2: Agent degradation — LLMs supervising LLMs eat their own context
Thread 3: Scale mismatch — towns, cities, gas stations
Thread 4: Unix philosophy — Raymond, Gancarz, this is deliberate architecture
Thread 5: The comparison — NATS, Redis, Kafka, pipes, Kwik-E-Mart — placed, not ranked

The thesis emerges: delegate to the unsleeping robot, give it the right prompt,
guarantee the attempt not the outcome.
-->

There's a command in [beads](https://github.com/steveyegge/beads) called `mail`. It doesn't really do anything. I stared at it for longer than I should have.

I was knee-deep in [catalyst-orchestrator](@/i-made-a-thing/catalyst-orchestrator.md) at the time, watching Haiku make routing decisions that were completely predictable, burning tokens to arrive at conclusions a status field could have told me for free. And I kept thinking about that mail command. What if it *worked*? What if agents could send each other specific prompts — not chat, not supervision, just: "here's the job, here's exactly how to do it, go."

But that's not what we do. What we do is open a session, type instructions into an LLM, watch it work for a while, watch it start to drift, watch its attention degrade as the context fills up, and then either restart the whole thing or just accept the garbage output because we're tired of babysitting.

Secret time: I was tired of babysitting.

---

## The Drudgery Problem

Here's what actually happens with documentation. Nobody updates the sequence diagram. Nobody syncs the API docs when the HTTP endpoint changes. Not because they don't care — because they're busy, or they forgot, or the PR was already approved and who's going back to add a diagram now?

The alternative to 85% good enough isn't 100%. It's zero.

We are in this very stupid cycle of talking to LLMs for all activities. Opening sessions. Crafting prompts in real time. Watching the robot work. Correcting the robot. Watching it again. That's not automation. That's a slightly fancier version of doing it yourself, except now you're also managing the thing that's doing it.

Repeatability is what gives us productivity and reduces drudgery. Not cleverness. Not meta-orchestration. Not agents supervising agents in some fractal management structure that would make Dilbert weep. Just: when this thing happens, do this other thing, with this specific prompt, every time.

You should be picking up the conflicts now. We want automation but we keep building supervision. We want repeatability but we keep reaching for general-purpose tools that require us to be in the room.

---

## The Degradation Problem

I watched an agent lose its mind over a long session. Not dramatically — it didn't hallucinate monsters or start writing poetry. It just got... worse. Slowly. The way a person gets worse at their job at hour eleven of a twelve-hour shift.

LLM supervision orchestration has a structural problem: the orchestrator is itself an LLM. It's consuming context to manage context. Every decision it makes, every status check it interprets, every routing choice — that's all context window. And context window is attention. And attention degrades over length. The longer the session runs, the less accurately the supervising agent works, which means the work it's supervising also gets less accurate, which means it has more problems to manage, which means more context consumed.

It's a death spiral with a credit card attached.

[Gas Town](https://github.com/steveyegge/gastown) solves this by being a *town* — 20 to 30 Claude instances coordinated by a Mayor, with Polecats and Refineries and Deacons. It's impressive engineering. It's also a town. I don't need a town. Most of the agentic work I do targets small deterministic workflows on low throughput operations. Frontier LLMs at high concurrent volume are expensive. I needed a gas station.

---

## What a Gas Station Looks Like

[Kwik-E-Mart](https://sr.ht/~ninjapanzer/Kwik-E-Mart/) is what happened when I stopped thinking about orchestration and started thinking about mail.

Not email. Newsgroups. NNTP. Remember newsgroups? Messages are public. You pull what you want. You copy what you want locally. Nobody's routing messages *to* you — you subscribe to what you care about and you read when you're ready. The server doesn't care if you read or not. The messages are there.

That's the mental model. A daemon persists events to an append-only JSON-lines file. Producers dispatch events through stdin or a watch command that polls arbitrary commands on an interval. Consumers pull events, render Go templates against the event payload, and execute LLM subprocesses with the rendered prompt. The consumer acknowledges when it's done. If it crashes, the event is still there.

```
kwike daemon --http :4444
kwike watch "git diff HEAD" --type ci.diff --interval 30s
kwike dispatch --type review.requested < payload.json
kwike consume --config reviewer.yaml --once
```

Four subcommands. Single binary. That's the whole thing.

I can't guarantee the LLM performs correctly. It's all based on prompt tuning and sometimes just luck. But I can guarantee that if the action should happen, the agent gets the message. There's always an execution, always a log, always feedback. And I can identify whether the LLM performed the expected operation and retry if it didn't. The durability is in the pipeline, not the output.

---

## The Unix Thing

This wasn't accidental. I explicitly followed Eric Raymond's 17 Unix Rules from *The Art of Unix Programming* and Mike Gancarz's Unix Philosophy tenets.

JSON-lines is "store data in flat text files." The four subcommands are "make each program do one thing well." Piping dispatch from stdin is "make every program a filter." The daemon over Unix sockets is "write transparent programs." `--dry-run` is "write programs which fail in a way that is easy to diagnose."

Small is beautiful. Build modular programs. Use composition. Avoid unnecessary output.

These aren't principles I admire from a distance. They're the architecture document.

---

## So Where Do the Big Tools Fit?

Here's the thing — NATS, Redis Streams, Kafka — they're not wrong. They're overscaled. They solve big problems at big scale with big infrastructure. Sometimes you need that. Most of the time, for this kind of work, you don't.

### Unix Pipes

The simplest version of reactive LLM flows is already in your shell:

```bash
fswatch -r ./content | while read file; do
  cat "$file" | llm "summarize this"
done
```

This works until your consumer crashes and misses events, or you want two consumers processing the same stream differently, or you want to replay history. Stdout is gone. There's no durability. There's no fanout. But for a one-off? Don't overthink it.

### NATS JetStream

General-purpose distributed messaging. Durable streams, consumer groups, subject-based routing, clustering. Battle-tested. Massive ecosystem.

```bash
nats stream add EVENTS --subjects "events.>"
nats consumer add EVENTS llm-reviewer --deliver all --ack explicit

# Consumer loop you write yourself
while true; do
  msg=$(nats consumer next EVENTS llm-reviewer --count 1 --timeout 30s)
  echo "$msg" | llm -s "you are a code reviewer"
done
```

The gap: every bit of LLM integration is your problem. You write the consumer loop, the retry logic, the concurrency limits, the backoff, the prompt rendering. It's a messaging system you build LLM workflows on top of. And you need a NATS server running.

### Redis Streams

If Redis is already in your stack:

```bash
redis-cli XADD events:file '*' path "$file" diff "$diff"
redis-cli XGROUP CREATE events:file reviewers '$' MKSTREAM
redis-cli XREADGROUP GROUP reviewers worker1 BLOCK 0 STREAMS events:file '>'
```

Similar trade-offs. Durable. Fanout via consumer groups. Simpler mental model than NATS. Same LLM gap.

### Kafka

I [wrote about recreating Kafka from scratch](@/i-made-a-thing/recreating-kafka-blind.md). The experience reinforced that Kafka's model is right for high-throughput distributed event streaming and way too much for everything I'm describing here. It's Java. It needs a cluster. Standing it up for "I want to watch some files and talk to an LLM" is like hiring a construction crew to hang a picture frame.

### The Table

|                        | Unix Pipes | NATS JetStream | Redis Streams | Kafka        | Kwik-E-Mart       |
|------------------------|------------|----------------|---------------|--------------|-------------------|
| Durability             | No         | Yes            | Yes           | Yes          | Yes               |
| Fanout                 | No         | Yes            | Yes           | Yes          | Yes               |
| LLM templating         | No         | No             | No            | No           | Yes               |
| Consumer config        | Shell      | Imperative     | Imperative    | Imperative   | Declarative YAML  |
| Watch → Event          | Manual     | Manual         | Manual        | Manual       | Built-in          |
| CI-friendly            | Sort of    | Needs server   | Needs server  | Needs cluster| `--once` `--dry-run` |
| Distributed            | No         | Yes            | Yes           | Yes          | Yes (HTTP mode)   |
| Deployment             | None       | Server + CLI   | Redis server  | JVM cluster  | Single binary     |

NATS and Kafka win on scale, ecosystem, and distributed infrastructure. They're designed for that. Kwik-E-Mart wins on "I need a single binary that watches things and feeds specific prompts to LLMs with zero infrastructure."

---

## Where It Actually Runs

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

`--once` means process events and exit. That's CI-native behavior. NATS consumers are designed to be long-lived. Kafka consumers definitely are. Kwik-E-Mart treats batch execution as a first-class mode because that's how CI works.

Or consider it deployed in a Lambda, spinning up instanced, routing based on the specific PagerDuty issue, delivering the right prompt for the right class of incident. Or as a k8s pod watching Prometheus for alarm conditions, helping define when PagerDuty should even be triggered. Or as a sidecar to any application, watching logs for specific conditions.

It runs where the events are, not where the developer is. These things don't run on developer systems.

---

## The Point

We keep building towns when we need gas stations. We keep opening chat sessions when we need mailboxes. We keep supervising robots when we should be delegating to them.

The unsleeping robot will do it 85% good enough. We can always clean it up. But otherwise? We get nothing. The sequence diagram stays stale. The API docs drift. The test failure sits unexplained until a human has time to look, which is never.

Just having sequence diagrams updated or API docs synced when HTTP endpoint code changes is like magic. Except it's not magic. It's a JSON-lines file, four subcommands, and a prompt that knows what it's looking at.

> "Small is beautiful." — Mike Gancarz, *The UNIX Philosophy*, 1994

<!-- REMARKS FOR NEXT SESSION:

## Status
- First draft complete from blog interviewer session
- Manson essayist inverse thesis structure applied:
  - Opens with beads mail command (tangential)
  - Weaves drudgery, degradation, scale, and Unix philosophy threads
  - Thesis emerges at the end: delegate to the unsleeping robot, 85% > 0%
- Comparison section covers Unix pipes, NATS, Redis Streams, Kafka, Kwik-E-Mart
- CI and deployment section covers GitLab, Lambda, k8s, sidecar patterns

## Verify with Author
- [VERIFY: Is the catalyst-orchestrator timeline right — was beads mail inspiration during that work?]
- [VERIFY: The Gas Town description — "20 to 30 Claude instances" — is that accurate to your experience or just marketing?]
- [VERIFY: The newsgroup/NNTP analogy — does that land the way you want it to?]
- [VERIFY: Any specific module/prompt examples you want to include for the CI section?]
- [VERIFY: Tone check — is this the right amount of heat or too much/too little?]

## Links
- Kwik-E-Mart: https://sr.ht/~ninjapanzer/Kwik-E-Mart/
- Gas Town: https://github.com/steveyegge/gastown
- Beads: https://github.com/steveyegge/beads
- Catalyst article: @/i-made-a-thing/catalyst-orchestrator.md
- Recreating Kafka article: @/i-made-a-thing/recreating-kafka-blind.md
-->
