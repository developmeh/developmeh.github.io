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

Beamlet is a minimum viable OTP over WebAssembly: supervised, capability-sandboxed processes running on [wazero](https://wazero.io), in Go. It exists as the substrate for a self-rewriting LLM harness, the model generates tool code, the supervisor hot-deploys it, and crashes feed back into reflection.

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

That last row is where Beamlet goes further than BEAM. OTP isolates faults; `Caps` also isolates *authority*. A tool the model wrote thirty seconds ago physically cannot open a socket or a file it wasn't granted, the grant is enforced by runtime linkage, not convention, because every process gets its own `wazero.Runtime` whose host functions are closures over that process's capability set.

## The harness that builds itself

On top of the substrate sits an agentic loop where the model has exactly five meta-tools, `create_tool`, `read_tool`, `delete_tool`, `call_tool`, `list_tools`, and no other abilities whatsoever. Every real capability it wants, it must author as a Go `wasip1` program, which the harness compiles, hot-deploys under supervision, and commits as `self-edit(<name>): <reason>`.

Compile errors and supervisor crash events flow back into the loop, so reflection-and-repair happens in-band. The repo is the agent's durable body: `git log` is its changelog, `git revert` its undo.

## Dependencies are a permission, not a convenience

Tool builds run with `-mod=readonly` and `GOPROXY=off`, so model-authored source can never pull a module. Availability is not authorization either, each tool's `manifest.json` carries an `imports` policy naming the non-stdlib packages *that tool* may use, reconciled against its parsed source on every deploy. Declaring an import doesn't grant it; operator approval does. With no terminal attached, nothing can be granted at all: an unattended run writes against the standard library or fails.

Host content enters only where you put it. `-mount <dir>` exposes one directory read-only for that run; the model cannot choose it, widen it, or write to it.

## Pruning on evidence

Every `call_tool` invocation is counted in a local ledger, and those figures, calls, failures, last used, come back from `list_tools`, so the model deletes on evidence rather than vibes. Standard-library tools can be upgraded but not deleted.

## What's deliberately missing

Links and monitors between guests, a `gen_server`-shaped call/reply ABI, CPU metering (wazero has no fuel, runaway loops are killable but not budgeted), and harness self-modification. The model can grow tools; it cannot yet rewrite the loop or the supervisor.

## Links

- [Source Code](https://git.sr.ht/~ninjapanzer/beamlet)

## Architecture decisions

Decisions are recorded as ADRs. [Browse them all](https://git.sr.ht/~ninjapanzer/beamlet/tree/main/item/docs/adr).

- [ADR-0001: Record architecture decisions with MADR](https://git.sr.ht/~ninjapanzer/beamlet/tree/main/item/docs/adr/0001-record-architecture-decisions-with-madr.md)
- [ADR-0002: Go wazero supervised WASM substrate](https://git.sr.ht/~ninjapanzer/beamlet/tree/main/item/docs/adr/0002-go-wazero-supervised-wasm-substrate.md)
- [ADR-0003: Capabilities enforced by per process runtime linkage](https://git.sr.ht/~ninjapanzer/beamlet/tree/main/item/docs/adr/0003-capabilities-enforced-by-per-process-runtime-linkage.md)
- [ADR-0004: Best effort message delivery](https://git.sr.ht/~ninjapanzer/beamlet/tree/main/item/docs/adr/0004-best-effort-message-delivery.md)
- [ADR-0005: Hot swap registers new before draining old](https://git.sr.ht/~ninjapanzer/beamlet/tree/main/item/docs/adr/0005-hot-swap-registers-new-before-draining-old.md)
- [ADR-0006: Self extension via three meta tools and git](https://git.sr.ht/~ninjapanzer/beamlet/tree/main/item/docs/adr/0006-self-extension-via-three-meta-tools-and-git.md)
- [ADR-0007: Openrouter as model gateway](https://git.sr.ht/~ninjapanzer/beamlet/tree/main/item/docs/adr/0007-openrouter-as-model-gateway.md)
- [ADR-0008: Expose tool source readback to the model](https://git.sr.ht/~ninjapanzer/beamlet/tree/main/item/docs/adr/0008-expose-tool-source-readback-to-the-model.md)
- [ADR-0009: Nix flake and direnv dev environment](https://git.sr.ht/~ninjapanzer/beamlet/tree/main/item/docs/adr/0009-nix-flake-and-direnv-dev-environment.md)
- [ADR-0010: Standard library tools as sandboxed WASM](https://git.sr.ht/~ninjapanzer/beamlet/tree/main/item/docs/adr/0010-standard-library-tools-as-sandboxed-wasm.md)
- [ADR-0011: Human designated host mount](https://git.sr.ht/~ninjapanzer/beamlet/tree/main/item/docs/adr/0011-human-designated-host-mount.md)
- [ADR-0012: Interactive sessions and ask user](https://git.sr.ht/~ninjapanzer/beamlet/tree/main/item/docs/adr/0012-interactive-sessions-and-ask-user.md)
- [ADR-0013: Surface tool activity to the operator](https://git.sr.ht/~ninjapanzer/beamlet/tree/main/item/docs/adr/0013-surface-tool-activity-to-the-operator.md)
- [ADR-0014: Tool removal and a protected standard library](https://git.sr.ht/~ninjapanzer/beamlet/tree/main/item/docs/adr/0014-tool-removal-and-a-protected-standard-library.md)
- [ADR-0015: Local usage telemetry](https://git.sr.ht/~ninjapanzer/beamlet/tree/main/item/docs/adr/0015-local-usage-telemetry.md)
- [ADR-0016: Offline read only tool builds](https://git.sr.ht/~ninjapanzer/beamlet/tree/main/item/docs/adr/0016-offline-read-only-tool-builds.md)
- [ADR-0017: Per tool import policy with operator approval](https://git.sr.ht/~ninjapanzer/beamlet/tree/main/item/docs/adr/0017-per-tool-import-policy-with-operator-approval.md)
- [ADR-0018: Interrupts exit everywhere](https://git.sr.ht/~ninjapanzer/beamlet/tree/main/item/docs/adr/0018-interrupts-exit-everywhere.md)
- [ADR-0019: Cache builds and compiled modules](https://git.sr.ht/~ninjapanzer/beamlet/tree/main/item/docs/adr/0019-cache-builds-and-compiled-modules.md)
- [ADR-0020: Testing strategy](https://git.sr.ht/~ninjapanzer/beamlet/tree/main/item/docs/adr/0020-testing-strategy.md)
