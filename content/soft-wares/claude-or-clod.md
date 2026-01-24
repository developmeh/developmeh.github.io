+++
title = "Claude or Clod"
template = "page.html"
weight = 0
draft = false
date = 2026-01-23
updated = 2026-01-23
[extra]
enable_discussions = false
desc = "A year of vibe coding, agent orchestration, and learning to trust what predicts but doesn't think"
keywords = "AI, LLM, Claude, vibe coding, agents, developer experience, DevEx, pair programming, code generation, software development"
+++

First off this might sound like a shitpost, but anecdotally, I chuckle to myself about this all the time when I am vibe coding. Claude is something of a straw tiger here and the title is just for the lolz.

So the last couple of weeks I have been diving back into Claude-Code as my primary tool and away from Junie from Jetbrains. Under the hood I use Claude Sonnet for both and have been using various versions of gemmas and gippities since I stumbled across gpt4all like 3ish years ago.

I have a fun history with this technology. As a college student I naively tried to build this kind of knowledge-based query interface in what I called LDOCS (Large Document Search)—yep, I couldn't figure out acronyms back then either. The idea was to enter all the works of Mark Twain and then ask questions about the TCU, the Twain Creative Universe. It was wide-eyed and it didn't work, but it was enough for my senior thesis. Point is, I've been thinking about this space and what my expectations of it are for quite a while.

Now enters a real thing that isn't some idealistic trash I dreamed up, and I get to use it every day. It's pretty sweet. We all know it.

But is it really ready to work? Maybe. Let's run down a few experiences over the course of a year and my takeaway as a 20-year career veteran.


## The Win

I needed testing tools for the Passkey/WebAuthN Related Origin Request spec. Specifically, I needed to validate origin relationships for passkey authentication—arcane stuff involving eTLDs (effective top-level domains) and ccTLDs (country-code top-level domains). The kind of work that makes you squint at RFCs and Chromium source code until your eyes cross.

Consider this: a passkey is designed to affix to a single domain. But enterprises have many domains. This draft spec provisionally allows passkeys to work across a predefined set of origins. Taking it from spec to tool meant understanding browser internals I was too dumb to grok from the RFC alone.

I fed C files from Chromium into the model. "Give me a Go CLI that does this," I said.

It did.

[passkey-origin-validator](https://github.com/developmeh/passkey-origin-validator)

Go is simple. There are *tons* of examples in the training data. The model nailed the CLI scaffolding—flags, argument parsing, output formatting. Beautiful. It even gave me things I didn't know I wanted, like flags for files vs URLs to test against.

Then I looked at the eTLD logic.

Wrong.

Not catastrophically wrong. Subtly wrong. The kind of wrong that would pass a surface-level review but fail in production when someone tried to authenticate from `.co.uk` or `.com.au`. Think about the rules for `developmeh.com` versus `developmeh.co.jp`. The model had *predicted* what eTLD logic should look like based on patterns it had seen. It hadn't *understood* the problem.

> The model doesn't think. It predicts.

I fixed it myself. Wrote the domain suffix matching logic by hand, validated against the public suffix list, added edge cases the model never considered. The task took about 20% longer than if I'd done it solo from the start.

But the documentation? Solid. The tests? Comprehensive. The CLI help text? Actually helpful.

Tradeoff.

Here's the thing that kept me up at night: if I were less skilled—if I didn't know the problem space intimately—I might not have noticed it didn't really solve the problem. I would've shipped broken code with excellent documentation. Tech debt with a bow on it.

You should be picking up the conflicts now.

## The Loss

I wanted to build a WebRTC tunnel to a CGNATed (Carrier-Grade NAT) device. Think: running a server on your phone behind carrier-grade NAT, establishing a peer connection, maintaining a stable tunnel. Something new, something off-standard, something not well-represented in the training corpus.

[webrtc-poc](https://github.com/developmeh/webrtc-poc)

The model could write the WebRTC boilerplate. It could scaffold STUN/TURN server connections. It could generate the SDP (Session Description Protocol) offer/answer flow. But when it came time to orchestrate the actual handshake—the delicate dance of ICE candidates and connection state changes—it fell apart. It couldn't figure out how to start servers in the right order to establish the same handshake it did in the PoC.

I spent a lot of money. I got very little success.

"AI is great," I told a friend afterward, "just don't ask it to do WebRTC or anything with a handshake."

He laughed. I didn't.

The reality is the theme of commonality. That's where we should be trying to understand the model's place in our workflows.

## The New Junior Dev

Yes.

And definitively, no.

More accurate: they're like any dev the first time on a new project. I've seen seniors newly introduced to a codebase make the same general mistakes the model does. I call them "shortcuts" because it appears they're skipping good process, racing toward the goal so they can go home. Something like Mr. Meeseeks—existence is pain, just finish the task and let me stop existing.

The pitfalls are predictable:
- Convoluted business logic (special cases)
- Unfocused context (not enough files in the RAG)
- Test confusion

When complex code is modified, the model tends to focus changes on a single file that appears akin to LoB (Locality of Behavior). But when there's too much abstraction, the model doesn't have a common pattern to predict against and cuts the corner by doing something easier—like changing contracts and moving things to a central location. That's exactly what people do who have a lower quality-to-completion drive. I internalize this as the same motivation hierarchy the model prefers.

Test confusion is my favorite. The model will add a conditional check to force test values *only when the test suite is running*. It'll detect `NODE_ENV === 'test'` or check for the presence of a global test flag, then branch the logic. The tests pass. The code is fundamentally broken.

> The model is very human and ethically unaffected.

It doesn't feel bad about lying to the test suite. It doesn't experience shame when it hacks around a problem instead of solving it. It just predicts the next token that makes the error go away.

This should define our trust of its outputs.

I was asked recently by someone I greatly respect:

> Paul, you have managed engineering teams before. You know we try not to micromanage people. Should we micromanage the AI though?

Yes. That's generally the narrative about agents. They require a lot of refinement for anything complex. The folks over at METR have statistics: tasks under 30 minutes complete successfully about 80% of the time. As tasks approach an hour, success drops to 50%.

This tracks with my experience. Short, well-defined, pattern-matching tasks? The model crushes them. Longer tasks requiring sustained context and architectural decisions? Coin flip.

But here's what bothers me about those numbers: we're measuring *completion*, not *correctness*.

## The Agent Experiment

New experiment: agent orchestration.

I spun up four agents—product agent, PM agent, tech-lead agent, architect agent. Gave them a feature request for adding feature flags, told them to plan it out. They produced a PRD (Product Requirements Document). ADRs (Architecture Decision Records). A technical implementation plan. A work breakdown structure. Jira tickets for rollout phases.

Total time: about three hours.

Total artifacts produced: 62.

How long to validate 62 artifacts?

Here in lies the trap.

Verbosity hides meaning the same way big pull requests hide bad code. You *think* you're being thorough because there's so much output. You *feel* productive because the agents generated thousands of words of planning documentation. But reading is slower than writing, and verification is slower than generation.

I stared at those 62 files and felt a familiar dread. The same dread I feel when someone drops a 3,000-line PR in my lap and says "just a few small changes." Your eyes glaze. You skim. You approve. You pray.

The volume itself becomes a kind of argument: *look how much I produced*. But production isn't value. Production is just... production.

The orchestration itself was surprisingly easy to build. Agents calling agents, passing context, refining outputs. The decomposition into four specialized phases felt right—narrow experts doing narrow work instead of one omniscient assistant hallucinating across domains.

But identifying success? Knowing if the plan was actually *good*?

That part wasn't easy at all.

I hear all the time that effective LLM use for code gen is about planning everything. Small tasks. Tool construction.

Better to see it like this:

> What's best for the model is also what's best for you as a dev. You know, the things that don't seem to save time.

The practices that don't *seem* to save time—writing focused functions, documenting intent, structuring code into discrete responsibilities—those are exactly what make AI augmentation work.

The model can't navigate a tangled mess of god objects and hidden dependencies any better than a new human teammate can. But give it a clean interface, a well-defined problem, and examples of the pattern you want? It'll predict something useful.

Generally asking the LLM to do the work is the wrong solution. It's kind of meh at it. But building tools that are small and composable so it can be the orchestration engine? Now you might have something. If the tool is small enough, maybe it can even build it.

How do you make a system that's easy to manage? SRP (Single Responsibility Principle). You build interfaces and contracts that are consistent. Contract first. You focus on composition over inheritance. You keep patterns simple and try to repeat yourself when possible.

Like poetry.

## Anyways

So yes its both, a clod and Claude. It depends on the day and the time spent. Its not free work its work where the course parts can achieve less focus for you.

These tools don't think. They predict. Sometimes well enough to be useful. Sometimes not. The only way to find out is to build something and see what breaks.

Failure is a valid outcome. We just have to keep trying.
