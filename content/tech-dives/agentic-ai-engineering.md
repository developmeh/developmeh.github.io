+++
title = "Agentic AI Engineering: Building Determinism Around a Non-Deterministic Engine"
template = "page.html"
weight = 0
draft = false
date = 2026-08-08
updated = 2026-08-08
slug = "agentic-ai-engineering"

[taxonomies]
topics = ["Agentic AI", "Agent Orchestration", "Context Engineering", "Capability Security"]

[extra]
schema_type = "TechArticle"
desc = "A working definition of agentic AI engineering: treating tool-using language models as processes rather than conversations, and putting the determinism in the pipeline instead of the prompt."
keywords = "agentic AI engineering, LLM orchestration, context engineering, agent orchestration, non-determinism, deterministic routing, multi-agent coordination, capability sandboxing, specification-driven development, coding agents, context graph, sub-agents, kwike, beamlet, wavelet"
sitemap_priority = "1.0"
sitemap_changefreq = "monthly"

[[extra.faq]]
q = "What is agentic AI engineering?"
a = "It is the practice of building systems where tool-using language models do work unattended, and where the reliability comes from the surrounding pipeline rather than from the model. The engineering problem is not prompting. It is deciding what the model is allowed to decide, what state survives it, and how its output is verified."

[[extra.faq]]
q = "Why do long agent sessions get worse over time?"
a = "Because each response is fed back into the next request. The model's own output steers its future decisions, so errors compound, in the same way that deriving new values from derived values compounds error. Attention also spreads thinner as context grows, which deprioritises details in the middle. Clearing context between phases is more effective than better prompting."

[[extra.faq]]
q = "Should an LLM make orchestration decisions?"
a = "Usually not. If a decision is predictable from a status field, a rule should make it. Model calls cost tokens, latency, a subprocess, and a failure-fallback path, and they buy nothing when the answer never varies. Reserve the model for judgement that genuinely requires reading prose."

[[extra.faq]]
q = "How do you keep an agent's context focused?"
a = "Plan and execute in separate contexts, checkpoint to durable artifacts between phases, and spawn a fresh short-lived sub-agent per task rather than running one long session. The orchestrator holds the plan, and the implementers hold one task each and are discarded."

[[extra.faq]]
q = "How do you verify work an agent did while you were not watching?"
a = "Give a separate agent a fresh context, the specification, and the diff, and nothing else. A reviewer that never saw the implementation reasoning is not biased by it. Rejection spawns a new implementer, and the loop repeats until it passes."

[[extra.faq]]
q = "How do you run code a model wrote without giving it access to your machine?"
a = "Run it in a sandbox where the zero-value grant is pure compute and every capability is an explicit, host-side grant enforced by runtime linkage rather than convention. A tool the model authored thirty seconds ago should be physically unable to open a socket or a file it was not given."
discussion_number = 57
discussion_url = "https://github.com/orgs/developmeh/discussions/57"
+++

# Agentic AI Engineering

The interesting engineering problem with language models is not what you say to them. It is everything you build around them so that the model's unreliability stops mattering.

This page is the through-line across the work on this site: [kwike](/projects/kwike/), [beamlet](/projects/beamlet/), [wavelet](/projects/wavelet/), [Catalyst](/i-made-a-thing/catalyst-orchestrator/), and the patterns writing behind them. The premise underneath all of it is that **a tool-using LLM is a process, not a conversation**. Once you accept that, most of the failures people complain about become ordinary systems problems with ordinary systems answers.

## What is agentic AI engineering?

It is the practice of building systems where tool-using language models do real work without a human watching each step, and where the reliability comes from the pipeline rather than the model.

That framing moves the question from "how do I make the model behave?", which you cannot fully answer, to three questions you can:

- **What is the model allowed to decide?**
- **What state survives the model?**
- **How is its output verified before anything depends on it?**

Every project linked from this page is an attempt at one of those three.

## Why do long agent sessions degrade?

Non-determinism in an LLM is an architectural trade-off, not a defect. Temperature is what produces the nuance in token prediction that makes the thing work at all. [Hallucination is a separate problem](/tech-dives/a-deterministic-box-for-non-deterministic-engines/), a confidence-calibration failure that occurs even at low temperature, and it is worth keeping the two distinct because they have different mitigations.

The compounding failure is structural. In an ordinary chat loop, every response is appended to the context and re-sent with the next request. The model reads the whole history each time, so its own earlier output steers its later decisions. It is the same shape as the rule about not deriving new values from derived values until the procedure is proven. Instability in accuracy grows.

Layered on top is attention dilution. As context lengthens, attention spreads across more tokens and precision on any individual detail drops, the "lost in the middle" effect. Details you established early get deprioritised exactly when they matter.

The practical consequence: **clearing context is a more powerful tool than better prompting.** Especially between phases, meaning planning, building, and verifying. The analogy is editing your own writing. You get objectivity by coming back cold, and a model's memory is as ephemeral as you want it to be.

## What does treating an LLM as a process mean?

The long conversation is an artifact of pre-tool-use models. When the model could not touch files or run commands, a human had to sit in the loop moving things between steps, and multi-turn chat was the only interface available. That constraint is gone, though the habit remains.

Treating the model as a process means each invocation gets exactly the context required for one task and nothing else, then dies. It means the interesting state lives outside the model, in artifacts you control. It means composition, the thing Unix has been good at for fifty years, rather than one long session that slowly poisons itself.

[kwike](/i-made-a-thing/kwike-llm-first-agentic-workflow-composition/) is that idea taken literally. Three primitives, `dispatch`, `consume` and `daemon`, over an append-only JSON-lines event store. Producers emit events, and consumers subscribe, render a template, and execute an LLM subprocess. The design followed Eric Raymond's Unix rules deliberately rather than by analogy: flat text files, one job per program, every program a filter.

The line that matters most:

> I can't guarantee the LLM performs correctly. But I can guarantee that if the action should happen, the agent gets the message. **The durability is in the pipeline, not the output.**

That is the discipline in one sentence. You are not making the model reliable. You are making the *system* reliable despite it.

## How do you keep context focused?

These are the working patterns, described at length in [Agentic Patterns](/soft-wares/agentic-patterns-elements-of-reusable-context-oriented-determinism/).

**Plan, then execute, in separate contexts.** Coding agents are biased towards producing code, which means they start building before understanding, and you end up refactoring the wrong idea with a context now polluted by examples of it. Rather than fight the bias, redirect it: make the agent produce a *research artifact* first. It will happily deep-dive a codebase and describe architecture and sequence.

**Kill, then break down.** Once the research is verified, throw the context away. A fresh agent reads the document and decomposes it into tasks with a planned implementation for each. Checkpointing into artifacts is what makes the flush safe.

**Spawn one agent per task and discard it.** The orchestrator holds the plan, and each implementer gets one task and a fresh context, then dies. Run sub-agents as separate processes rather than in-band, or verbose tool output will overflow the orchestrator's own context.

**Write the sub-agent's prompt to a file before spawning it.** Two benefits, both real. The orchestrator re-reads the workflow template each time, which keeps it in attention. And the prompt files are a crash-recovery log, so if a context collapses you can see exactly where the work was.

**Match model tier to job.** Reasoning model to orchestrate, competent model to implement, cheap model to review.

## Where should determinism live?

This is the question I keep arriving at from different directions, and the answer has been the same every time: **use the model only for what needs a model, and make everything else a rule.**

I did not start there. I built the model-driven version first, three separate times, and replaced it each time.

**[Catalyst](/i-made-a-thing/catalyst-orchestrator/)** v0.1 put a Haiku orchestrator in charge of routing. The daemon watched for ready work, ran agents, and whenever something interesting happened it packaged the situation as a gate and asked Haiku what to do. "Review passed, merge or MR?" It worked. It was also spending tokens to reach conclusions that were completely predictable, since review passed always meant merge and review failed always meant fix. The daemon could have parsed a status field. The next version stopped asking and started deciding.

**[wavelet](/projects/wavelet/)** hit the same thing in a different guise. Delivering every participant's full prose to every agent in a fleet dilutes each session's role and interrupts work mid-task. The obvious fix is an attention layer, and a model-based attendant was designed first, then rejected. Once the policy is "digest whatever isn't addressed, from the operator, or already distilled," no judgement remains that justifies a subprocess, API cost, latency, and a fallback path. The attention layer became deterministic rules. Rules before models.

**[kwike](/projects/kwike/)** encodes the same instinct structurally, through schema-validated output contracts, cursors, and retries. The model's job is bounded, and the pipeline's job is everything else.

Three projects, three independent arrivals at the same conclusion. It is the strongest position I hold about this work. **Adding a model call feels like adding intelligence, when usually it adds a dependency, a cost, and a new failure mode in exchange for a decision an `if` statement could have made.**

The corollary matters too. When you do need judgement over prose, spend the model call without guilt. The skill is telling the two apart.

## What state survives the model?

If context must be cleared constantly, memory has to live somewhere durable. Every project here answers this the same way:

- **kwike**, an append-only event log. Always an execution, always a log, always feedback.
- **[The context graph](/tech-dives/a-deterministic-box-for-non-deterministic-engines/)**, task tracking in a graph whose IDs become commit names, so the decision tree extends into git history. Code, spec, and reasoning as one artifact.
- **wavelet**, a wave of forked wavelets and blips. Provenance for free: which agent asserted what, when, and in response to what.
- **beamlet**, where the repository *is* the agent's body. `git log` is its changelog and `git revert` its undo.

The pattern is that **the artifact is the memory, and the conversation is disposable.** It also gives you something a chat transcript will not: the ability to re-run a variation of a task later with less ambiguity than the first time, because the plan and its failures were recorded as data rather than as a scroll of prose.

## How do you verify unattended work?

Give a separate agent a fresh context, the specification, and the diff. Nothing else. A reviewer that never saw the implementation's reasoning cannot be persuaded by it. If it rejects, spawn a new implementer and try again, then spawn a *new* reviewer. Loop until it passes.

This is cheap, since the reviewer is the smallest model in the stack, and it is the difference between an agent that produced output and an agent that produced work you can rely on.

## How do you run model-authored code safely?

Once agents write their own tools, "what is this allowed to touch?" stops being hypothetical.

[beamlet](/projects/beamlet/) is the answer I built, a minimum viable OTP over WebAssembly with supervised, capability-sandboxed processes on wazero. It reproduces Erlang's supervision semantics, including restart policies, intensity windows and hot code loading, and then goes one step further than BEAM. **OTP isolates faults, and capabilities also isolate authority.** The zero-value grant is pure compute. Every ability is an explicit grant enforced by per-process runtime linkage rather than by convention, so a tool the model wrote thirty seconds ago physically cannot open a socket or a file it was not given.

Two consents, both the operator's. Dependencies are a permission, since builds run with `GOPROXY=off` and each tool declares an import policy reconciled against its parsed source on every deploy. And host content enters only where you explicitly mount it.

The harness on top has exactly five meta-tools, create, read, delete, call and list, and no other abilities. Every real capability it wants, it must author, and the supervisor hot-deploys it under supervision with crashes feeding back into reflection.

## How do multiple agents coordinate?

A conversation between several agents is a forked graph, not a linear transcript. [wavelet](/projects/wavelet/) takes Google Wave's data model of wave, wavelet and blip, and keeps the structure while discarding operational transformation and the everything-is-live UX that made Wave unusable.

Two agents hashing out a subproblem fork a private wavelet, and only the conclusion surfaces to the parent. That is the "sub-agent scratch context" pattern expressed as a **data model** instead of an orchestration script, which makes it replayable and inspectable rather than encoded in whoever wrote the script.

There is no agent-to-agent RPC. Everything is mediated through the shared artifact, a structured, replayable blackboard.

## What is this actually for?

Drudgery. Version upgrades, dependency management, keeping documentation in sync, resolving test failures with context. Boring, repeatable, well-specified work that has to happen.

The point is not to replace engineers. It is to replace grunt work, and the pattern is implied by the use case. Something built for yourself needs only to meet the need, while something others rely on demands considerably more of your agency. I would hope my carpenter cuts fewer corners on my cabinets than on their own.

## Related reading

- [Agentic Patterns: Elements of Reusable Context-Oriented Determinism](/soft-wares/agentic-patterns-elements-of-reusable-context-oriented-determinism/), the patterns in full
- [A Deterministic Box for Non-Deterministic Engines](/tech-dives/a-deterministic-box-for-non-deterministic-engines/), on non-determinism, context management, and the context graph
- [kwike: LLM-First Agentic Workflow Composition](/i-made-a-thing/kwike-llm-first-agentic-workflow-composition/), Unix primitives for agent workflows
- [Catalyst: An Orchestrator That Stopped Asking and Started Deciding](/i-made-a-thing/catalyst-orchestrator/), the model-to-rules migration as it happened
- [beamlet](/projects/beamlet/), supervised, capability-sandboxed WASM for model-authored tools
- [wavelet](/projects/wavelet/), multi-agent coordination as a data model
- [Keep Your Eyes on the IDE, and Your Robots on the Tickets](/i-made-a-thing/keep-your-eyes-on-the-ide-and-your-robots-on-the-tickets/), feeding the graph from an IDE
- [The AI Diaries](/soft-wares/ai-diaries/), running notes on all of it

Everything on this subject: [Agentic AI](/topics/agentic-ai/), [Agent Orchestration](/topics/agent-orchestration/), [Context Engineering](/topics/context-engineering/)
