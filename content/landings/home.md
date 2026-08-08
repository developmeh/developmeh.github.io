+++
title = "Home"

# A draft section is only loaded if the `--drafts` flag is passed to `zola build`, `zola serve` or `zola check`.
draft = false

path = "/"
date = 2025-06-06
updated = 2026-08-07
[extra]
# NOTE: was `description`, which no template reads - the homepage was silently
# falling back to the site-wide description. The key templates look for is `desc`.
desc = "The workshop of Paul Scarrone - essays, devlogs and working code on agentic AI workflows, developer experience, shell testing and software architecture. That stuff you don't get to do at work."
keywords = "Paul Scarrone, developmeh, agentic AI, LLM orchestration, kwike, beamlet, wavelet, developer experience, Nix, BATS, bash testing, software architecture, devlog"
enable_discussions = false
discussion_number = 26
discussion_url = "https://github.com/orgs/developmeh/discussions/26"
+++

<div class="hero-section">
  <img src="https://github.com/developmeh.png" class="hero-logo" alt="Developmeh Logo">
  <div class="hero-content">
    <h2>Developmeh</h2>
    <div class="subtitle">Develop ¯\_(ツ)_/¯</div>
    <p>Contained within are harebrained ideas that have no commercial value... still here... you are one of the special ones.</p>
  </div>
</div>

<div class="home-layout">
  <div class="main-column">

<div class="callout info">
  <span class="callout-title">Perspective</span>
  I have done a lot of software engineering in my life and after all that time I have come to appreciate an industry in constant evolution.

  I, though, seem to stand as a fixed point, arriving to accomplish a specific task and obstinately refusing to become a tradesman.
</div>

<div class="callout success">
  <span class="callout-title">Welcome</span>
  For those of you who have a craft and participate in a creative act on the regular, I salute you. Your bravery is what I idolize. In pursuit of of some kind of self-idolatry I create toys to expand my knowledge and forgive myself for being a shill.

  But who cares? Welcome to my workshop!
</div>

<div class="callout warning">
  <span class="callout-title">Standards</span>
  This is a safe space for all ideas; the point is to have fun with it; you don't wanna write tests...suuuuure....

  GET THE HELL OUT! I am not some kind of heathen. I have standards, bud.
</div>

<div class="card-stack">
  <div class="card">
    <h3>Devlogs</h3>

- [05-04-2026 kwike (Fan-Out Gets a Collector)](/i-made-a-thing/kwike-llm-first-agentic-workflow-composition#05-04-2026)
- [04-04-2026 kwike (v0.1.0)](/i-made-a-thing/kwike-llm-first-agentic-workflow-composition#04-04-2026)
- [03-04-2026 kwike (Live Reload and Test Stability)](/i-made-a-thing/kwike-llm-first-agentic-workflow-composition#03-04-2026)
- [29-03-2026 kwike (v0.0.15 - Refining Options)](/i-made-a-thing/kwike-llm-first-agentic-workflow-composition#29-03-2026)
- [15-03-2026 kwike (v0.0.5 - Making tools for robots)](/i-made-a-thing/kwike-llm-first-agentic-workflow-composition#15-03-2026)
- [06-03-2026 The AI Diaries (My Own Ideas)](/soft-wares/ai-diaries#06-03-2026)
- [01-03-2026 The AI Diaries (Limitless Abstraction)](/soft-wares/ai-diaries#01-03-2026)
- [22-02-2026 The AI Diaries (Unboudned Growth)](/soft-wares/ai-diaries#22-02-2026)
- [14-02-2026 Catalyst Orchestrator (The Daemon Creates Steps at Runtime)](/i-made-a-thing/catalyst-orchestrator#14-02-2026)
- [13-02-2026 Catalyst Orchestrator (The Daemon Parses, The Daemon Routes)](/i-made-a-thing/catalyst-orchestrator#13-02-2026)
- [11-02-2026 Catalyst Orchestrator (The Haiku Decides)](/i-made-a-thing/catalyst-orchestrator#11-02-2026)
- [08-02-2026 The AI Diaries (80/20 Rule Still Applies)](/soft-wares/ai-diaries#08-02-2026)
- [03-02-2026 The AI Diaries (Composable Code Future)](/soft-wares/ai-diaries#03-02-2026)
- [02-02-2026 The AI Diaries (Browser Extensions and Manifest Woes)](/soft-wares/ai-diaries#02-02-2026)
- [02-02-2026 Krappy Internet (WASM is the way in)](/projects/krappy-internet/#02-02-2026)
- [02-02-2026 Rust Dancing Banana (SSE vs Chunked Encoding)](/i-made-a-thing/rust-streaming-banana-dancer-server-sent-events#02-02-2026)
- [02-02-2026 Rust Dancing Banana (Rust's Async Streams)](/i-made-a-thing/rust-streaming-banana-dancer-server-sent-events#02-02-2026-1)
- [01-02-2026 Rust Dancing Banana (Compile-Time Frame Embedding)](/i-made-a-thing/rust-streaming-banana-dancer-server-sent-events#01-02-2026)
- [01-02-2026 Rust Dancing Banana (Nix for Rust)](/i-made-a-thing/rust-streaming-banana-dancer-server-sent-events#01-02-2026-1)
- [30-01-2026 The AI Diaries (Comparing Coding Models)](/soft-wares/ai-diaries#30-01-2026)
- [28-01-2026 The AI Diaries (Eager Intern Problem)](/soft-wares/ai-diaries#28-01-2026)
- [27-01-2026 The AI Diaries (Throughput over Precision)](/soft-wares/ai-diaries#27-01-2026)
- [20-01-2026 The AI Diaries (AI-generated Code Debt)](/soft-wares/ai-diaries#20-01-2026)
- [14-07-2025 This Week's Crazy Idea (Just build binaries)](/i-made-a-thing/this-weeks-crazy#14-07-2025)
- [13-07-2025 This Week's Crazy Idea (Everything is a Stream)](/i-made-a-thing/this-weeks-crazy#13-07-2025)
- [21-06-2025 This Week's Crazy Idea (OpenTelemetry and the question of ditching logs)](/i-made-a-thing/this-weeks-crazy#21-06-2025)
- [15-06-2025 This Week's Crazy Idea (WebRTC and what not to ask AI to do)](/i-made-a-thing/this-weeks-crazy#15-06-2025)
- [14-06-2025 This Week's Crazy Idea (WebRTC, NAT Traversals, and American Manufacturing)](/i-made-a-thing/this-weeks-crazy#14-06-2025)
- [08-06-2025 This Week's Crazy Idea (Decentralized DynamicDns Krappy-DynDns)](/i-made-a-thing/this-weeks-crazy#08-06-2025)
- [24-02-2025 Krappy Internet (Working around the browser)](/projects/krappy-internet/#24-02-2025)
- [11-02-2025 Krappy Internet (An Ideal World)](/projects/krappy-internet/#11-02-2025)
- [06-02-2025 Krappy Internet (Getting over the Browser)](/projects/krappy-internet/#06-02-2025)
- [31-01-2025 Streaming Dancing Banana (Nix Cross Platform Improvements)](/i-made-a-thing/ruby-streaming-banana-dancer/#31-01-2025)
- [29-01-2025 The Krappy Internet (Protocol Servers)](/projects/krappy-internet/#devlog)
- [27-01-2025 Streaming Dancing Banana (Nix Build and Deploy to K8s)](/i-made-a-thing/ruby-streaming-banana-dancer/#27-01-2025)
- [21-01-2025 Distributed Game of Life (Debugging stats)](/projects/gol/#21-01-2025)
- [20-01-2025 Distributed Game of Life (Stats)](/projects/gol/#20-01-2025)
- [19-01-2025 Distributed Game of Life (Profiling)](/projects/gol/#19-01-2025)
- [15-01-2025 Distributed Game of Life (Getting Started](/projects/gol/#15-01-2025)
- [25-12-2024 Krappy Kafka (k0s Deployment)](/i-made-a-thing/recreating-kafka-blind/#25-12-2024)
- [22-12-2024 Krappy Kafka (Handler Cleanup and Func Interface)](/i-made-a-thing/recreating-kafka-blind/#22-12-2024)
- [05-11-2024 Krappy Kafka (Shared Consumer Groups)](/i-made-a-thing/recreating-kafka-blind/#22-12-2024)
- [11-09-2024 Krappy Kafka (Connection Handshake and Context State)](/i-made-a-thing/recreating-kafka-blind/#11-09-2024)
- [10-09-2024 Krappy Kafka (LSM Compaction and PebbleDB)](/i-made-a-thing/recreating-kafka-blind/#10-09-2024)

</div>

  <div class="card">
    <h3>Articles</h3>

- [Cando](/projects/cando)
- [Catalyst: An Orchestrator That Stopped Asking and Started Deciding](/i-made-a-thing/catalyst-orchestrator)
- [The AI Diaries](/soft-wares/ai-diaries)
- [Copying Life](/devex/copying-life)
- [This Week's Crazy Idea](/i-made-a-thing/this-weeks-crazy)
- [The Krappy Internet](/projects/krappy-internet)
- [Go Generics Example](/software-architecture/go-generics-example)
- [An Internet of Changing Morality](/terms-and-afflictions/software_ethics)
- [Am I the Crazy One?](/soft-wares/aitco)
- [Not Invented Here](/soft-wares/nih)
- [The Software Delivery Trap](/terms-and-afflictions/software_delivery)
- [We Do Delivery Now Eh?](/soft-wares/the-future-is-delivery)
- [Decoupling Patterns in Ruby: Overview](/software-architecture/decoupling_patterns_in_ruby_overview)
- [End User Languor Agreement](/terms-and-afflictions/eula)
- [CI Over CD](/devex/ci_cd)
- [Ruby Dancing ANSI Banana for Curl](/i-made-a-thing/ruby-streaming-banana-dancer)
- [Wavelet](/projects/wavelet/)
- [Beamlet](/projects/beamlet/)
- [kwike: LLM-First Agentic Workflow Composition](/i-made-a-thing/kwike-llm-first-agentic-workflow-composition)
- [Automatic Programming: Iteration 4](/devex/automatic-programming-iteration-4)
- [BATS - Testing Bash Like You Mean It](/tech-dives/bats-testing-bash-like-you-mean-it)
- [Keep Your Eyes on the IDE, and Your Robots on the Tickets](/i-made-a-thing/keep-your-eyes-on-the-ide-and-your-robots-on-the-tickets)
- [Agentic Patterns: Elements of Reusable Context-Oriented Determinism](/soft-wares/agentic-patterns-elements-of-reusable-context-oriented-determinism)
- [Just Forget About Owning Code](/soft-wares/just-forget-about-owning-code)
- [Rust Dancing ANSI Banana with Server-Sent Events](/i-made-a-thing/rust-streaming-banana-dancer-server-sent-events)
- [A Deterministic Box for Non-Deterministic Engines](/tech-dives/a-deterministic-box-for-non-deterministic-engines)
- [Claude or Clod](/soft-wares/claude-or-clod)
- [The Magic of Stubbing sh](/i-made-a-thing/the-magic-of-stubbing-sh)
- [Sufficient Complexity](/soft-wares/sufficient-complexity)
- [Do Devs Really Do DevOps in your Org?](/soft-wares/do-devs-really-do-devops)
- [The Good Sergeant](/soft-wares/the-good-sergeant)
- [Creative Impostor Syndrome](/soft-wares/creative-impostor-syndrome)
- [The Perfect Dev Env Part 1](/devex/the-perfect-dev-env/)
- [Distributed Game of Life](/projects/gol/)
- [Krappy Kafka](/i-made-a-thing/recreating-kafka-blind)

</div>
</div>

  </div>
  <div class="side-column">

<div class="callout info">
  <span class="callout-title">Connect</span>
  
### The code lives on sourcehut:
- [git.sr.ht/~ninjapanzer](https://sr.ht/~ninjapanzer/) — where the current work is: [kwike](https://git.sr.ht/~ninjapanzer/kwike), [cando](https://git.sr.ht/~ninjapanzer/cando), [beamlet](https://git.sr.ht/~ninjapanzer/beamlet), [wavelet](https://git.sr.ht/~ninjapanzer/wavelet)

### And some of it on GitHub:
- <svg class="github-link-icon" height="16" width="16" viewBox="0 0 16 16" fill="currentColor"><path d="M8 0C3.58 0 0 3.58 0 8c0 3.54 2.29 6.53 5.47 7.59.4.07.55-.17.55-.38 0-.19-.01-.82-.01-1.49-2.01.37-2.53-.49-2.69-.94-.09-.23-.48-.94-.82-1.13-.28-.15-.68-.52-.01-.53.63-.01 1.08.58 1.23.82.72 1.21 1.87.87 2.33.66.07-.52.28-.87.51-1.07-1.78-.2-3.64-.89-3.64-3.95 0-.87.31-1.59.82-2.15-.08-.2-.36-1.02.08-2.12 0 0 .67-.21 2.2.82.64-.18 1.32-.27 2-.27.68 0 1.36.09 2 .27 1.53-1.04 2.2-.82 2.2-.82.44 1.1.16 1.92.08 2.12.51.56.82 1.27.82 2.15 0 3.07-1.87 3.75-3.65 3.95.29.25.54.73.54 1.48 0 1.07-.01 1.93-.01 2.2 0 .21.15.46.55.38A8.013 8.013 0 0016 8c0-4.42-3.58-8-8-8z"/></svg>[https://github.com/developmeh](https://github.com/developmeh)
- <svg class="github-link-icon" height="16" width="16" viewBox="0 0 16 16" fill="currentColor"><path d="M8 0C3.58 0 0 3.58 0 8c0 3.54 2.29 6.53 5.47 7.59.4.07.55-.17.55-.38 0-.19-.01-.82-.01-1.49-2.01.37-2.53-.49-2.69-.94-.09-.23-.48-.94-.82-1.13-.28-.15-.68-.52-.01-.53.63-.01 1.08.58 1.23.82.72 1.21 1.87.87 2.33.66.07-.52.28-.87.51-1.07-1.78-.2-3.64-.89-3.64-3.95 0-.87.31-1.59.82-2.15-.08-.2-.36-1.02.08-2.12 0 0 .67-.21 2.2.82.64-.18 1.32-.27 2-.27.68 0 1.36.09 2 .27 1.53-1.04 2.2-.82 2.2-.82.44 1.1.16 1.92.08 2.12.51.56.82 1.27.82 2.15 0 3.07-1.87 3.75-3.65 3.95.29.25.54.73.54 1.48 0 1.07-.01 1.93-.01 2.2 0 .21.15.46.55.38A8.013 8.013 0 0016 8c0-4.42-3.58-8-8-8z"/></svg>[https://github.com/ninjapanzer](https://github.com/ninjapanzer)

### Who's writing this
[About Paul Scarrone](/about)

### Correspondence
Please address all hate mail [here](https://github.com/orgs/developmeh/discussions/categories/general)
</div>

  </div>
</div>
