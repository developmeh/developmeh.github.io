+++
title = "Home"

# A draft section is only loaded if the `--drafts` flag is passed to `zola build`, `zola serve` or `zola check`.
draft = false

path = "/"
date = 2025-06-06
updated = 2026-08-08
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
    <h1>Developmeh</h1>
    <div class="subtitle">Develop ¯\_(ツ)_/¯</div>
    <p>The workshop of Paul Scarrone. Harebrained ideas, built for the joy of it: agentic AI workflows, developer experience, shell testing, and software architecture, taken apart until they explain themselves.</p>
    <div class="hero-links">
      <a href="/about" class="primary">About Paul</a>
      <a href="/projects">Projects</a>
      <a href="/topics">Topics</a>
      <a href="/rss.xml">RSS</a>
    </div>
  </div>
</div>

<div class="home-layout">
  <div class="main-column">

<div class="card">
  <h2>Why this site</h2>

I have done a lot of software engineering in my life, long enough to appreciate an industry in constant evolution. I keep arriving with a specific task in hand and obstinately refusing to become a tradesman. The projects here are toys in the best sense, built to expand my knowledge and keep the craft sharp.

This is a safe space for all ideas; the point is to have fun with it. And every idea here ships with tests. I have standards, bud.

</div>

<div class="card">
  <h2>Recent writing</h2>

<ul class="post-list">
  <li><a href="/tech-dives/judgement-capable-circuits">Judgement-Capable Circuits: What You Write Loops Out Of</a></li>
  <li><a href="/tech-dives/orchestrating-agents-on-bedrock">Orchestrating Agents on Amazon Bedrock: When You Cannot Push, Make Completion the Wake</a></li>
  <li><a href="/tech-dives/techniques-that-keep-paying-off">Five Techniques That Keep Paying Off When You Build for LLMs</a></li>
  <li><a href="/tech-dives/testing-shell-scripts">Testing Shell Scripts: BATS, TAP, and Treating Bash as a Runtime</a></li>
  <li><a href="/tech-dives/agentic-ai-engineering">Agentic AI Engineering: Building Determinism Around a Non-Deterministic Engine</a></li>
  <li><a href="/i-made-a-thing/kwike-llm-first-agentic-workflow-composition">kwike: LLM-First Agentic Workflow Composition</a></li>
  <li><a href="/devex/automatic-programming-iteration-4">Automatic Programming: Iteration 4</a></li>
  <li><a href="/tech-dives/bats-testing-bash-like-you-mean-it">BATS - Testing Bash Like You Mean It</a></li>
  <li><a href="/i-made-a-thing/keep-your-eyes-on-the-ide-and-your-robots-on-the-tickets">Keep Your Eyes on the IDE, and Your Robots on the Tickets</a></li>
  <li><a href="/soft-wares/agentic-patterns-elements-of-reusable-context-oriented-determinism">Agentic Patterns: Elements of Reusable Context-Oriented Determinism</a></li>
</ul>

<details class="archive">
<summary>More articles</summary>

- [Beamlet](/projects/beamlet)
- [Wavelet](/projects/wavelet)
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

</details>

</div>

<div class="card">
  <h2>Devlogs</h2>

<ul class="post-list">
  <li><a href="/i-made-a-thing/kwike-llm-first-agentic-workflow-composition#05-04-2026">kwike (Fan-Out Gets a Collector)</a><time datetime="2026-04-05">05-04-2026</time></li>
  <li><a href="/i-made-a-thing/kwike-llm-first-agentic-workflow-composition#04-04-2026">kwike (v0.1.0)</a><time datetime="2026-04-04">04-04-2026</time></li>
  <li><a href="/i-made-a-thing/kwike-llm-first-agentic-workflow-composition#03-04-2026">kwike (Live Reload and Test Stability)</a><time datetime="2026-04-03">03-04-2026</time></li>
  <li><a href="/soft-wares/ai-diaries#06-03-2026">The AI Diaries (My Own Ideas)</a><time datetime="2026-03-06">06-03-2026</time></li>
  <li><a href="/i-made-a-thing/catalyst-orchestrator#14-02-2026">Catalyst Orchestrator (The Daemon Creates Steps at Runtime)</a><time datetime="2026-02-14">14-02-2026</time></li>
  <li><a href="/i-made-a-thing/rust-streaming-banana-dancer-server-sent-events#02-02-2026">Rust Dancing Banana (SSE vs Chunked Encoding)</a><time datetime="2026-02-02">02-02-2026</time></li>
</ul>

<details class="archive">
<summary>Full devlog history</summary>

- [29-03-2026 kwike (v0.0.15 - Refining Options)](/i-made-a-thing/kwike-llm-first-agentic-workflow-composition#29-03-2026)
- [15-03-2026 kwike (v0.0.5 - Making tools for robots)](/i-made-a-thing/kwike-llm-first-agentic-workflow-composition#15-03-2026)
- [01-03-2026 The AI Diaries (Limitless Abstraction)](/soft-wares/ai-diaries#01-03-2026)
- [22-02-2026 The AI Diaries (Unboudned Growth)](/soft-wares/ai-diaries#22-02-2026)
- [13-02-2026 Catalyst Orchestrator (The Daemon Parses, The Daemon Routes)](/i-made-a-thing/catalyst-orchestrator#13-02-2026)
- [11-02-2026 Catalyst Orchestrator (The Haiku Decides)](/i-made-a-thing/catalyst-orchestrator#11-02-2026)
- [08-02-2026 The AI Diaries (80/20 Rule Still Applies)](/soft-wares/ai-diaries#08-02-2026)
- [03-02-2026 The AI Diaries (Composable Code Future)](/soft-wares/ai-diaries#03-02-2026)
- [02-02-2026 Rust Dancing Banana (Rust's Async Streams)](/i-made-a-thing/rust-streaming-banana-dancer-server-sent-events#02-02-2026-1)
- [01-02-2026 Rust Dancing Banana (Compile-Time Frame Embedding)](/i-made-a-thing/rust-streaming-banana-dancer-server-sent-events#01-02-2026)
- [01-02-2026 Rust Dancing Banana (Nix for Rust)](/i-made-a-thing/rust-streaming-banana-dancer-server-sent-events#01-02-2026-1)
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

</details>

</div>

  </div>
  <div class="side-column">

<div class="card">
  <h2>The code</h2>

Current work lives on sourcehut at [~ninjapanzer](https://sr.ht/~ninjapanzer/):

- [kwike](https://git.sr.ht/~ninjapanzer/kwike)
- [cando](https://git.sr.ht/~ninjapanzer/cando)
- [beamlet](https://git.sr.ht/~ninjapanzer/beamlet)
- [wavelet](https://git.sr.ht/~ninjapanzer/wavelet)

And some of it on GitHub:

- [developmeh](https://github.com/developmeh)
- [ninjapanzer](https://github.com/ninjapanzer)

</div>

<div class="card">
  <h2>Who's writing this</h2>

Paul Scarrone. Staff-level engineer, AWS Community Builder, and serial builder of odd little tools, several of which now have real users.

[More about Paul](/about)

</div>

<div class="card">
  <h2>Correspondence</h2>

Questions, corrections, hate mail, and the occasional strongly worded opinion all land in [GitHub Discussions](https://github.com/orgs/developmeh/discussions/categories/general).

</div>

  </div>
</div>
