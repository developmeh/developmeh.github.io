+++
title = "Judgement-Capable Circuits: What You Write Loops Out Of"
template = "page.html"
weight = 0
draft = false
date = 2026-08-08
updated = 2026-08-08
slug = "judgement-capable-circuits"

[taxonomies]
topics = ["Agentic AI", "Agent Orchestration", "Context Engineering", "Developer Experience"]

[extra]
schema_type = "TechArticle"
desc = "If the job is writing loops rather than prompts, the question is what you write them out of. A circuit is deterministic wiring with judgement localised to specific nodes, assembled from tmux, an event transport, and generated prompts, running on credentials you already hold."
keywords = "loop engineering, agent loops, agentic workflow, judgement capable circuits, multi-agent orchestration, tmux agent fleet, generated prompts, prompt writer pattern, kwike, wavelet, Bedrock, Claude Code, deterministic routing, agent supervision, dynamic workflow"
sitemap_priority = "0.9"

[[extra.faq]]
q = "What does it mean that the job is writing loops rather than prompts?"
a = "A prompt is a single instruction to a model. A loop is the surrounding program that decides what to send, when to send it, what to do with the result, and when to stop. Boris Cherny, who leads Claude Code at Anthropic, has described no longer prompting the model directly and instead writing the loops that prompt it."

[[extra.faq]]
q = "What is a judgement-capable circuit?"
a = "A fixed, inspectable wiring of agents where the topology is deterministic and judgement is localised to the specific nodes that need it. You can look at the circuit and know exactly where the non-determinism lives, which is not true of a long agent conversation."

[[extra.faq]]
q = "Why use tmux to run an agent fleet?"
a = "Spawning is a terminal concern rather than a daemon concern. tmux already owns process lifetime, environment, teardown, and the operator's ability to see and intervene. Panes are visible, attachable, killable, and die with the session, so the orchestrator never needs to gain process management."

[[extra.faq]]
q = "Do you need an agent platform to run multi-agent workflows?"
a = "No. The models you already have access to arrive as chat, whether over an OAuth subscription, an API key, or Bedrock in your own account. Orchestration can be built on top of those credentials rather than bought as a second product with its own billing relationship."

[[extra.faq]]
q = "Where should judgement live in an agent circuit?"
a = "Only where a decision genuinely varies. If an outcome is predictable from a status field, a rule should make it. Model calls cost tokens, latency, a subprocess, and a failure-fallback path, and they buy nothing when the answer never changes."
+++

# Judgement-Capable Circuits

Boris Cherny, who leads Claude Code at Anthropic, has said he does not prompt Claude anymore, and that his job is to write loops ([The New Stack](https://thenewstack.io/loop-engineering/)). That reframing is correct and it is the right thing for people to be repeating.

It also leaves the more interesting question open. If the job is writing loops, **what are you writing them out of?**

A loop needs parts. Something has to hold state between iterations, something has to decide what each participant sees, something has to own process lifetime, and something has to author the instructions each node receives. Answer those four badly and you get a long conversation that degrades. Answer them well and you get a circuit.

## What is a circuit?

A circuit is a fixed, inspectable wiring of agents where the topology is deterministic and judgement is localised to the nodes that need it.

The word is doing real work. In an electronic circuit you can point at the components, trace the paths between them, and reason about the whole from the parts. That is the property worth importing. **You should be able to look at an agent workflow and know exactly where the non-determinism lives.** A long agent conversation does not have that property, because the non-determinism is smeared across every turn and each response steers the next.

This is [rules before models](/tech-dives/agentic-ai-engineering/) expressed as a shape rather than an argument. The wiring is rules. The nodes are judgement. A component that needs to read prose and decide something gets a model call. Everything else is a status field and an `if`.

## What do you build it out of?

The constraint I care about is that the only primitives a person should have to learn are **launching shells in tmux**. Everything above that gets generated: the launcher, the per-node prompts, the wiring. If understanding the system requires learning a framework, the framework has become the product and the work has moved from your problem to someone else's abstraction.

Three parts, each of which already exists.

**tmux is the supervisor.** Spawning participants is a terminal concern, not a daemon concern. tmux already owns process lifetime, environment, teardown, and the operator's ability to watch and intervene. Panes are visible, attachable, killable, and die with the session. In [wavelet](/projects/wavelet/) this is `spawn`: one window per conversation, one pane per participant, each pane running a wrapped session. The daemon never gains process management, which means there is no supervision code to write or trust.

**[kwike](/projects/kwike/) is the transport.** Three primitives, `dispatch`, `consume` and `daemon`, over an append-only JSON-lines event store, with a daemon that meshes across network boundaries. It is designed for remote systems from the start, and its durability guarantee is the useful one: the model may or may not perform correctly, but if the action should happen, the agent gets the message. There is always an execution, always a log, always feedback. The durability is in the pipeline rather than the output.

**[wavelet](/projects/wavelet/) is the cursor.** Its job is position and attention: which participant has seen what, and what should reach them next. It runs on the developer machine, wrapping sessions that use whatever credentials you already have. Delivery is a per-wavelet cursor and a set of rules, with full text for anything that mentions you, comes from the operator, or is a surfaced conclusion, and a one-line digest for everything else.

Those two are complementary rather than competing. **kwike moves work across machines. wavelet moves conversation.** One is an event transport with a durability guarantee, the other is an attention layer with a delivery policy. Combining them, kwike as the substrate that crosses network boundaries and wavelet as the cursor that decides what reaches whom, is the direction I think this goes. I want to be clear that this is a design direction rather than something I have shipped.

## Where does the judgement go?

The temptation with a circuit is to put a model at every junction, because a model at a junction feels like intelligence. It usually is not.

I have made this mistake and reversed it more than once. [Catalyst](/i-made-a-thing/catalyst-orchestrator/) v0.1 had a Haiku orchestrator clearing gates: the daemon would package a situation and ask what to do. "Review passed, merge or MR?" It worked, and it was spending tokens to reach conclusions that never varied, because review passed always meant merge. The next version parsed a status field instead. wavelet's attention layer went the same way, since a model-based attendant was designed first and rejected once the policy turned out to be expressible as rules.

So the placement heuristic is narrow. **A node gets a model when the decision genuinely varies with the content.** Reading a diff and deciding whether it satisfies a specification is judgement. Routing on the outcome of that decision is not.

The corollary matters too. When a junction does need judgement, spend the call without hesitating. The skill is telling the two apart, not minimising model usage.

## Who writes the prompts?

The circuit is authored per task rather than assembled from a library, which is the part that makes it dynamic and the part that makes it sustainable.

This is the [prompt writer pattern](/soft-wares/agentic-patterns-elements-of-reusable-context-oriented-determinism/) promoted from a technique to the interface. Rather than maintaining a growing collection of skills that drift and go stale, an orchestrator reads the workflow template and writes each node's prompt to a file immediately before spawning it. Two things fall out. The orchestrator re-reads the template every time, which keeps the workflow in its attention rather than letting it decay across spawns. And the prompt files are a crash-recovery log, so a collapsed context does not lose the shape of the work.

It also lines up with something I have found reliably true: agents are better at writing code than at using tools. A long sequence of tool calls is hidden context and procedural order the model must hold, and models struggle past three or four steps. Discovering a process by iterating, then having the model turn a successful run into a script it executes, is more semantically specific and much more repeatable.

A generated launcher is that idea applied to the circuit itself. The artifact is a shell script and a directory of prompts. Both are readable, diffable, and committable.

## What does it run on?

This is where the shape earns its keep, and it is the part I see discussed least.

The models you already have access to arrive as chat. An OAuth subscription, an API key, or Bedrock running in your own AWS account all give you single-turn conversation and nothing above it. Orchestration is left as an exercise, and the usual answer is to buy a second thing: an agent platform, a separate API budget, a hosted harness with its own billing relationship and its own opinions.

A circuit built from tmux, an event log, and generated prompts does not need any of that. **It adds orchestration on top of credentials you already hold.**

Supporting the credentials people actually have means supporting the awkward ones. Sessions on third-party providers, Bedrock among them, have no development-channel surface and cannot be woken by notifications at all. They are, however, re-invoked when a background process completes. So the wake becomes a blocking process that exits: it waits until new work lands, prints everything since the cursor, emits a re-arm line, and terminates. **Completion is the wake.**

That is not a workaround bolted on for one provider. It is what happens when portability across credential sources is a design requirement rather than an afterthought. A Bedrock participant is a node in the circuit like any other, wired slightly differently because its transport differs.

## What this is not

It is not a platform, and the moment it becomes one it has failed its own premise. The value is that the parts are individually comprehensible: a shell, a log file, a pane, a prompt on disk.

It is also not a claim that the composition is finished. tmux supervision is real and running. kwike is real and designed for remote systems. wavelet is real and running locally against whatever credentials are to hand. Wiring them into a single circuit is a direction, and the honest state is that the parts exist and the composition is in progress.

The question the loops framing opens is a good one, and worth answering with components rather than a product. **Loops need parts. The parts should be ones you can already reason about.**

## Related reading

- [Agentic AI Engineering](/tech-dives/agentic-ai-engineering/), the broader position on putting determinism in the pipeline
- [Agentic Patterns](/soft-wares/agentic-patterns-elements-of-reusable-context-oriented-determinism/), including the prompt writer pattern
- [Catalyst](/i-made-a-thing/catalyst-orchestrator/), the model-to-rules migration as it happened
- [kwike](/projects/kwike/), Unix primitives for agent workflows
- [wavelet](/projects/wavelet/), forked conversations and targeted delivery
- [Five Techniques That Keep Paying Off](/tech-dives/techniques-that-keep-paying-off/), on harnesses, hooks, and why agents write code better than they use tools

Everything on this subject: [Agentic AI](/topics/agentic-ai/), [Agent Orchestration](/topics/agent-orchestration/)
