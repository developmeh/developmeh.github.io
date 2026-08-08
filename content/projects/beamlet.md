+++
title = "Beamlet"
template = "page.html"
weight = 3
date = 2026-08-07
updated = 2026-08-07
[extra]
schema_type = "TechArticle"
desc = "A minimum viable OTP over WebAssembly - supervised, capability-sandboxed processes on wazero, built as the substrate for a self-rewriting LLM harness"
keywords = "beamlet, OTP, WebAssembly, WASM, wazero, Go, supervisor tree, capability security, sandboxing, self-modifying agent, LLM harness, agentic AI, hot code loading, BEAM, Erlang"
categories = "Projects"
discussion_number = 55
discussion_url = "https://github.com/orgs/developmeh/discussions/55"

[taxonomies]
topics = ["Agentic AI", "WebAssembly", "Capability Security", "Go"]
+++

Beamlet is a minimum viable OTP over WebAssembly: supervised, capability-sandboxed processes running on [wazero](https://wazero.io), in Go. It exists as the substrate for a self-rewriting LLM harness — the model generates tool code, the supervisor hot-deploys it, and crashes feed back into reflection.

## The OTP mapping

Erlang's OTP gives you supervised processes with isolated heaps and mailboxes. Beamlet reproduces that shape on WASM:

| OTP | Beamlet | Mechanism |
|---|---|---|
| process | `process` | one WASM instance + one goroutine |
| heap isolation | linear memory | per-instance, capped via `Caps.MemPages` |
| mailbox | `chan Message` | guest blocks in the `harness.recv` host function |
| `exit(Pid, kill)` | `context.CancelFunc` | cancel interrupts even a hot loop |
| supervisor | `Supervisor` | one-for-one, restart intensity window |
| hot code loading | `Upgrade(spec)` | new instance registers first, old drains and dies |
| ambient authority | **none** | zero-value `Caps` is pure compute |

That last row is where Beamlet goes further than BEAM. OTP isolates faults; `Caps` also isolates *authority*. A tool the model wrote thirty seconds ago physically cannot open a socket or a file it wasn't granted — the grant is enforced by runtime linkage, not convention, because every process gets its own `wazero.Runtime` whose host functions are closures over that process's capability set.

## The harness that builds itself

On top of the substrate sits an agentic loop where the model has exactly five meta-tools — `create_tool`, `read_tool`, `delete_tool`, `call_tool`, `list_tools` — and no other abilities whatsoever. Every real capability it wants, it must author as a Go `wasip1` program, which the harness compiles, hot-deploys under supervision, and commits as `self-edit(<name>): <reason>`.

Compile errors and supervisor crash events flow back into the loop, so reflection-and-repair happens in-band. The repo is the agent's durable body: `git log` is its changelog, `git revert` its undo.

## Dependencies are a permission, not a convenience

Tool builds run with `-mod=readonly` and `GOPROXY=off`, so model-authored source can never pull a module. Availability is not authorization either — each tool's `manifest.json` carries an `imports` policy naming the non-stdlib packages *that tool* may use, reconciled against its parsed source on every deploy. Declaring an import doesn't grant it; operator approval does. With no terminal attached, nothing can be granted at all: an unattended run writes against the standard library or fails.

Host content enters only where you put it. `-mount <dir>` exposes one directory read-only for that run; the model cannot choose it, widen it, or write to it.

## Pruning on evidence

Every `call_tool` invocation is counted in a local ledger, and those figures — calls, failures, last used — come back from `list_tools`, so the model deletes on evidence rather than vibes. Standard-library tools can be upgraded but not deleted.

## What's deliberately missing

Links and monitors between guests, a `gen_server`-shaped call/reply ABI, CPU metering (wazero has no fuel — runaway loops are killable but not budgeted), and harness self-modification. The model can grow tools; it cannot yet rewrite the loop or the supervisor.

## Links

- [Source Code](https://git.sr.ht/~ninjapanzer/beamlet)
- Architecture decisions are recorded as MADRs in `docs/adr/` — twenty of them, covering the WASM substrate choice, capability enforcement, import policy, and testing strategy
