# Future platform: self-hosting, idle, identity, and local-first data (Lean, phase 3)

## 0. What this document is

Phase 1 (`plans/lean-browser.md`) is a memory-first browser: a short-lived loader compiles a page into a read-only rkyv page file and a small renderer paints it. It is being built now and this document does not change it. Phase 2 (`plans/wasm-apps.md`) turns every card into a WASI app that emits Lui trees, and adds `lean://` over QUIC, a content store, log/KV channels and a sync ladder.

This document is the **future-platform layer**. It keeps phase 2's app model (Lui, the WIT world, card lifecycle, stacks, page-file deltas) and amends phase 2 wherever its networking, identity and delivery choices contradict four principles that are now fixed:

1. **The platform exists to enable self-hosting and reliable delivery.** A person must be able to run their own origin at home, and other people must be able to reach it without a domain name, a certificate authority, or a cloud account.
2. **Rich apps that idle reliably.** Idle means near-zero CPU, **zero wakeups and zero network traffic**. Apps have no timers. The host schedules everything, and "phoning home" is a capability the user grants and can see.
3. **Leanness stays.** The phase-1 and phase-2 `Private_Dirty` gates stay, and budgets now also apply to the home server (a Pi or an old laptop), to bytes on the wire, and to dependency count.
4. **Delivery is stateful, identity is a platform concern, user data is local.** Apps are installed, not visited; they work offline indefinitely; updates are pinned and reversible. The host holds keys; apps never see credentials. User data does not leak identity to servers that do not need it.

The author's data point: "burl" (Crystal + raylib) idles at ~1% of one core and ~80 MB RSS, from raylib's render loop, GL context and driver memory, font atlases and the Boehm GC heap. None of those costs can exist here by construction: apps do not render (P3 in phase 2), the renderer is CPU-only with a strip buffer, fonts are mmap'd, and frozen cards are files. §8 turns that into a CI test.

### 0.1 What this supersedes in `wasm-apps.md`

| Phase-2 section | Status | Why |
|---|---|---|
| §5.1 `lean://<host>[:port]/…` over QUIC with TLS/CA certificates and 0-RTT "for known origins" | **Superseded** by §5 here | Origins are addressed by public key, not DNS host + CA. 0-RTT tickets link connections, so they are allowed only toward origins the user is already authenticated to, and never for anonymous blob fetch. |
| §5.1 "Manifests are the only mutable thing… updates are atomic… old cards keep pinned hashes until restarted" | **Superseded** by §6 | Updates are user-controlled: the host never advances an install without the user's action; pin and rollback are first-class. Phase 2's behaviour is a silent update with a delay. |
| §5.2 wake windows (15 min background / 2 min foreground) | **Amended** by §7 | Timers that fire with no granted capability are wakeups and traffic while idle. Wake windows exist only for apps holding a user-granted `background-sync` capability, and they are visible in the network-activity panel. |
| §5.3 transport fallback ladder (QUIC → SSE → WebSocket → polling, all to one HTTPS host) | **Superseded** by §5.3 | The ladder becomes direct-by-key → hole-punched → relayed, plus an HTTPS *mirror* level that serves only content-addressed blobs and encrypted channel entries. There is no HTTPS level that requires an identity-bearing session. |
| §5.4 scope `user` "synced across the user's devices via the origin" | **Amended** by §6 | `user`-scope channels are end-to-end encrypted between the user's devices. The origin or relay stores ciphertext. The origin reads a channel only if the app holds the `origin-read` capability and the user granted it. |
| §5.5 `fetch` proxied through the origin's gateway | **Superseded** by §6.4 | The gateway seeing every app request is exactly the leak this layer removes. Third-party fetch goes direct from the broker (anonymously); proxying through the origin is an opt-in capability with a stated reason. |
| §5.6 gateway as an HTTPS server the origin "must run" | **Amended** by §5 | The reference home node serves `lean` natively; the HTTPS reverse bridge remains optional and is only for legacy browsers. |
| §6 signing: developer key bound to origin via `/.well-known/lean-keys`, TOFU | **Superseded** by §4 and §6 | The origin *is* a key. There is nothing to bind. App-publisher keys are separate from origin keys and are pinned per install. |
| §6 fingerprinting: "per-(app, origin) random device id" | **Retained**, derived deterministically from the identity hierarchy in §4 so it survives reinstall of the host but not uninstall of the app. |
| §7 sync-broker baseline ≤ 1.0 MB (quinn + rustls) | **Revised** to ≤ 2.5 MB with iroh, gated by the F0 experiment (§9). |
| R7 "multi-user shared data needs identity" | **Resolved** here (§4). |
| W3 exit criteria (two devices converge "via the origin", broker holds no connection after 30 s idle) | **Amended**: convergence must be through ciphertext the origin cannot read, and "no connection" becomes "no endpoint, no wakeups, no bytes" (§7). |

Everything else in phase 2 stands, including P1 (every restriction cites memory or network). This layer adds a third admissible justification: **identity leakage** (a feature would reveal a user's identity, or link two of their identities, to a party that does not need it). `DECISIONS.md` entries may cite `because: memory|network|identity`.

## 1. Survey: transport and addressing

All footprints are `Private_Dirty`-style estimates unless marked measured. "Fit" is against the principles above and the budgets in §3.

**iroh (n0; iroh, iroh-blobs, iroh-docs, iroh-gossip).** Rust. QUIC (a quinn fork) with dial-by-Ed25519-public-key, NAT hole punching, relays as fallback, discovery by DNS or by pkarr records on the BitTorrent mainline DHT. iroh core reached 1.0 in June 2026 per third-party reports (the project site was unreachable from this environment; treat the date as reported). iroh-blobs (BLAKE3 + bao verified streaming, range requests, any node can serve a hash), iroh-docs (multi-writer key-value with range-based set reconciliation) and iroh-gossip are still pre-1.0 and iterating. Licence: MIT/Apache-2.0. Footprint: a tokio current-thread runtime plus one quinn endpoint plus rustls is about 2–4 MB dirty (estimate; F0 measures it). There is an open issue (#4549) about anonymous memory growth under path churn on long-running endpoints; relevant to the home node, not the client, which tears its endpoint down. Fit: dial-by-key is exactly the addressing model the self-hosting principle needs; relays are stateless and cheap; hole punching is the thing nobody should write twice. Misfit: an iroh endpoint keeps a relay connection alive with periodic pings, which is wakeups and bytes, so an idle client must *not* hold an endpoint (§7); pkarr publishing requires periodic republish, which the home node does but clients never do; the default mainline-DHT discovery leaks node-id-to-relay mapping publicly (acceptable for a home origin, unacceptable for a client). **Recommendation: adopt iroh core for the broker and the home node; adopt iroh-blobs for content-addressed transfer; borrow ideas from iroh-docs (range-based reconciliation) but do not adopt it as the channel store (its data model is unencrypted key-value with author keys, which are global identities); do not adopt iroh-gossip (a gossip mesh is continuous background traffic).**

**libp2p (rust-libp2p).** Modular transport stack: multiple transports (TCP, QUIC, WebRTC, WebTransport), Noise or TLS handshakes, Kademlia DHT, gossipsub, relay v2 and DCUtR hole punching. Mature, well maintained, MIT. Footprint: one task per connection, many optional protocols; a minimal QUIC-only build is plausible at 4–8 MB dirty but the ecosystem's defaults (Kademlia, identify, ping) all generate idle traffic. Fit: peer ids are keys, which is right. Misfit: it is a toolkit for building a *network*; we need one transport with one addressing rule. Dependency count is high. **Avoid; borrow the relay-v2/DCUtR ideas already present in iroh.**

**Hypercore / Holepunch (Hyperswarm, Autobase, Pear, Bare).** Append-only signed logs, a DHT for discovery by topic, hole punching, multi-writer via Autobase. Active and productised (Keet). MIT/Apache. The runtime is JavaScript (Bare), which rules it out as a dependency for a Rust host with a `Private_Dirty` gate. Fit: Hypercore's "one writer per log, replicate by (key, seq)" is exactly phase 2's log channel with a signature per entry; Hyperswarm's "connect by topic hash" is a good pattern for group sync without revealing the group key. **Borrow ideas (signed per-writer logs, topic = hash(secret)); avoid the code.**

**IPFS / Bitswap (Kubo, Helia).** Content addressing with CIDs, Bitswap block exchange, Kademlia routing. Mature, but Kubo's idle bandwidth and memory are notorious (long-standing issues about idle traffic from DHT and wantlists; an unbounded Bitswap memory leak advisory in August 2026). Fit: the content-addressing principle, nothing else. Misfit: continuous DHT participation is the opposite of idle; blocks are small and the routing chatter is large. **Avoid; BLAKE3 + bao in iroh-blobs gives us verified content addressing without the network.**

**Veilid.** Rust P2P framework from cDc, DHT plus private routing (onion-like). 0.5.x as of July 2026; the maintainers still call it beta. Footprint unknown, but it maintains a routing table and route keepalives by design, which is idle traffic. Fit: strong anonymity ambitions. Misfit: continuous participation; large surface; not needed for "reach a known key". **Avoid for now; re-evaluate if a metadata-hiding transport becomes a requirement.**

**Tailscale / Headscale / WireGuard.** Overlay VPN keyed by WireGuard public keys, with NAT traversal (DERP relays; iroh's relays descend from DERP). Headscale is actively released (v0.29.x, July 2026). Fit: the best-proven "reach my own machine by key" system for people, and a legitimate way for a household to reach a home node without any Lean-specific relay. Misfit: WireGuard keepalives are periodic traffic; the overlay is per-device, not per-app or per-origin; identity is a node key shared across every service. **Do not depend on it; document that a home node behind Tailscale/Headscale is a supported deployment (the direct-address path in §5.3 works over it).**

**Noise protocol.** Handshake framework (X25519, ChaChaPoly, BLAKE2), tiny, no certificates. Used by WireGuard and libp2p. Fit: ideal for device-to-device pairing and for encrypting channel entries end-to-end when the transport's TLS terminates at a relay. Misfit: QUIC mandates TLS 1.3, so Noise cannot be the QUIC handshake. **Adopt `snow` (or a hand-rolled NNpsk0/XX subset) for the pairing handshake in §4.4 and for the sealed-sender envelope in §6.2.**

**quinn vs quiche vs s2n-quic.** quinn: pure Rust, rustls; `quinn-proto` is a sans-IO state machine usable without tokio, but the `quinn` crate (and iroh) use tokio. quiche: Cloudflare, sans-IO by design, BSD-2, bundles BoringSSL via `boring-sys` (a C/C++ build; ~2–3 MB of code that is file-backed and clean, but a heavy dependency and a second TLS stack next to phase 1's rustls). s2n-quic: AWS, spawns tokio tasks opaquely from endpoint creation; not usable without tokio. Interop results show quiche faster than quinn; speed is not our metric. Fit: what matters is dirty memory of an *idle* endpoint and the ability to have no endpoint at all. A tokio current-thread runtime with nothing scheduled costs a few hundred KB and zero wakeups; the multi-thread runtime's worker threads and stacks are what people measure when they say "tokio is heavy". **Recommendation: iroh on quinn with `tokio` features limited to `rt` (current thread) and `net`; F0 measures it. Fallback if F0 fails: a broker built on `quinn-proto` sans-IO with a single `poll(2)` loop, re-implementing only iroh's relay client protocol. quiche is rejected on dependency grounds (BoringSSL), not on merit.**

## 2. Survey: data and sync, capabilities, storage

**Willow protocol / Earthstar (willow-rs, now at codeberg.org/worm-blossom/willow_rs).** A data model of namespaces, subspaces (per-author keys), paths and timestamps; Meadowcap capabilities (read/write caps that are delegatable, with the ability to *prove* a capability without revealing the key); the WGPS sync protocol with private area intersection so peers do not reveal what they are syncing unless both have it. Rust implementation covers the data model, Meadowcap and a sled-backed store; WGPS and sideloading are in progress as of September 2026. Fit: this is the closest published design to what §6 needs (per-author subspaces, capability-gated sync, privacy-preserving intersection, destructive edits). Misfit: not finished; sled is memory-hungry; the encoding is intricate. **Borrow heavily (Meadowcap's read/write split, private area intersection, "prefix pruning" deletes). Do not adopt the crate until WGPS ships and a `Private_Dirty` measurement exists; revisit at F5.**

**p2panda.** Rust local-first toolkit (append-only logs with Bamboo, aquadoggo node). aquadoggo was archived in May 2026; the project has moved to smaller crates. Fit: good ideas about schema-versioned entries. **Avoid as a dependency; the ecosystem is in transition.**

**Secure Scuttlebutt.** Per-identity append-only feed, gossip replication, no deletion, one global Ed25519 identity per user, pubs as relays. Manyverse is no longer updated. Fit: offline-first done for real, a decade ago. Misfit: global identity, no deletion, gossip traffic. **Avoid; the "one feed per writer, replicate by sequence" model is already in phase 2's log channel.**

**Nostr.** Signed JSON events, one global secp256k1 pubkey, relays that see all plaintext and all metadata (DM encryption in NIP-04/59 is bolted on; NIP-59 "gift wrap" adds sealed sender). Fit: relays as dumb stores, easy to self-host. Misfit: global identity is a single-key fingerprint across every relay; relays learn the social graph. **Borrow NIP-59's sealed-sender envelope shape for §6.2; avoid the identity model.**

**AT Protocol.** Repos of public records per DID, PDS hosting, relays that crawl all PDSs, `did:plc` resolved through a directory operated by Bluesky (a Swiss association is promised). Identity leaks: `did:plc` history (every handle ever bound) is public and immutable; all records are public; private data is "upcoming"; the DID is global across every app. Fit: the "you own your repo, host it anywhere" story and content-addressed record blocks (CAR). Misfit: designed for public broadcast; identity is global and its history cannot be erased. **Avoid; cite as the example of what a global identifier costs.**

**Solid pods.** HTTP resources in a personal pod, WebID (a global HTTPS identity), ACLs via WAC/ACP. Stewardship moved from Inrupt to the Open Data Institute in late 2024; enterprise pilots continue. Fit: "data in the user's store, apps request access". Misfit: WebID is a global identity; everything is HTTP + DNS + CA; the pod sees plaintext. **Avoid; borrow the app-asks-for-scoped-access model, which we implement as capabilities.**

**Automerge / Loro / Yjs.** Automerge 3 (2025) cut memory roughly 10x (a Moby-Dick-sized text at ~1.3 MB in memory) and is Rust with C/wasm bindings. Loro (Rust, 1.0 in 2024) has smaller encodings and movable trees. Yjs is JS. Fit: phase 2's position holds: the host offers logs with causal delivery, and CRDTs are stack libraries the app links. Automerge 3 is now lean enough to be *the* recommended stack library for text. **Borrow; adopt Automerge 3 as the phase-2 W6 "text CRDT stack library"; no CRDT in the host.**

**cr-sqlite.** SQLite extension adding CRDT tables (forks maintained by vlcn-io, superfly and Fundament). Inserts ~2.5x slower than plain SQLite. Fit: relational local data with merge. Misfit: SQLite is C and, per phase 2's estimate, ~300 KB dirty per open connection; the host's storage is the log/KV pair. **Avoid in the host; a stack library may link it inside a `large` card if it wants.**

**Syncthing.** Go, block-exchange file sync, device ids are certificate hashes, global discovery servers and relays. Mature. Idle memory on a Pi is tens of MB (GOGC tuning is a documented topic); index scans and periodic rescans are idle CPU. Fit: the relay and discovery deployment model for a home node is proven. Misfit: a Go process at 50–200 MB; folder-level, not app-level. **Avoid as a component; copy its relay operator model (anyone can run one, clients pick by latency).**

**Braid-HTTP.** HTTP extensions for versions, patches and subscriptions. The IETF draft expired in May 2024; work continues at braid.org. Fit: a clean shape for "subscribe to a resource's changes". Misfit: subscriptions are long-lived connections, i.e. wakeups; HTTP means DNS+CA. **Borrow the version/patch vocabulary for the HTTPS mirror level; avoid subscriptions.**

**UCAN.** Capability tokens as JWT (1.0 moves to DAG-CBOR), DID-based principals, delegation chains, revocation by CID. Fit: user-originated, offline-delegable, public-key. Misfit: DID resolution is a dependency we do not want (our principals are raw Ed25519 keys); nested tokens bloat; JSON/CBOR parsing and DID handling are code we would pay for. **Borrow the delegation-chain semantics; avoid the format.**

**biscuit (biscuit-auth).** Ed25519-signed token blocks, each block adds Datalog facts/checks; offline attenuation by appending blocks; third-party blocks; a Rust reference crate. Fit: exactly "a capability the holder can narrow but not widen", public-key rooted, compact (a two-block token is a few hundred bytes). Misfit: the Datalog engine is more generality than we need (estimate 300–600 KB of code, clean, plus small parse/eval allocations). **Adopt, with the predicate set frozen to a small vocabulary (§4.3); measure the crate in F2 and, if its code size exceeds 1 MB or evaluation allocates more than 64 KB, replace with a hand-rolled signed-attenuation chain using the same semantics.**

**Macaroons.** HMAC-chained caveats; verification needs the root secret, so authority is rooted in a server. Tiny and fast. Fit: perfect for the home node to hand *itself* short session tokens. Misfit: authority is the server's, not the user's; third-party caveats need a discharge service. **Borrow the caveat idea; avoid as the platform token.**

**MLS (RFC 9420, OpenMLS).** Group key agreement with forward secrecy and post-compromise security via TreeKEM. OpenMLS is at 0.9 (2026) with an SRLabs audit funded by the Sovereign Tech Agency. Footprint: code ~1–2 MB (clean); group state proportional to members, stored via a storage-provider trait so it can live on disk; runtime allocations per commit are small (estimate < 256 KB transient). Fit: the correct answer for "share with a group whose membership changes". Misfit: MLS wants a Delivery Service with ordering; our relay provides it as an append-only log per group. **Adopt for `shared:<group>` channels (§6.3), loaded only during a sync window.**

**Keyhive (Ink & Switch; formerly Beehive; BeeKEM, Beelay).** Local-first access control: capabilities as a delegation graph, BeeKEM (a concurrency-tolerant TreeKEM variant) for group keys, Beelay for auth-enabled encrypted sync. Pre-alpha, unaudited, Rust. Fit: the design target is exactly ours, including "who can read which document" without a server. **Borrow the delegation-graph and revocation-as-data ideas; do not depend on it before an audit and a stable API. Revisit at F6.**

**Passkeys / WebAuthn / FIDO2 and the PRF extension.** Platform authenticators with per-relying-party keys (no global id). The PRF extension (Firefox 148+, Chrome 147+ committed on-create, Safari 18+, Windows 11 via the February 2026 update; roaming authenticators on iOS still cannot pass extensions) lets a site derive a secret from the credential; Bitwarden uses it to unlock vaults. Critics note the derived secret is bound to one credential-manager implementation and can be lost with it. Fit: the "pairwise key per origin, no global id" model is exactly what we adopt in §4.2. Misfit: WebAuthn is a browser API; we are the platform, so we implement the model natively. **Borrow the model; optionally use a passkey PRF as one *unlock factor* for the local key store, never as the root of the hierarchy.**

**BLAKE3 / bao verified streaming.** Tree hash; bao encodes the tree so any range can be verified against the root hash while streaming. The `bao-tree`/`abao` crates (n0) add chunk groups (16 KB groups cut outboard size). Not formally audited as a streaming format. **Adopt (it is what iroh-blobs uses).**

## 3. The recommended stack

| Layer | Choice | Adds (estimate) | Idle | Rejected |
|---|---|---|---|---|
| Hashing / blobs | BLAKE3 + bao-tree, via iroh-blobs; on-disk content store from phase 2 | ≤ 128 KB index (clean mmap) | 0 | IPFS/Bitswap (chatter), Hypercore (JS) |
| Transport | iroh core: QUIC (quinn fork) + rustls, relay client, hole punching; tokio current-thread | broker baseline ≤ 2.5 MB while an endpoint exists; **0 bytes when parked** (§7) | endpoint torn down when idle | libp2p (toolkit, chatter), quiche (BoringSSL dependency), s2n-quic (opaque tokio spawns), raw quinn-proto (kept as fallback) |
| Addressing | origin = Ed25519 public key; install record carries address hints (last direct addrs + relay URL); optional DNS (`_iroh` TXT) or pkarr for *home nodes only* | 0 | 0 on clients | DNS+CA (phase 2 §5.1), DID methods |
| Identity | root seed → device keys → per-origin pairwise keys (HKDF); biscuit tokens | key store ≤ 16 KB dirty when unlocked; biscuit ≤ 64 KB transient | 0 | UCAN (DID, bloat), macaroons (server-rooted), global ids |
| Device-to-device | Noise XX (`snow`) pairing; sync over iroh with per-device keys certified by the root | ≤ 64 KB transient | 0 | Tailscale as a requirement |
| Channel encryption | per-channel symmetric keys (XChaCha20-Poly1305) derived from the root; sealed-sender envelopes | ≤ 4 KB per open channel | 0 | plaintext at origin (phase 2 §5.4) |
| Groups | OpenMLS, state on disk, loaded per sync window | ≤ 512 KB transient during a commit | 0 | Keyhive (pre-alpha), static shared secret (no PCS) |
| Storage | phase 2's log + KV files, now encrypted at rest with a device key; content store unchanged | unchanged (≤ 32 KB per open channel) | 0 | cr-sqlite, sled |
| Home node | `lean-home`: one static binary = iroh endpoint + blob server + channel mailbox + group delivery service + optional HTTPS mirror | ≤ 24 MB RSS idle, ≤ 12 MB `Private_Dirty` (gate) | ≤ 1 wakeup/min, ≤ 2 KB/min with a relay connection; 0/0 with a direct address | Kubo, Syncthing, a "gateway" that terminates app requests |

Dependency budget: the broker may add at most **12 new top-level crates** over phase 2 (iroh, iroh-blobs, bao-tree, blake3, snow, biscuit-auth, openmls + its two provider crates, chacha20poly1305, hkdf, ed25519-dalek). Anything beyond needs a `DECISIONS.md` entry.

## 4. Identity design

### 4.1 Key hierarchy

```
root seed S (32 bytes, generated on first run; never leaves the device set except as recovery shares)
 ├─ root signing key   R  = Ed25519(HKDF(S, "lean/root"))         used only to certify devices and revocations; kept sealed (§4.5)
 ├─ device key         D_i = Ed25519(HKDF(S_i, "lean/device"))     S_i is a fresh per-device seed; D_i is certified by R (or by delegation, §4.4)
 ├─ pairwise seed      P  = HKDF(S, "lean/pairwise")                synced E2E to every device (so all devices present the same identity to an origin)
 │    └─ per-origin key  O_k = Ed25519(HKDF(P, "lean/origin" || origin_pubkey_k))
 ├─ channel root       C  = HKDF(S, "lean/channels")
 │    └─ channel key     K_{app,ch} = XChaCha key HKDF(C, app_id || channel || epoch)
 ├─ storage key        L_i = HKDF(S_i, "lean/local")                per device, encrypts the state dir at rest
 └─ app device id      A_{app,origin} = HKDF(P, "lean/devid" || app_id || origin_pubkey)  (phase 2 §6's random id, now derived)
```

The root seed exists in two places only: split by Shamir across the user's devices (§4.5) and in the offline recovery key. No device holds S in clear after enrolment; each device holds its own S_i, the pairwise seed P, the channel root C, and its share of S. Signing with R is a ceremony (recovery or root rotation), not a routine.

### 4.2 Per-origin pairwise keys

Every origin sees exactly one public key for this user, `O_k`, and cannot compute or recognise `O_j` for another origin. Two origins that collude learn nothing linking their users unless the user shared data between them. This is the passkey model implemented natively. Because `O_k` is derived from `P`, a newly enrolled device presents the same `O_k` without the origin being told a new device exists.

The iroh endpoint that connects to origin `k` uses `O_k` as its node secret key, so the transport identity *is* the pairwise identity: no separate login step, no session cookie. The origin's server code authenticates the QUIC peer by its node id and looks up the account keyed by it. Anonymous fetches (§6.4) use a fresh random node key per session with 0-RTT disabled.

### 4.3 Capability tokens

Format: biscuit, restricted to a fixed vocabulary of facts so tokens are small and evaluation is bounded:

```
authority block (signed by the user's O_k, or by D_i for device-scoped tokens):
  user(<O_k>); right(<app_id>, <channel>, read|write|admin); expires(<unix>); device(<D_i>)
attenuation blocks (appended offline, each signed by the previous holder's key):
  check if right(app, ch, read)        -- narrow to read-only
  check if time($t), $t < <earlier>    -- shorten expiry
  check if channel($c), $c == "inbox"  -- narrow to one channel
```

**Login** (first contact with an origin): none, in the classic sense. The client dials by the origin's key using `O_k`; the origin creates an account for `O_k` if the app's manifest marks `signup=open`, or asks for an invite biscuit (issued by the origin owner, signed by the origin key, carrying `invite(<code>)`) which the user pasted or scanned. No email, no password, no phone number. An origin that wants a display name gets it from the app, as data, later.

**Delegation to a second device**: the new device `D_2` is paired (§4.4) and receives `P` and `C`; it therefore already has `O_k` for every origin. What it needs is *authority*: a device certificate `cert(D_2)` signed by `D_1` under `D_1`'s own certificate, which carries `right(enrol)`. Origins do not see the certificate at all, since they only ever see `O_k`. Device certificates matter only for the user's own device set (sync, revocation) and for groups (§6.3), where each device is an MLS leaf.

**Delegation to another person** (sharing a channel): the sharer issues a biscuit with `right(app, ch, read)` and `expires`, attenuated as they like, plus the channel key wrapped to the recipient's public key for this group (HPKE). Both go in the invitation envelope. The origin or relay serving the channel verifies the biscuit chain (it holds only ciphertext but enforces who may fetch it).

**Revocation**: revocation is data. A signed `revoke(token_id | device_key)` entry is appended to the user's `identity` channel; every device replicates it and every origin the user contacts receives it as the first message of the next connection. Token lifetimes are short (default 24 h, renewed silently when the user is active, never renewed while idle) so an unreachable origin forgets a stolen token within a day. Device revocation also triggers an MLS commit removing that leaf from every group (§6.3) and rotates `P`'s *epoch* (a new `P'` and hence new `O_k'` for every origin; each origin receives `rekey(O_k → O_k')` signed by both, so a stolen device cannot impersonate the user to any origin the user contacts after revocation).

### 4.4 Pairing

A new device shows a QR/short string containing its node id and a 6-word pairing secret. An existing device scans it, dials by key over iroh, runs Noise XX with the pairing secret as a PSK (`NNpsk0` on top of the QUIC channel, so the relay learns nothing), and transfers `P`, `C`, a Shamir share, and the device certificate. Both devices show a 4-emoji fingerprint to confirm. Total traffic: under 8 KB.

### 4.5 Recovery

Recovery is treated as the platform's worst failure mode, so there are two independent paths and both are tested in CI with scripted devices:

- **Device quorum**: `S` is split with a k-of-n threshold (default 2-of-3 across devices; with only one device, 1-of-2 across the device and the recovery key). Any k devices can reconstruct `S` to certify a new device when the others are lost. Shares are re-issued on every enrolment or revocation.
- **Offline recovery key**: 24 BIP39-style words (or a printed QR) encoding `S` encrypted under a user-chosen passphrase with Argon2id. Generated at first run; the shell nags until the user confirms they stored it.

Optional: a passkey PRF output as an *additional* unlock factor for `L_i` on platforms that support it; never a source of `S`.

Losing every device and the recovery key loses the identity. Origins that opted into `recovery-contact` (a capability an origin can request, never granted by default) can hold a fourth share; the user is told plainly that this makes that origin a party to recovery.

### 4.6 What the app sees vs what the host holds

| The app sees | The host holds |
|---|---|
| `principal`: the user's public key *at this app's origin* (`O_k`), as bytes | all seeds and private keys |
| its own `A_{app,origin}` device id | device certificates and shares |
| capability results (allowed / denied) | biscuit tokens and their private keys |
| `sync.status` and `sync.request` as in phase 2 | the network, the endpoint, relays, addresses |
| channel data as plaintext bytes | channel keys; encryption and decryption happen in the broker |
| group membership as a list of *opaque member ids* | MLS state |

New WIT interface added to `lean:app`:

```wit
interface identity {
  principal: func() -> list<u8>;                      // O_k for this app's origin; 32 bytes
  invite: func(channel: string, rights: list<string>, expires: u64) -> list<u8>;  // an invitation envelope the user can hand to someone
  accept: func(envelope: list<u8>) -> result<string, string>;                    // joins a shared channel; returns its name
  members: func(channel: string) -> list<list<u8>>;   // opaque per-group member ids
}
```

There is no `sign`, no `derive`, and no `token` function. An app that needs to prove something to a third party asks the host to `invite`, which yields a token the host built.

### 4.7 Threat model

- **Lost device**: the thief gets `S_i` (if the device was unlocked), `P`, `C`, one share and a device certificate. They can impersonate the user to origins *until* the user revokes from another device or from the recovery key, which rotates `P` and removes the MLS leaf. Local data on the device is encrypted under `L_i`, which is unlocked by the OS keystore or a passphrase. Without unlocking, the thief has ciphertext and a share below threshold.
- **Malicious app**: sees `O_k` and its own channels. It cannot obtain a token, cannot reach another origin (no `fetch` without a grant; grants are visible), cannot learn other origins' keys, and cannot correlate the user across apps (different `A` and different `O_k`). It can leak the user's data at its own origin, which is the data the user gave it.
- **Curious server (origin)**: sees `O_k`, connection times, the ciphertext and sizes of the channels it stores, and the plaintext of channels the user granted it `origin-read` for. It does not see other origins, other apps, device count, or content.
- **Malicious relay**: sees node ids of the two parties (the user's `O_k` for that origin and the origin key), packet timing and sizes. QUIC end-to-end encryption hides everything else; channel entries are additionally encrypted, so even a relay that is also the origin gains nothing. A relay can deny service; the client tries direct paths and other relays.
- **Network observer**: sees UDP flows to an origin's address or a relay's. Client node ids are per-origin, so observing two flows to two origins does not link them. 0-RTT is enabled only after the first authenticated connection to that origin (the ticket then links connections *to that origin*, which the origin already links by `O_k`). Anonymous blob connections never use 0-RTT and use a fresh key.

## 5. Self-hosting end to end

### 5.1 What a person runs

`lean-home`: one static musl binary, no database, no TLS certificate, no domain. First run prints its public key (the origin address) as a QR and a string, writes `~/.lean-home/{key, store/, channels/, groups/, config.kdl}`, and starts serving. It contains: an iroh endpoint; the blob server (iroh-blobs) for app bundles the owner publishes and for any content-addressed blob it has been asked to mirror; the channel mailbox (append-only encrypted entries per `(O_k, app, channel)`, with quota); the MLS delivery service (ordered log per group); optional HTTPS mirror and reverse bridge (phase 2 §5.6) for people still on the legacy web.

Budgets (CI-gated on an aarch64 Pi 4 and an x86-64 laptop-class VM):

| Metric | Gate |
|---|---|
| `Private_Dirty` idle, 0 connections | ≤ 12 MB |
| RSS idle | ≤ 24 MB |
| Wakeups idle, direct-address mode | ≤ 1/min |
| Wakeups idle, relay mode | ≤ 15/min (relay keepalive; the price of NAT) |
| Bytes idle, relay mode | ≤ 2 KB/min |
| Disk | store + channels; owner-set quota; default 2 GB |
| Per connected client | ≤ 256 KB |
| Startup to serving | ≤ 2 s on the Pi |

### 5.2 How clients reach it

The origin address is its key. The client's install record carries **address hints**: the last known direct addresses and relay URL, refreshed on every successful connection. First contact needs one of: hints embedded in the invite (QR contains key + hints), a DNS TXT record the owner optionally publishes (`_iroh.<name>` as iroh does), or a pkarr record the home node optionally republishes to the mainline DHT (owner opt-in; it makes "this key is at this relay" public).

Connection order, per attempt: (1) direct to hinted addresses (works on a LAN, over IPv6, with a port-forward, or over the owner's Tailscale/Headscale); (2) hole punch via the origin's relay; (3) relayed through it. The relay is any iroh relay: n0's public ones, one the owner runs (`iroh-relay` is a separate small binary; estimate 20–40 MB RSS, unverified), or a community one. `lean-home` prints which of the three paths clients are using so owners can fix their router.

### 5.3 Updates, backups, migration

- Updates to `lean-home` itself: the binary checks nothing. The owner's package manager or a manual download updates it; the format of `~/.lean-home` is versioned and migrations are forward-only with a backup copy.
- Backups: `~/.lean-home` is plain files; a copy is a backup. Channels are ciphertext, so a backup on an untrusted disk leaks only sizes. The key file is the only secret; losing it means a new origin address (clients keep hints for the old one and show "origin key changed" unless the old key signed a `successor(new_key)` record, which the tool offers to produce).
- Moving house: copy the directory; the address does not change because the address is the key.
- Publishing an app: `lean-home publish ./app` hashes the bundle, signs a release record with the *publisher* key (distinct from the origin key; both live on the home node by default), and serves it. Any other node that mirrors the blob can serve it; the release record still says who published.

## 6. Data model changes relative to phase 2

### 6.1 Stateful delivery

An **install** is a signed record `{app_id, publisher_key, version, manifest_hash, bundle_hashes, pinned: bool, installed_at, previous: Option<install_hash>}` kept in the user's `installs` channel (encrypted, synced across devices). The host never fetches a new manifest on its own. Update discovery happens when the user opens the app's card or the "Updates" panel; it is one anonymous blob fetch of the publisher's latest release record (a tiny signed blob addressed by `blake3(publisher_key || app_id || "latest")`, served like any blob so mirrors work). Applying an update is a user action that writes a new install record; the old bundle stays in the store until the user clears rollbacks. **Rollback** rewrites the install record to `previous`; state blobs are versioned by manifest hash so a rolled-back app finds its own state (a migration-aware app can opt to read newer state).

Pinning is default-on for the pinned version; "auto-apply updates" is a per-app toggle the user turns on, and even then the host applies updates only at the next open, with a visible "updated to 1.4 — roll back" line for a day.

### 6.2 Encrypted channels

`user`-scope channels: each entry is `{seq, prev_hash, nonce, ciphertext}` where ciphertext = XChaCha20-Poly1305(`K_{app,ch,epoch}`, entry) with AAD `(app_id, channel, device_id, seq)`. Version vectors, siblings and causal ordering from phase 2 apply to the plaintext after the broker decrypts. The origin (or any mailbox node) stores entries by `(O_k, app, channel, seq)` and serves them to whichever of the user's devices asks with a valid biscuit (`right(app, ch, read)` rooted in `O_k`). It cannot read them. **Origin-readable** channels (an app whose server-side needs the data, e.g. a mail server) are declared in the manifest as `channels { "outbox" mode="log" scope="user" origin-read=true }`, shown at install time, and encrypted under a key wrapped to the origin's key as well.

Sealed sender: entries pushed to someone else's origin (an invitation, a message to a group hosted elsewhere) are wrapped Nostr-NIP-59-style: the outer envelope is from a fresh random key, the inner is signed by `O_k` of the recipient's origin as seen by the sender. The hosting node learns "someone sent this group a message", not who.

### 6.3 Groups

`shared:<group>` channels use MLS. Each of a member's *devices* is a leaf (so device revocation is an MLS remove). The group's delivery service is the origin of whoever created the group, or any `lean-home` the creator names; ordering is the node's append order. The channel key for the group is the MLS exporter secret for the current epoch; entries carry the epoch. Membership changes are commits in the group log; the app sees `members()` as opaque ids.

Why MLS and not a simpler wrapped-key scheme: removing a member must actually stop them reading future entries (post-compromise security), and a household of six devices changes membership more often than a chat group does. OpenMLS state lives on disk through its storage provider and is loaded only inside a sync window; the broker holds no group in memory while parked.

### 6.4 Anonymous blob fetch and the `fetch` fix

Blobs (app bundles, assets, release records, stack modules, public snapshots) are fetched from *any* provider: the publisher's node, a mirror, an HTTPS mirror (`GET /blob/<hash>`, bao-encoded), with a per-session random node key, no biscuit, no 0-RTT, no resumption. Provider order: mirror list from the install record; the publisher; n0-style content discovery is *not* used (it is a DHT). Mirrors learn "some IP fetched hash H", which is the minimum possible.

`fetch.send` (phase 2 §4.2) is redefined: with the manifest capability `fetch origins="api.example.com"` the broker connects **directly** to that host over HTTPS from an unauthenticated connection (no cookies, no client certificate, phase 1's broker rules), batched inside sync windows. Proxying through the app's origin becomes a separate capability `fetch via-origin=true`, which the install screen explains as "the app's server will see these requests". Both are counted in the network-activity panel.

### 6.5 WIT and manifest deltas

- `identity` interface (§4.6) added to the world.
- Manifest `channels` gain `origin-read`; `capabilities` gain `background-sync interval="1h"`, `fetch via-origin`, `recovery-contact`, `push`.
- `sync.request(channel, wish)` semantics: `now` and `soon` are honoured only while the card is foreground; `idle` means "at the next window the user granted", and if none was granted, "the next time the user opens me".

## 7. Idle: definition, metrics, CI

**Definition.** A process is idle when no user input has arrived for 10 s and no granted background capability is due. Idle is measured over a 10-minute window after a 30-second settle.

**Metrics** (added to `lean-measure`, next to `Private_Dirty`):

- *Wakeups per hour*: from `/proc/<pid>/sched`'s `nr_switches` (delta over the window, all threads summed via `/proc/<pid>/task/*/sched`), cross-checked in CI with `perf stat -e sched:sched_switch -p <pid>` where `perf` is available. A wakeup is any transition to running.
- *Bytes per hour*: each measured process runs in its own network namespace with a veth pair; the harness reads `rx_bytes`/`tx_bytes` from `/sys/class/net/<veth>/statistics/` before and after. This counts everything including keepalives and DNS.
- *CPU*: `utime+stime` from `/proc/<pid>/stat`, reported, not gated (it follows from wakeups).

**Gates:**

| Process / state | Wakeups/hour | Bytes/hour | `Private_Dirty` |
|---|---|---|---|
| Frozen card (per card) | 0 | 0 | ≤ 4 KB |
| Running card, background, no grant | 0 (the host never calls into it) | 0 | its class cap |
| Running card, foreground, no input | 0 attributable to the card; the host may repaint on window events only | 0 | its class cap |
| App host, 0 running cards | ≤ 6 (allowance for the OS memory-pressure signal and Landlock bookkeeping) | 0 | ≤ 1.5 MB |
| Sync broker, parked (no endpoint) | 0 | 0 | ≤ 512 KB |
| Sync broker, endpoint held (only while a foreground card has `live` and the user interacted within 5 min) | ≤ 720 (one keepalive per 5 s) | ≤ 60 KB | ≤ 2.5 MB |
| Renderer, no input | 0 | 0 | phase-1 gates |
| `lean-home` | §5.1 | §5.1 | §5.1 |

**Mechanisms that make these achievable:** apps have no clock callbacks (phase 2 already gives them only a coarsened monotonic clock and no timers); the host's only periodic timer is the wake-window scheduler, which is *armed only if at least one app holds a granted `background-sync`*, and it arms a single `timerfd` for the earliest due window, never one per app; the broker closes its endpoint 30 s after the last stream and drops tokio's runtime entirely (the runtime is created per window); winit's event loop uses `ControlFlow::Wait`; the renderer never animates while idle (phase 2's 150 ms cross-fades run to completion and stop); there are no file watchers on the state dir (changes come through the broker's IPC, not inotify).

**Visibility.** The shell's network-activity panel lists, per app: last sync, bytes this week, granted background interval, `fetch` hosts contacted, and a "revoke" button per capability. This is the user-facing side of "phoning home is a capability".

## 8. The burl test

The reference app is `burl-lean`, a port of the author's burl to the Rust stack: the same screens and the same data source, as a `small`-class card. It is checked into `corpus/apps/burl-lean` alongside a `burl-native` row in the CI report that records the original program's idle CPU (~1% of a core) and RSS (~80 MB) from a one-time measurement, so the comparison is on the same page as the gates.

What the platform guarantees for it, and how each guarantee is enforced:

| burl's idle cost | Cause in burl | Why it cannot exist here | Gate |
|---|---|---|---|
| ~1% CPU | raylib's continuous render loop | apps have no render loop; the renderer paints on events only (`ControlFlow::Wait`); a running card is called only on `update`/`render` after an event | 0 wakeups/hour attributable to the card |
| GL context + driver memory | GPU rendering | no GPU path; CPU strip buffer ≤ 320 KB; window buffer is `Shared_Dirty` | phase-1 renderer gate |
| font atlases | per-app text rendering | fonts mmap'd once by the renderer; glyph cache capped at 256 KB for the whole device | phase-1 gate |
| GC heap growth | Boehm GC | TEA model in wasm; `small` linear-memory cap 4 MB; frozen after 60 s of background: instance dropped, 0 dirty | frozen card ≤ 4 KB |
| polling its data source | app-owned timer | no timers; `sync.request(idle)` only fires in a user-granted window; default is on-open | 0 bytes/hour without grant |

Exit criteria for the test: with `burl-lean` open and unfocused for 10 minutes, the whole system (renderer + app host + broker) shows ≤ 6 wakeups, 0 bytes, and the card's marginal `Private_Dirty` ≤ 4 KB when frozen and ≤ 4 MB + 96 KB when running. Thaw-and-refresh on focus completes in ≤ 400 ms including a sync window on a LAN. The test also runs with a granted `background-sync interval="1h"` and must show exactly one endpoint lifecycle per hour with ≤ 8 KB transferred for an unchanged source.

## 9. Milestones

These slot in after or alongside phase 2's W-milestones. Effort assumes one engineer; numbers are targets.

**F0 – Go/no-go: iroh idle and memory (2 weeks; alongside W1).** The riskiest assumption is that iroh on quinn/tokio can be small enough and can be *fully parked*. Build a broker skeleton with an iroh endpoint and measure, with the §7 harness: (a) parked (no endpoint, runtime dropped): wakeups, bytes, `Private_Dirty`; (b) endpoint with a relay connection, idle; (c) endpoint with one direct connection, idle; (d) create-endpoint → dial-by-key → 1 KB round trip → teardown, latency and peak dirty. **Go**: (a) is 0/0/≤ 512 KB, (b) ≤ 2.5 MB dirty and ≤ 720 wakeups/h and ≤ 60 KB/h, (d) ≤ 400 ms on a LAN and ≤ 1.5 s via relay. **No-go**: adopt the fallback: `quinn-proto` sans-IO on a single `poll` loop, plus a re-implementation of iroh's relay client protocol (estimate 3–4 weeks extra) and no hole punching until F4b. Also measured in F0: tokio current-thread vs multi-thread baseline, to document the number.

**F1 – Idle harness and gates (2 weeks; alongside W2).** `lean-measure` gains wakeups and bytes per §7, per-process network namespaces, and the gate table. The phase-2 W2 exit ("100 frozen cards ≤ 8 KB each") gains "0 wakeups, 0 bytes over 10 minutes with 100 frozen cards". Exit: every existing phase-1 and phase-2 corpus run reports the three idle metrics; renderer and app host pass their rows.

**F2 – Identity core (4 weeks; independent of the network).** Key hierarchy, sealed key store under `L_i`, device certificates, Shamir shares, recovery-key generation and restore, biscuit tokens with the frozen vocabulary, revocation records, `identity` WIT interface, scripted-device tests for enrol/revoke/recover. Exit: property tests that no origin-visible value is equal across two origins for one user; recovery from (i) k devices and (ii) the recovery key alone, both scripted in CI; biscuit crate code size and eval allocation measured against the §2 threshold.

**F3 – Stateful delivery (3 weeks; alongside W3/W5).** Install records, pinning, rollback, versioned state blobs, release records as blobs, the Updates panel, HTTPS blob mirror endpoint. Exit: install → update → roll back → update again on the mail example with state preserved at each step; the host makes zero network requests over 24 h with auto-apply off (harness-verified); a bit-flipped bundle is rejected by bao verification before any byte reaches the store.

**F4 – Transport and the home node (5 weeks; replaces the QUIC half of W3).** Broker on iroh with per-origin endpoint keys, parked/held lifecycle, address hints, `lean-home` v0 (blob server, channel mailbox, key printout, optional DNS/pkarr publishing), three-path connection with path reporting. Exit: two clients behind different NATs (network namespaces with NAT rules in CI, plus one real-world run across two ISPs) reach a `lean-home` by key on path (1), (2) and (3) in forced scenarios; `lean-home` passes its §5.1 gates on a Pi 4; the broker passes §7 in both states.

**F5 – Encrypted channels and anonymous blobs (4 weeks; alongside W6).** Channel keys and epochs, at-rest encryption of the state dir, sealed-sender envelopes, `origin-read` declaration, direct `fetch` with the via-origin capability split, 0-RTT policy, network-activity panel. Exit: the W3 two-device convergence test passes with the mailbox holding only ciphertext (verified by grepping the mailbox for a known plaintext marker); a blob fetch from a mirror carries no identity (packet capture shows a random node id and no session ticket); the panel's byte counts match the harness within 5%.

**F6 – Groups (4 weeks; after W6).** OpenMLS integration with on-disk storage provider, device-as-leaf, delivery service in `lean-home`, invitation flow, member removal on device revocation. Exit: a six-device, three-person group; removing a device makes its subsequent entries undecryptable and the group log stays under 2x payload; broker parked between windows with OpenMLS state entirely on disk (0 dirty attributable to groups while parked).

**F7 – The burl test and the i18n spike (3 weeks).** `burl-lean` ported and gated per §8. In parallel, the internationalisation experiment (§10): a bidi paragraph and a CJK IME field through the renderer to size the debt. Exit: §8 exit criteria; a written estimate for bidi and IME with a decision to schedule or to document as a limitation.

**F8 – Hardening (ongoing).** Fuzz targets for envelope parsing, install records, biscuit inputs, MLS message handling; a second recovery drill per release; an external review of §4 before any public release.

## 10. Risks and open questions, with experiments

- **R1: iroh's idle profile.** Covered by F0's go/no-go. Secondary risk: iroh issue #4549 (memory growth with path churn) affects `lean-home`; experiment in F4: 72-hour soak with clients cycling through NATs, RSS gate enforced by an internal watchdog that restarts the endpoint (not the process) above 32 MB.
- **R2: Scope for one person.** This layer is roughly 27 engineer-weeks on top of phase 2's ~29. Mitigation is ordering: F0–F3 deliver value without any of F4–F6 (idle gates, identity, pinned installs work over phase 2's HTTPS fallback); F4 is the first network milestone and the only one with a hard dependency on iroh. If time runs out, stop after F5: single-user self-hosting is complete; groups are the deferrable half.
- **R3: The wasm runtime undermines leanness.** Largely defused by freezing (a frozen card is a file), but verify: experiment in F1 measures Wasmi 1.0's dirty memory for a compiled `small` module (phase 2's open question) and per-instance overhead with the idle harness. Threshold: if a *running* idle card costs more than 96 KB host-side beyond its linear memory, or if compiled modules are not file-backable, evaluate Wasmi's module serialisation or AOT to file-backed code before F7.
- **R4: Bidi/RTL and IME debt.** Phase 1 renders RTL as LTR and phase 2 has no IME path; a platform meant for people cannot ship that way indefinitely. F7's spike sizes it: unicode-bidi (UAX #9) plus swash's shaping is likely 2–3 weeks; IME via winit's `Ime` events into the input-state table is likely 1–2 weeks plus platform testing. Decision rule: if under 6 weeks combined, schedule as F9; otherwise document as a limitation with a tracking issue. Memory impact must be measured (bidi tables are static, clean).
- **R5: biscuit is more than we need.** Measured in F2 against the code-size and allocation thresholds in §2; the fallback is a signed attenuation chain with identical semantics and a hand-written verifier (~600 lines).
- **R6: MLS delivery ordering across mirrors.** MLS assumes one ordered log per group; if a group's home node is unreachable, members cannot commit. Accepted for v1: reads still work from any mirror of the ciphertext; commits wait. Experiment in F6: measure how long a six-device group tolerates its DS being offline (expected: indefinitely for reads, no data loss).
- **R7: Discovery without DNS is a UX problem.** Keys are not memorable. Mitigation: invites carry hints; the home node's optional DNS TXT gives people who own a domain a name; the shell keeps a local address book of origins with user-chosen labels. Open: whether to support petnames shared inside groups. Experiment: five people set up `lean-home` from the README with no help; count who reaches it from a phone on mobile data.
- **R8: Recovery-key hygiene.** People lose paper. Experiment in F2's usability pass: measure how many test users can restore from the 24 words after two weeks. If under 80%, add the optional `recovery-contact` origin share by default *with* a clear explanation, or a printed QR sheet.
- **R9: Relay dependence.** Hole punching fails for symmetric NATs; relays then carry all traffic and become a cost centre. Mitigation: `lean-home` prints a one-line "your router blocks direct connections; forward UDP port N" hint; relays are pluggable and the owner can run their own. Measured in F4's real-world run.
- **R10: Updates that are never applied.** Pinned-by-default means security fixes wait for the user. Mitigation: publishers can mark a release `security=true`; the shell shows a persistent, non-blocking banner and offers one-tap apply with rollback; it still never applies silently. Open: whether a user-set "auto-apply security releases" toggle is acceptable under the principle. Proposed: yes, off by default, per app.
- **Open: pkarr/DHT publishing for home nodes.** Convenient but public. Proposed default: off; on only when the owner has no DNS name and accepts the notice.
- **Open: the HTTPS reverse bridge and legacy browsers.** Kept as optional in `lean-home`; it needs a domain and a certificate, which is exactly the legacy stack. Proposed: ship it, mark it "legacy web only", and never require it for any Lean-to-Lean feature.

## 11. Critical files (to be created under `lean-browser/crates/`, after the current build)

- `identity/src/keys.rs` — the hierarchy in §4.1, derivation labels, sealed store, Shamir shares.
- `identity/src/caps.rs` — biscuit vocabulary, issuance, attenuation, verification, revocation records.
- `sync-broker/src/endpoint.rs` — parked/held lifecycle, per-origin endpoint keys, 0-RTT policy, address hints.
- `sync-broker/src/channel_crypto.rs` — channel keys, epochs, sealed-sender envelopes.
- `sync-broker/src/groups.rs` — OpenMLS integration with the on-disk storage provider.
- `lean-home/src/main.rs` — the home node: blob server, mailbox, delivery service, key printout, budgets.
- `harness/src/idle.rs` — wakeups and bytes measurement, network namespaces, the §7 gate table.
- `corpus/apps/burl-lean/` — the reference app for §8.
- `plans/DECISIONS.md` — gains the `identity` justification tag and the supersession table from §0.1.
