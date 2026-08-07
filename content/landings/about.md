+++
title = "About Paul Scarrone"

draft = false

path = "about"
template = "page.html"
date = 2026-08-07
updated = 2026-08-07
[extra]
schema_type = "ProfilePage"
desc = "Paul Scarrone is a software engineer working on agentic AI workflows, developer experience, shell and test tooling, and software architecture. DevelopMeh is his workshop."
keywords = "Paul Scarrone, agentic AI engineering, LLM orchestration, context engineering, developer experience, devex, Nix, BATS, bash testing, TAP, software architecture, engineering leadership, ninjapanzer, developmeh"
enable_discussions = false
sitemap_priority = "1.0"
sitemap_changefreq = "monthly"
+++

# About Paul Scarrone

I build things that aren't useful. Not useful to most people, anyway. I build to learn, to capture just enough knowledge to work out why I'm an utter imbecile, and then waste an ungodly amount of time getting genuinely good at something nobody asked for. This site is where that lands.

The joke in the name is real but the work isn't a joke. After enough seasons of this industry you either become a tradesman or you keep picking at the thing until it tells you how it works. I picked the second one. What follows is the boring version, for the people — and the machines — that need it spelled out.

## What I work on

**Agentic AI engineering.** Treating tool-using LLMs as processes rather than conversations. Context engineering, agent orchestration, specification-driven agent development, multi-agent coordination, and the patterns that make non-deterministic engines produce deterministic-enough output. This is where most of my recent work lives — [kwike](/i-made-a-thing/kwike-llm-first-agentic-workflow-composition/), [beamlet](/projects/beamlet/), [wavelet](/projects/wavelet/), [Catalyst](/i-made-a-thing/catalyst-orchestrator/), and [Agentic Patterns](/soft-wares/agentic-patterns-elements-of-reusable-context-oriented-determinism/).

**Sandboxing and capability security.** Running model-authored code without handing it the machine — WebAssembly substrates, supervision trees, and capability grants enforced by runtime linkage rather than convention. See [beamlet](/projects/beamlet/).

**Developer experience.** Reproducible, template-able development environments with Nix. Why CI matters more than CD. Automating the parts of the job that shouldn't need a human. See [The Perfect Development Environment](/devex/the-perfect-dev-env/) and [CI Over CD](/devex/ci-cd/).

**Shell and test tooling.** Bash scripts are software and deserve to be tested like software. I write about BATS, the Test Anything Protocol, and stubbing your way to a testable CLI — [BATS: Testing Bash Like You Mean It](/tech-dives/bats-testing-bash-like-you-mean-it/), [TAPS](/tech-dives/test-anything-means-testing-bash/), [The Magic of Stubbing sh](/i-made-a-thing/the-magic-of-stubbing-sh/).

**Software architecture.** Decoupling patterns, encapsulation over abstraction, and building systems by rebuilding them badly first — see [Decoupling Patterns in Ruby](/software-architecture/decoupling-patterns-in-ruby-overview/) and [Learn Event Streaming by Recreating Kafka](/i-made-a-thing/recreating-kafka-blind/).

**Engineering leadership.** Code ownership, whether your org actually does DevOps, and what technical leadership costs the person doing it — [Just Forget About Owning Code](/soft-wares/just-forget-about-owning-code/), [Do Devs Really Do DevOps in your Org?](/soft-wares/do-devs-really-do-devops/), [The Good Sergeant](/soft-wares/the-good-sergeant/).

## Selected work

- **[kwike](/i-made-a-thing/kwike-llm-first-agentic-workflow-composition/)** — LLM-first agentic workflow composition. Building workflows agents can compose rather than scripts they have to follow.
- **[beamlet](/projects/beamlet/)** — a minimum viable OTP over WebAssembly. Supervised, capability-sandboxed processes on wazero, used as the substrate for an LLM harness that authors and hot-deploys its own tools.
- **[wavelet](/projects/wavelet/)** — Google Wave's data model repurposed as a coordination substrate for multiple agents. Forked conversations as a data model rather than an orchestration script.
- **[Catalyst](/i-made-a-thing/catalyst-orchestrator/)** — an orchestrator that stopped asking and started deciding. A daemon that parses, routes, and creates steps at runtime.
- **[Distributed Game of Life](/projects/gol/)** — Conway's Game of Life as a distributed systems teaching problem.
- **[Recreating Kafka blind](/i-made-a-thing/recreating-kafka-blind/)** — learning event streaming by rebuilding it without reading the source first.
- **[The Krappy Internet](/projects/krappy-internet/)** — experiments in what the network owes you.

Most of the recent project code lives on sourcehut at [git.sr.ht/~ninjapanzer](https://sr.ht/~ninjapanzer/) — kwike, cando, beamlet, and wavelet are all there. Older and mirrored work is at [github.com/developmeh](https://github.com/developmeh) and [github.com/ninjapanzer](https://github.com/ninjapanzer).

## Background

I've been writing software professionally for a long time, across finance, consulting, and product work, in Ruby, Go, Rust, Crystal, Java, and whatever else the problem demanded. I currently work at [8th Light](https://8thlight.com). I studied computer science, including a good deal of logic and ethics coursework that I've never quite been able to put down — a fair amount of what I write about the software industry traces back to it.

## Elsewhere

- [sourcehut](https://sr.ht/~ninjapanzer/) — where the current project code lives
- GitHub — [ninjapanzer](https://github.com/ninjapanzer) (personal) and [developmeh](https://github.com/developmeh) (projects)
- [LinkedIn](https://www.linkedin.com/in/scarronp/)
- [dev.to/paulscoder](https://dev.to/paulscoder)
- [YouTube](https://www.youtube.com/channel/UCpjgLE2xw-19Ty0si6cFCNg)
- [Bluesky](https://bsky.app/profile/developmeh.com)

## Contact

Email me at [paul@scarrone.co](mailto:paul@scarrone.co), or argue with me in [GitHub Discussions](https://github.com/orgs/developmeh/discussions/categories/general) — every article on this site has a thread.
