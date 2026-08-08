+++
title = "Orchestrating Agents on Amazon Bedrock: When You Cannot Push, Make Completion the Wake"
template = "page.html"
weight = 0
draft = false
date = 2026-08-08
updated = 2026-08-08
slug = "orchestrating-agents-on-bedrock"

[taxonomies]
topics = ["AWS", "Agentic AI", "Agent Orchestration", "Capability Security"]

[extra]
schema_type = "TechArticle"
desc = "Multi-agent designs assume push delivery, which Bedrock-hosted sessions cannot receive. Making process completion the wake signal, and scoping agent credentials with session policies so blast radius is a property of the credential rather than the prompt."
keywords = "Amazon Bedrock, Bedrock agents, multi-agent orchestration AWS, agent fleet AWS, MCP notifications, asyncRewake hook, cursor delivery, sts AssumeRole, session policy, scoped credentials for AI agents, IAM least privilege agents, Claude on Bedrock, agent supervision, wavelet"
sitemap_priority = "0.9"

[[extra.faq]]
q = "Can Bedrock-hosted model sessions receive push notifications from an MCP server?"
a = "No. Sessions on third-party providers including Bedrock have no development-channel surface, so they cannot be woken by MCP notifications. Any orchestration design that assumes push delivery is implicitly assuming you are not on Bedrock."

[[extra.faq]]
q = "How do you wake an agent session that cannot be pushed to?"
a = "Make process completion the wake signal. A blocking process waits until new work lands, prints everything since the cursor, emits a re-arm line, and exits. The session is re-invoked when the background process completes, reads the funnelled output, and immediately re-arms with the new cursor."

[[extra.faq]]
q = "Why track a delivery cursor per conversation rather than globally?"
a = "With time-sortable ids such as UUIDv7, a single global cursor advanced by a busy conversation steps straight over older but unseen messages in a quiet one. Tracking position per conversation is the only way to avoid dropping them with no error reported."

[[extra.faq]]
q = "How should you scope AWS credentials for an autonomous agent?"
a = "Short-lived credentials from sts:AssumeRole with a session policy, which intersects with the role policy so effective permissions are the smaller of the two, and a separate role per agent task rather than one shared credential. The blast radius of a confused agent then becomes a property of the credential rather than the prompt."

[[extra.faq]]
q = "What happens if the wake process dies quietly?"
a = "Delivery stops until the next turn, which on an idle session is never. The exit code has to carry the signal, and an unreachable daemon must be treated as a reason to wake rather than as silence."
discussion_number = 59
discussion_url = "https://github.com/orgs/developmeh/discussions/59"
+++

# Orchestrating Agents on Amazon Bedrock

Most multi-agent designs assume they can push. A coordinator has something for a participant, so it sends a notification and the session wakes up. That assumption is invisible until it breaks, and on Bedrock it breaks immediately.

**Sessions on third-party providers, Bedrock among them, have no development-channel surface and cannot be woken by MCP notifications at all.** Every orchestration design built on push delivery is assuming you are not running on Bedrock, which is an odd assumption given how many organisations reach Claude through their own AWS account rather than through a subscription.

This is what that constraint forces, and why the result is better than the thing it replaced.

To be clear about the status: the transport described here is built and running. The AWS-specific credential scoping in the second half is an argument about how it should be deployed rather than a report of a fleet I have running in production.

## Why does Bedrock get orchestrated differently?

Because access and capability are not the same thing.

The models you already have arrive as chat. An OAuth subscription, an API key, or Bedrock in your own account all give you single-turn conversation and nothing above it. Orchestration is left as an exercise, and the usual answer is to buy a second product with its own billing relationship and its own opinions about how your agents should be arranged.

Bedrock is the credential source with the strongest organisational reasons to exist. It keeps model traffic inside an account you control, under an IAM boundary your security team already understands, on a bill you already reconcile. Those are the reasons regulated teams end up there, and they are good ones.

They also mean the surface is narrower. You get inference. You do not get the developer-channel affordances that the first-party CLI relies on, and a design that treats those affordances as a floor rather than a bonus simply does not run.

## What do you do when you cannot push?

Invert it. **Completion is the wake.**

A session that cannot receive a notification is nonetheless re-invoked when a background process it launched finishes. So the delivery mechanism becomes a process whose exit is the signal:

1. It blocks until at least one new message lands across the conversations the participant belongs to.
2. It prints everything since the cursor, each message with its id.
3. It emits a re-arm line carrying the new cursor position.
4. It exits.

The session reads the funnelled output as ordinary task output, then immediately re-arms. No push, no polling loop inside the model's context, no notification surface required. The runtime you already have, a process that starts and stops, becomes the transport.

There is a variant for hosts that re-arm automatically on every turn end and have no caller to thread a cursor through. That version reads and commits position to the channel's own state file, so a session that falls back from push delivery to completion delivery resumes exactly where it left off, with no replay and no gap. A lock beside the file stops concurrent arms racing it, and a second arm exits immediately without writing.

## Where do the sharp edges turn out to be?

Three, and they are all about position rather than transport.

**Track the cursor per conversation, not globally.** Ids that sort by time, UUIDv7 for instance, make a single global cursor actively harmful: a busy conversation advances it past older but unseen messages sitting in a quiet one. Those messages are then never delivered, and nothing reports an error. The re-arm line necessarily flattens the map to its highest value, which is why the auto-re-arming variant persists the whole map instead.

**The exit code has to carry the signal.** Two means wake the session, zero means nothing happened. Critically, a daemon that has been unreachable for thirty seconds must also exit with the wake code. A delivery process that dies quietly stops delivery until the next turn, and on an idle session the next turn is never. Failing loudly is the only safe default when the alternative is silence that looks identical to "no news."

**Position advances on emission, not on acknowledgement.** Nothing acks a completion-based wake. A session killed between the wake firing and the output being read loses that batch. That is a real cost, accepted deliberately, because holding the cursor back until proof of receipt means re-delivering the same messages on every arm forever. Choosing which failure you want is the whole job.

None of these are Bedrock problems. They are what you find once you stop assuming a reliable push channel, and the design is more honest for having been forced through it.

## How should an agent's AWS credentials be scoped?

The transport gets messages to an agent. The next question is what that agent can reach when it acts, and this is where running on AWS is an advantage rather than a constraint.

The prevailing instinct is defence-in-depth around an agent that can already do anything: guardrails, prompt-level restrictions, a reviewer watching output. That is trying to secure something un-securable. The alternative is to build so the agent can only do what it was granted, which turns "how do I stop it doing the wrong thing" into "what can it do at all."

AWS has had the primitives for this since long before any of it was about models:

- **Short-lived credentials** from `sts:AssumeRole` rather than long-lived keys in an environment variable.
- **A session policy**, which intersects with the role policy so effective permissions are the smaller of the two. The role describes the ceiling; the session describes this task.
- **A role per agent task**, not one credential shared across a fleet. An agent spawned to work on one bucket, one table, one queue gets exactly that.

The result is that **the blast radius of a confused agent becomes a property of the credential rather than a property of the prompt.** No amount of clever instruction is required, and none is trusted.

This is the same shape as capability sandboxing in [beamlet](/projects/beamlet/), where the zero-value grant is pure compute and every ability is an explicit host-side grant enforced by runtime linkage. IAM is a coarser instrument than a WASM host function, but the principle carries: the grant is made outside the thing being granted, and the thing being granted cannot widen it.

There is a pleasing consequence if you also believe [agents are better at writing code than using tools](/tech-dives/techniques-that-keep-paying-off/). When the durable artifact is a script the agent produced and you reviewed, that script is the unit you scope credentials against. You stop granting permissions to a conversation.

## What does this add up to?

A participant on Bedrock is a node in a [circuit](/tech-dives/judgement-capable-circuits/) like any other, wired slightly differently because its transport differs. It receives work through completion rather than notification, it holds position in a cursor file, and it acts through credentials scoped to the task it was spawned for.

The broader point is that portability across credential sources is worth treating as a design requirement rather than an afterthought. Supporting the credentials people actually have means supporting the awkward ones, and the awkward ones are where the interesting constraints live. The completion-as-wake transport exists because Bedrock could not be pushed to, and it turned out to be the more robust mechanism: no notification surface to depend on, no delivery channel to keep alive, just a process that exits.

## Related reading

- [Judgement-Capable Circuits](/tech-dives/judgement-capable-circuits/), on what you assemble loops out of
- [wavelet](/projects/wavelet/), the coordination substrate this transport belongs to
- [beamlet](/projects/beamlet/), capability sandboxing for model-authored code
- [Agentic AI Engineering](/tech-dives/agentic-ai-engineering/), the wider position on determinism in the pipeline
- [Five Techniques That Keep Paying Off](/tech-dives/techniques-that-keep-paying-off/), on harnesses and why agents write code better than they use tools

Everything on this subject: [AWS](/topics/aws/), [Agentic AI](/topics/agentic-ai/), [Agent Orchestration](/topics/agent-orchestration/)
