# Portfolio & Blog Tracking: kwik-e-mart (LLM Agent Orchestration)

## Overview

Building portfolio page and blog content around kwik-e-mart - a Unix-philosophy event-driven daemon for orchestrating LLM agents on coding tasks.

**Related Projects:**
- `kwik-e-mart` - Core daemon (watch, dispatch, consume, daemon)
- `feature-forge` - Implementation using kwike for automated code review pipeline
- `beads` - Issue tracking system that feeds the pipeline

---

## Portfolio Page

### Concept
A dedicated page showcasing the architecture, design decisions, and real-world usage of the multi-agent coding pipeline.

### Sections to Include
- [ ] **Hero**: "Unix Philosophy Meets LLM Orchestration"
- [ ] **Problem Statement**: Why visual workflow tools don't fit
- [ ] **Architecture Diagram**: watch → dispatch → daemon → consume → agents
- [ ] **Key Innovations**:
  - Uniformed agents (role-based markdown prompts)
  - Permission isolation (`--allowedTools` per agent)
  - Crash-only design (append-only logs, cursor-based recovery)
  - Event-driven handoff (JSON contracts between agents)
- [ ] **Demo/Screenshots**: feature-forge pipeline in action
- [ ] **Code Samples**: Consumer config, uniform template, event flow
- [ ] **Comparison**: vs n8n, LangChain, OpenClaw
- [ ] **Links**: GitHub repos, related blog posts

---

## Blog Post Ideas

### 1. "Unix Philosophy for LLM Orchestration"
_Status: IDEA_

**Thesis**: Visual workflow builders are wrong for AI agents. Text protocols and pipes are the answer.

**Key Points**:
- `watch | dispatch` as Unix filter pattern
- Why JSON-lines beats nested workflow JSON
- Composability: swap agents without rewiring
- LLM-native: agents can write their own configs

**Hook**: "n8n has 400 integrations. I built something with zero UI that's more powerful for coding agents."

---

### 2. "Uniformed Agents: Role-Based Prompts as Configuration"
_Status: IDEA_

**Thesis**: Each agent role needs different capabilities. The "uniform" pattern makes this explicit.

**Key Points**:
- Refiner (read-only, explores) vs Implementer (writes code) vs Reviewer (verifies)
- Markdown templates with Go template syntax
- Permission boundaries in config, not code
- Why this beats "one agent does everything"

**Hook**: "What if your AI agents had job descriptions and security clearances?"

---

### 3. "Permission Isolation for AI Agents"
_Status: IDEA_

**Thesis**: Least-privilege for LLMs is a solved problem. Here's how.

**Key Points**:
- `--allowedTools` scopes what Claude can do
- Subprocess isolation = crash boundaries
- Audit trail via append-only logs
- Enterprise compliance angle

**Hook**: "Your AI agent doesn't need root access. Here's how to give it exactly what it needs."

---

### 4. "Crash-Only Design for AI Pipelines"
_Status: IDEA_

**Thesis**: Your LLM pipeline shouldn't need graceful shutdown. Design for crashes.

**Key Points**:
- Append-only JSONL for events
- Cursor-based consumer recovery
- Why transactions are overkill
- Real example: daemon dies mid-job, consumer resumes

**Hook**: "I kill my AI agents randomly. They don't lose work."

---

### 5. "Building a Code Review Loop with Claude Agents"
_Status: IDEA_

**Thesis**: End-to-end walkthrough of feature-forge.

**Key Points**:
- Bead (issue) → Refiner → Implementer → Reviewer → Retry/Complete
- JSON contracts as agent handoff
- The orchestrator routing logic
- What worked, what didn't (honest retrospective)

**Hook**: "I let three AI agents argue about my code. Here's what happened."

---

## Research Notes

### Architecture Highlights
- Single Go binary with subcommands
- UUIDv7 for time-sortable event IDs
- Unix socket for local daemon (no network overhead)
- Durable consumer with inbox.jsonl and ack.jsonl

### Differentiators from Competition
| vs | kwike advantage |
|----|-----------------|
| n8n | Text-based, LLM can generate configs |
| LangChain | Subprocess isolation, not in-process |
| OpenClaw | Multi-agent with role separation |
| Devin | Open source, self-hosted, auditable |

### Quotes/Angles to Explore
- "The best orchestration is the one your agents can modify"
- "Visual workflows are write-once. Text configs are iterate-forever."
- "I don't trust any single AI agent. I trust a pipeline of specialists."

---

## Next Steps

- [ ] Pick first blog post to write
- [ ] Run blog-interviewer to gather context
- [ ] Draft with manson-dev-essayist or senior-engineer-voice
- [ ] Create portfolio page layout
- [ ] Add to site navigation

---

## Session Log

**2026-03-10**: Initial tracking file created. Brainstormed 5 blog post ideas and portfolio page structure.
