+++
title = "A Deterministic Box for Non-Deterministic Engines"
template = "page.html"
weight = 0
draft = false
date = 2026-01-27
updated = 2026-01-27
[extra]
desc = "Taming LLM non-determinism through structured workflows, task tracking with Beads, and memory management strategies for more predictable AI-assisted development"
keywords = "LLM, AI agents, non-determinism, task tracking, Beads, Claude Code, agentic workflows, context management, Kubernetes, DevOps, AI orchestration"
+++

## The Nature of Non-Determinism with LLMs

So you may have heard of weights, biases, and temperature when LLMs are described. For the uninitiated: weights and biases are the core parameters learned during training that encode the model's knowledge, while temperature is an inference-time parameter that controls how much variance appears in the model's outputs. Higher temperature means more randomness in token selection; lower temperature means more deterministic responses. It's exactly this temperature parameter that ensures the model will respond with some variance for the same input. So that's clearly this non-determinism which flies in the face of the normal expectation of computers, but it's this that also provides some of the nuance in token prediction that makes the LLM work so it's easy to identify this as an __Architectural Trade-Off__ and not necessarily a __Detractor__. So hoping that provides some grounding let's talk about how to make good use of this engine of... making shit up.

### Making Shit Up

Yep, so that's not a tradeoff, it's a flaw, one we haven't solved yet. When the context is ambiguous the model chooses to do one of two things:

1. Just pretend it didn't hear what it was asked to do
2. Make shit up, hallucinations

Of course I think the former is not talked about as much as the hallucinations. Not to mention that the hallucinations are harder to detect and protect. Note that hallucinations are actually a separate problem from non-determinism - they're about confidence miscalibration and training data limitations, not temperature variance. Hallucinations can occur even with low temperature settings. But we can take a stab at it with some extra prompting and extra runtimes at the cost of tokens. Don't get too upset this is just the normals of computers, we make a simple thing and it has sharp edges, so we make more things that consume some extra energy to constrain the first.

Usually, these are to solve for the inefficiency of the human communication, but sometimes it's just cause people wanna abuse it. I like to think of Auth as a regular pain point we don't really need but have to have because trust is a hard problem. Most of whats on the web doesn't need centralized authentication but GPG has always been too hard so we made something easier to understand.

## What to do?

Ok, back to the question, well I call it micromanagement but that kind of implies that the model and its agents have some kind of human agency, which they don't. Although some of their processes are directly modeled after humans so we can loosely apply some techniques to rein them in.

First, let's talk about context and ambiguity. If you haven't figured this out yet the longer the context the more the model's attention distributes across tokens, reducing precision on individual details - a "lost-in-the-middle" effect where information gets deprioritized. Most of this is your fault because even with your best effort you introduce inconsistencies and other inaccuracies into the conversation. The lesson, clear your context often and especially between phases of your work, aka, planning, building, and verifying. I like to consider this an analogy to writing and editing. Have someone else edit your work or write it and review it a week later to improve objectivity. Thankfully with LLMs their memory is as ephemeral as you like.

So we need a way to turn a goal into a workstream that allows us to actually look away from the model's stream. Some might call this an agentic orchestration but I feel these often sprint from meaningful to overly complicated in a matter of weeks. Especially if you use something like Claude-Code, Codex, or OpenCode all the building tools are there already. So starting from something like Claude-Code we need to teach our main agent interface to better follow some process when working.

Here is an example:

__CLAUDE.MD__
```markdown

## Working Style

When collaborating on this project:
- Check existing files first before suggesting changes
- Ask questions one at a time to refine ideas
- Prefer multiple choice questions when possible
- Focus on understanding: purpose, constraints, success criteria
- Apply YAGNI ruthlessly - remove unnecessary features from all designs
- Present designs in sections and validate each incrementally
- Go back and clarify when something doesn't make sense

## Deliverables

- Break down the decisions from collaboration into tasks
- You must use any defined task tracking tools outlined in the Task Tracking section to create tasks falling back to markdown files if nothing is defined
- Create a report for the executiong plan with dependencies mapped

## Workflow Guidelines

- Create an epic for each high-level objective
- Create subtasks as a todo chain under the epic
- Write titles as the task to be performed (imperative form)
- Add detailed descriptions with examples of work to be done
- Verify each task before closing
- Log details about failures and retries in ticket descriptions for historical tracking
- When an epic is completed, write a report of the task graph and verify all items were performed
```

### Controlling Core Memories

As I included above _Deliverables_ and _Workflow Guidelines_ we initially want our first pass to be on work breakdown and dependency. This will provide some added benefits the way we will track that work progress though. Often the agent writing code falls victim to the two points above with a couple of variations. Hallucinations in this case are items that just don't work and the remainder is missed features. That's good though because we can track and essentially later interrogate success and failure of the model's execution. Better yet we can finally realize the age old dream that we can repeat a variation of a task in the future more accurately because each replanning is less ambiguous. Good luck doing this with people but with LLMs it's all data.

So memory management moves into tasks, which can be in markdown, Jira via MCP (Model Context Protocol - a standard for connecting AI agents to external tools), or my preference, [Beads](https://github.com/steveyegge/beads) I don't think there is a lot of big effective differences for me except when we come back to the nature of context size complication introducing confusion.

So beads does for AI what Jira does for humans and yet even as a human I would rather use Beads than Jira. Arguably, the difference is that tools like Beads focus on de-complicating the organization of work, its there for the worker's benefit. Jira on the other hand only benefits the bean counters and the workers just have to suffer so that a very few can complain that the reports it produces are useless.

Sorry, my Jira PTSD is showing... Beads, right Beads lets the coding agent take its task breakdown and put it into a graph with dependencies and epics, these feel meaningless to the agent but it's more about what we get to do with it later. It's easier for me to say to a fresh context, review the epic X and verify its functionality. You'll notice that when it finds something is a failure it usually just tries to fix it but it's also going to record a stream of attempts and what was the final resolution. Resulting in a history of the model's confusion introduced from me or the plan, but when I wanna do something similar I can use the JSONL (JSON Lines format - one JSON object per line) from the beads sync operation to prompt a variation of the task and create a new task breakdown.

Here is a claude partial to explain beads
````markdown
### Task tracking

Use 'bd' (beads) for task tracking. Run `bd onboard` to get started.

#### bd Quick Reference

```bash
# Discovery & Navigation
bd ready              # Find available work
bd show <id>          # View issue details
bd show <id> --children  # Show issue with subtasks

# Task Management
bd create "<title>" --type epic    # Create an epic
bd create "<title>" --parent <id>  # Create subtask under parent
bd update <id> --description "..."  # Update description
bd update <id> --status in_progress # Claim work
bd close <id>         # Complete work

# Sync & Persistence
bd sync               # Sync with git (exports to JSONL)
```

#### Workflow Guidelines

- Create an epic for each high-level objective
- Create subtasks as a todo chain under the epic
- Write titles as the task to be performed (imperative form)
- Add detailed descriptions with examples of work to be done
- Verify each task before closing
- Log details about failures and retries in ticket descriptions for historical tracking
- When an epic is completed, write a report of the task graph and verify all items were performed

#### Displaying Task Graphs

Use `bd show <epic-id> --children` to display the task hierarchy. For visual reports, create ASCII diagrams showing task dependencies and completion status.
````

## Uniqueness vs Repeatability

This is kind of the funny part of this whole process, the LLM can help with a bespoke task but it doesn't generally improve performance because the context size tends to bias towards failures and you end up having to check its outputs and re-validate anything ambiguous. You may say that you don't need to, but just look at the news, it's the failure mode the AI tools get lambasted on. Of course being an engineer we know that everything is essentially wrong and we are balancing the acceptable amount of wrong we can accept at any given moment.

This of course means that when we can find a process that is refinable to a predictable set of tasks we will end up trying to build some complicated brittle script that can automate the process and here is why building things with computers can be kinda dry. We should let the models handle the fixed set of tasks that need a little flexibility but doesn't offer too much range of opportunities for errors.

Refinement of process from memory is just a strategy but it's one that works quite well since the next agent can read the actions of its predecessor, you can bias it to take the success path and start ignoring it, which is the dream. For years I have been using LLMs and finding myself trapped staring at the console because it prompts me for feedback every couple of seconds building context or I have to endlessly remind it to complete the tasks. Both of these conditions are mostly eliminated.

### Deploying to K8s

A concrete example of this is to deploy an application to kubernetes. This is super well documented and there is a ton of good tooling but it's also a highly configurable system. Each enterprise makes its own rules and policies around how containers are described. It can be very tiresome reading charts and chart documentation while bashing your head against a statement like _CrashBackoffLoop_. It's not like there isn't a way to learn about what's happening but it's a lot of command orchestration, the LLM can collect context of the failure much better since it can read multiple streams concurrently. So I recently deployed [flagd](https://flagd.dev) (an open-source feature flag daemon) to our cluster and let Claude handle the building of charts. To keep things simple I let it just write the deployment and service artifacts itself. It failed a bunch but eventually determined from feedback of the k8s MCP that we were missing a config file. It mounted a volume and created a config map, 20 minutes later I have a stable instance running. I will need to do some work to make this fit into organizational norms but I am also ready for my PoC at the same time. I also have a log of what worked and what didn't work so when it comes time to deploy something else to our cluster I have a baseline of tasks which will reduce the planning time and token volume.

An example of this completed task output:

```bash
Beads Task Report - January 26, 2026

Epic: Deploy flagd to namespace

ID: feature-flag-prd-q6w | Status: ✅ CLOSED

---
Task Graph

feature-flag-prd-q6w (EPIC) ✅ CLOSED
├── .1 Create flagd Deployment manifest     ✅ CLOSED
├── .2 Create flagd Service manifest        ✅ CLOSED
├── .3 Deploy flagd Deployment to cluster   ✅ CLOSED
├── .4 Deploy flagd Service to cluster      ✅ CLOSED
└── .5 Verify flagd deployment is running   ✅ CLOSED

---
Task Completion Summary
┌─────┬────────────────────────────┬────────┬───────────────────────────────────────────┐
│ ID  │            Task            │ Status │               Close Reason                │
├─────┼────────────────────────────┼────────┼───────────────────────────────────────────┤
│ .1  │ Create Deployment manifest │ ✅     │ flagd_deploy/k8s/deployment.yaml created  │
├─────┼────────────────────────────┼────────┼───────────────────────────────────────────┤
│ .2  │ Create Service manifest    │ ✅     │ flagd_deploy/k8s/service.yaml created     │
├─────┼────────────────────────────┼────────┼───────────────────────────────────────────┤
│ .3  │ Deploy Deployment          │ ✅     │ Deployment exists in cluster              │
├─────┼────────────────────────────┼────────┼───────────────────────────────────────────┤
│ .4  │ Deploy Service             │ ✅     │ Service exists with ClusterIP 127.0.0.1   │
├─────┼────────────────────────────┼────────┼───────────────────────────────────────────┤
│ .5  │ Verify deployment running  │ ✅     │ Pod running after troubleshooting         │
└─────┴────────────────────────────┴────────┴───────────────────────────────────────────┘
---
Troubleshooting Log (Task .5)

The verification task encountered issues that required 3 attempts:

Attempt 1 - CrashLoopBackOff
- Container exited with code 0 after printing help
- Fix: Added args: [start] to deployment

Attempt 2 - Fatal: no sync implementation set
- flagd requires a flag source (file, gRPC, or HTTP sync)
- Fix: Created ConfigMap with example flags, added volume mount

Attempt 3 - SUCCESS ✓
- Created ConfigMap: accounts-utility-flagd-service-dev-flags
- Updated deployment with volume mount to /etc/flagd
- Added args: --uri file:/etc/flagd/flags.json

---
Final Deployment State
┌───────────────┬────────────────────────────────────────────────────┐
│   Component   │                       Value                        │
├───────────────┼────────────────────────────────────────────────────┤
│ Pod           │ flagd-service-dev-87fb5946f-vft4t                  │
├───────────────┼────────────────────────────────────────────────────┤
│ Status        │ 1/1 Running                                        │
├───────────────┼────────────────────────────────────────────────────┤
│ Pod IP        │ 10.10.10.102                                       │
├───────────────┼────────────────────────────────────────────────────┤
│ Service IP    │ 172.10.10.10                                       │
├───────────────┼────────────────────────────────────────────────────┤
│ Node          │ node                                               │
├───────────────┼────────────────────────────────────────────────────┤
│ flagd Version │ v0.13.2                                            │
└───────────────┴────────────────────────────────────────────────────┘
Exposed Ports:
- gRPC: 8013
- HTTP: 8014
- FlagSync: 8015
- OFREP: 8016

---
Result: 5/5 tasks completed - Epic successfully closed
```

## What next

Here is how I would go about things, start by recording your plans. Maybe take one of my examples and refine it for you and check your experiences. Then install Beads and just manually create tasks and see how the agent interacts. Then go ahead an automate the whole thing but maybe this time we can avoid [xkcd:1319](https://xkcd.com/1319/) but probably not :)
