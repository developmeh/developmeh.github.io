// The browser's half of the mailbox-assisted punch (KRAP-1 §5 rung 3, §7.6):
// take a verified Dial Record, fabricate the node's session description out of
// `dtls` and `ice` alone, drop a postcard carrying this browser's candidates,
// and wait for the node's connectivity checks to arrive from an address nobody
// ever signalled.
//
// This mirrors `krappy-node/dial.go` and `krappy-node/internal/punch`, which
// are the same protocol in Go and have been run end to end on loopback. Where
// this file deliberately differs from Go the comment says so, and there are
// four such places. All four are browser facts, not preferences:
//
//   1. **STUN is not optional here, and that is measured rather than
//      asserted.** The Go client walks local interfaces; a browser cannot, and
//      both engines hand out host candidates as `<uuid>.local` mDNS names that
//      a node cannot parse, let alone route to. `make web-browser-dial` runs a
//      control for exactly this: with obfuscation left at the browser default
//      and no STUN server, Chromium 149 gathered one candidate and Firefox 146
//      gathered two, and `postcardCandidates` kept ZERO of them in both. Add a
//      STUN server and both dial. That third party is in the dial path (HANDOFF
//      settled decision 4) and this module keeps it in the API: `stunServers`
//      is a constructor argument, `thirdParties()` prints it back, and nothing
//      here pretends the dial is two-party.
//
//   2. **The `mid` is read from our own offer, not hard-coded.** Go's
//      `AnswerFromRecord` writes `a=mid:0` and `a=group:BUNDLE 0` because pion
//      offers `0`, and an answer whose mid does not match the offer's is
//      rejected outright. Chromium 149 and Firefox 146 both offered `0` when
//      this was first run in a browser — the `sdparta_0` this comment used to
//      predict for Firefox did not appear, so the hard-coded default would in
//      fact have worked. It is still read rather than assumed: Gecko has
//      spelled it `sdparta_0` historically, that is a per-build fact about a
//      string, and `answerFromRecord` costs nothing to parameterise.
//      Defaulting to `'0'` keeps the output byte-identical to Go's, which is
//      what the cross-check compares.
//
//   3. **The fabricated answer advertises `a=ice-options:trickle`.** Go's does
//      not, and pion accepts both (asserted in
//      `krappy-node/internal/punch/browser_sdp_test.go`). The answer carries
//      **no candidates and no `a=end-of-candidates`** — that absence is the
//      whole trick, because a NATed node has no address to publish. But per
//      RFC 8839 an agent that does not advertise trickle is declaring its
//      candidate list complete, and a complete list of zero candidates is a
//      description a conforming implementation may fail immediately rather
//      than wait on. Chromium 149 and Firefox 146 both accept the answer WITH
//      it (2026-08-16) and both then learn the node peer-reflexively — but
//      neither has been asked to accept the answer WITHOUT it, so whether the
//      line is load-bearing or merely correct is still open. See the header of
//      `web/test-punch.mjs` for the other guesses in this class.
//
//   4. **The DTLS fingerprint and the ICE credentials are read back out of the
//      SDP.** Go generates its own ufrag/pwd and takes the fingerprint off the
//      certificate object. A browser exposes neither: `RTCPeerConnection` has
//      no API for its own ufrag, pwd or certificate digest. The only place
//      those three facts exist is `pc.localDescription.sdp`, so `localFromSdp`
//      parses them — which is also what makes the postcard-building half of
//      this module a pure function, testable with no browser at all.
//
// ## What is pure here, and why that matters
//
// Node 22 has no `RTCPeerConnection`, and a shim is not a browser: wiring one
// in would produce a green test that proves nothing, which is a mistake this
// repo has already made once. So everything decidable without a peer
// connection is a pure function over strings and bytes —
//
//     fingerprintLine   parseFingerprint   answerFromRecord
//     parseSessionDescription   localFromSdp   postcardCandidates
//
// — and those are cross-checked against pion in both directions by
// `make web-punch`. Everything below the "browser only" banner needs a real
// browser, and gets one from `make web-browser-dial`
// (`web/browser-check/dial.mjs`): a real Chromium and a real Firefox resolving
// a record, dropping a sealed postcard on a real `krappy-bb`, and being
// punched by a real `krappy-node watch`. Neither target subsumes the other —
// `web-punch` is in `make check` and needs no browser; `web-browser-dial` is
// not, and needs two binaries, a browser and a free port.

/* eslint-disable no-bitwise */

/**
 * The default STUN server, stated loudly rather than buried in a config
 * object. A browser cannot learn its own reflexive address without one, so
 * this is a third party on the happy path of every browser dial — the same
 * kind of dependency as the board (HANDOFF settled decision 4), and it should
 * be swapped for one the operator runs wherever that is possible.
 *
 * It is a list because `RTCConfiguration.iceServers` is, and because racing
 * two independent operators is the cheap way to stop one outage from being a
 * dial outage.
 */
export const DEFAULT_STUN_SERVERS = Object.freeze([
  'stun:stun.l.google.com:19302',
  'stun:stun.cloudflare.com:3478',
]);

/**
 * How many candidates a postcard carries.
 *
 * Matched to `maxCandidates` in `krappy-node/internal/punch/sdp.go`, which is
 * an amplification bound and not a tuning knob: every candidate is another
 * destination the node sends connectivity checks to. Sending more than the
 * node will read just wastes postcard bytes, and the postcard is ~150 bytes
 * on purpose.
 */
export const MAX_POSTCARD_CANDIDATES = 8;

/** The default data channel label, matching `defaultChannelLabel` in Go. */
export const DEFAULT_CHANNEL_LABEL = 'krappy';

/**
 * Something about the punch went wrong.
 *
 * `kind` separates the failures a caller can act on differently:
 *
 *   - `record`    — the Dial Record cannot be punched toward: no `dtls`, no
 *                   `ice`. Refetch, or the node published something broken.
 *   - `sdp`       — a session description could not be built or parsed.
 *   - `gather`    — ICE gathering produced nothing a postcard can carry. On a
 *                   browser this almost always means STUN failed.
 *   - `platform`  — no `RTCPeerConnection`. Node, or a non-secure context.
 *   - `timeout`   — nothing arrived in time.
 *   - `failed`    — the peer connection reached `failed`/`closed`.
 */
export class PunchError extends Error {
  constructor(kind, message, { cause, ...rest } = {}) {
    super(message, cause === undefined ? undefined : { cause });
    this.name = 'PunchError';
    this.kind = kind;
    Object.assign(this, rest);
  }
}

// ===========================================================================
// Pure: fingerprints
// ===========================================================================

/**
 * Render a raw 32-byte SHA-256 digest as SDP's colon-separated uppercase hex.
 *
 * A record's `dtls` and a postcard's `dtls` are raw bytes; every SDP parser
 * wants this shape. Byte-for-byte the same as Go's `fingerprintLine`.
 *
 * @param {Uint8Array} digest
 * @returns {string}
 */
export function fingerprintLine(digest) {
  if (!(digest instanceof Uint8Array)) {
    throw new PunchError('sdp', `punch: DTLS fingerprint is ${typeof digest}, want 32 bytes`);
  }
  if (digest.length !== 32) {
    throw new PunchError('sdp', `punch: DTLS fingerprint is ${digest.length} bytes, want 32`);
  }
  let out = '';
  for (let i = 0; i < digest.length; i++) {
    if (i > 0) out += ':';
    out += digest[i].toString(16).padStart(2, '0').toUpperCase();
  }
  return out;
}

/**
 * The inverse: colon-hex back to the 32 raw bytes a postcard's `dtls` carries.
 *
 * Case-insensitive, because RFC 8122 says the hex is case-insensitive and
 * implementations disagree in practice — pion emits uppercase, Firefox
 * uppercase, Chrome uppercase, but nothing forbids the other.
 *
 * @param {string} line the value after `a=fingerprint:sha-256 `
 * @returns {Uint8Array}
 */
export function parseFingerprint(line) {
  const parts = String(line).trim().split(':');
  if (parts.length !== 32) {
    throw new PunchError('sdp', `punch: fingerprint has ${parts.length} octets, want 32`);
  }
  const out = new Uint8Array(32);
  for (let i = 0; i < 32; i++) {
    if (!/^[0-9a-fA-F]{2}$/.test(parts[i])) {
      throw new PunchError('sdp', `punch: fingerprint octet ${i} is ${JSON.stringify(parts[i])}, not two hex digits`);
    }
    out[i] = parseInt(parts[i], 16);
  }
  return out;
}

// ===========================================================================
// Pure: fabricating the node's session description
// ===========================================================================

/** The single bundled media section. One m-line, data channels only. */
const DEFAULT_MID = '0';

/**
 * The node's answer, rebuilt from the Dial Record's `dtls` and `ice` alone —
 * the JS half of the munge, and the exact counterpart of Go's
 * `punch.AnswerFromRecord`.
 *
 * There are no candidates in it and no `a=end-of-candidates`, and that is the
 * entire point: a NATed node has no address to publish. Its connectivity
 * checks arrive from an address nobody predicted and our ICE agent learns it
 * as a **peer-reflexive** candidate. One-way signalling, standard ICE from
 * there — and `prflx` on the selected pair afterwards is the only direct
 * evidence that is what happened.
 *
 * `a=setup:active` is hard-coded and is one half of a role assumption made on
 * two machines that never speak. We offer `actpass`; this answer says the
 * **node** is the DTLS client, so this browser is the DTLS server. The node
 * agrees by way of `se.SetAnsweringDTLSRole(webrtc.DTLSRoleClient)` in
 * `krappy-node/internal/punch/punch.go`. Get this backwards and both ends wait
 * for the other's ClientHello: the connection stalls in DTLS with ICE
 * connected and nothing to diagnose. If you change one, change the other.
 *
 * @param {object} args
 * @param {Uint8Array} args.dtls the record's `dtls`, 32 raw bytes
 * @param {{ufrag: string, pwd: string}} args.ice the record's static `ice`
 * @param {string} [args.mid] the mid of OUR offer's data m-line; see the
 *   header — Firefox does not use `0` and a mismatched mid is fatal
 * @param {boolean} [args.trickle] emit `a=ice-options:trickle`. Default false,
 *   which is byte-identical to Go; the browser path passes true.
 * @returns {string} SDP, CRLF-terminated
 */
export function answerFromRecord({ dtls, ice, mid = DEFAULT_MID, trickle = false } = {}) {
  const fp = fingerprintLine(dtls);
  if (!ice || !ice.ufrag || !ice.pwd) {
    throw new PunchError('record', 'punch: record carries no static ICE credentials');
  }
  if (typeof mid !== 'string' || mid === '' || /[\s\r\n]/.test(mid)) {
    throw new PunchError('sdp', `punch: mid ${JSON.stringify(mid)} is not an SDP token`);
  }
  // Guarded because these two land in the SDP verbatim and both come off the
  // wire. RFC 8839's ice-char is [A-Za-z0-9+/]; Go mints them with base64's
  // raw alphabet for exactly that reason, but a record is written by someone
  // else and a newline here would be an SDP injection.
  for (const [name, v] of [['ufrag', ice.ufrag], ['pwd', ice.pwd]]) {
    if (!/^[A-Za-z0-9+/\-_]+$/.test(v)) {
      throw new PunchError('record', `punch: record's ice.${name} is not RFC 8839 ice-char`);
    }
  }

  const lines = [
    'v=0',
    'o=- 0 0 IN IP4 0.0.0.0',
    's=-',
    't=0 0',
    `a=fingerprint:sha-256 ${fp}`,
    `a=group:BUNDLE ${mid}`,
    'm=application 9 UDP/DTLS/SCTP webrtc-datachannel',
    'c=IN IP4 0.0.0.0',
    'a=setup:active',
    `a=mid:${mid}`,
    'a=sendrecv',
    'a=sctp-port:5000',
    'a=max-message-size:1073741823',
    `a=ice-ufrag:${ice.ufrag}`,
    `a=ice-pwd:${ice.pwd}`,
  ];
  if (trickle) lines.push('a=ice-options:trickle');
  return lines.join('\r\n') + '\r\n';
}

// ===========================================================================
// Pure: reading a session description
// ===========================================================================

/**
 * @typedef {object} SdpCandidate
 * @property {string} foundation
 * @property {number} component
 * @property {string} protocol lowercased — `udp` or `tcp`
 * @property {number} priority
 * @property {string} address as written: an IP literal, or an mDNS `.local`
 * @property {number} port
 * @property {string} type `host` | `srflx` | `prflx` | `relay`
 */

/**
 * @typedef {object} ParsedSdp
 * @property {Uint8Array|null} dtls
 * @property {{ufrag: string, pwd: string}|null} ice
 * @property {string|null} setup
 * @property {string|null} mid
 * @property {string[]} bundle mids named by `a=group:BUNDLE`
 * @property {string[]} iceOptions
 * @property {boolean} endOfCandidates
 * @property {SdpCandidate[]} candidates
 */

/**
 * Pull out the handful of facts KRAP-1 cares about.
 *
 * Deliberately not a general SDP parser — there is no need for one, and a
 * partial parser that says so is easier to trust than a general one that
 * silently ignores what it does not model. Session-level and media-level
 * attributes are merged, with media level winning, which is what every browser
 * does for `a=fingerprint`, `a=ice-ufrag` and `a=ice-pwd` in a bundled
 * one-m-line session.
 *
 * @param {string} sdp
 * @returns {ParsedSdp}
 */
export function parseSessionDescription(sdp) {
  if (typeof sdp !== 'string' || sdp === '') {
    throw new PunchError('sdp', 'punch: empty session description');
  }
  const out = {
    dtls: null,
    ice: null,
    setup: null,
    mid: null,
    bundle: [],
    iceOptions: [],
    endOfCandidates: false,
    candidates: [],
  };
  let ufrag = null;
  let pwd = null;

  for (const raw of sdp.split(/\r\n|\r|\n/)) {
    const line = raw.trim();
    if (line === '') continue;
    if (!line.startsWith('a=')) continue;
    const body = line.slice(2);
    const colon = body.indexOf(':');
    const key = colon === -1 ? body : body.slice(0, colon);
    const val = colon === -1 ? '' : body.slice(colon + 1);

    switch (key) {
      case 'fingerprint': {
        const [alg, hex] = val.split(/\s+/, 2);
        // sha-256 only. §4.1 publishes a SHA-256 digest and nothing else, so
        // a sha-1 fingerprint is not a weaker option, it is a different node.
        if (alg && alg.toLowerCase() === 'sha-256' && hex) out.dtls = parseFingerprint(hex);
        break;
      }
      case 'ice-ufrag':
        ufrag = val.trim();
        break;
      case 'ice-pwd':
        pwd = val.trim();
        break;
      case 'setup':
        out.setup = val.trim();
        break;
      case 'mid':
        // First m-line wins; there is exactly one in a datachannel session.
        if (out.mid === null) out.mid = val.trim();
        break;
      case 'group': {
        const parts = val.trim().split(/\s+/);
        if (parts[0] === 'BUNDLE') out.bundle = parts.slice(1);
        break;
      }
      case 'ice-options':
        out.iceOptions = val.trim().split(/\s+/).filter(Boolean);
        break;
      case 'end-of-candidates':
        out.endOfCandidates = true;
        break;
      case 'candidate': {
        const c = parseCandidateLine(val);
        if (c) out.candidates.push(c);
        break;
      }
      default:
        break;
    }
  }
  if (ufrag !== null || pwd !== null) out.ice = { ufrag: ufrag ?? '', pwd: pwd ?? '' };
  return out;
}

/**
 * One `a=candidate:` value, or the `candidate:`-prefixed string an
 * `RTCIceCandidate.candidate` carries.
 *
 * Returns null rather than throwing: candidate lines carry extensions this
 * does not model (`generation`, `network-id`, `ufrag`, tcptype), and a line
 * shaped wrong is a line to skip, not a reason to abandon the SDP.
 *
 * @param {string} value
 * @returns {SdpCandidate|null}
 */
export function parseCandidateLine(value) {
  let s = String(value).trim();
  if (s.startsWith('a=')) s = s.slice(2);
  if (s.startsWith('candidate:')) s = s.slice('candidate:'.length);
  const f = s.split(/\s+/);
  if (f.length < 8 || f[6] !== 'typ') return null;
  const component = Number(f[1]);
  const priority = Number(f[3]);
  const port = Number(f[5]);
  if (!Number.isInteger(component) || !Number.isInteger(port) || !Number.isFinite(priority)) return null;
  if (port < 0 || port > 65535) return null;
  return {
    foundation: f[0],
    component,
    protocol: f[2].toLowerCase(),
    priority,
    address: f[4],
    port,
    type: f[7],
  };
}

// ===========================================================================
// Pure: IP literals
//
// Needed because the filter below has to answer questions about an address
// that JavaScript has no built-in type for, and because "is this even an IP
// literal" is the check that drops Chrome's mDNS `.local` host candidates —
// which a node cannot parse and could not route to if it could.
// ===========================================================================

/**
 * Parse an IPv4 or IPv6 literal into its bytes.
 *
 * @param {string} s
 * @returns {{version: 4|6, bytes: Uint8Array}|null}
 */
export function parseIp(s) {
  const str = String(s);
  if (str.includes(':')) return parseIp6(str);
  return parseIp4(str);
}

function parseIp4(s) {
  const parts = s.split('.');
  if (parts.length !== 4) return null;
  const bytes = new Uint8Array(4);
  for (let i = 0; i < 4; i++) {
    // No leading zeros: `010` is octal to some parsers and decimal to others,
    // and disagreeing about that is how filters get bypassed.
    if (!/^(0|[1-9][0-9]{0,2})$/.test(parts[i])) return null;
    const n = Number(parts[i]);
    if (n > 255) return null;
    bytes[i] = n;
  }
  return { version: 4, bytes };
}

function parseIp6(s) {
  let str = s;
  // A zone index (`fe80::1%eth0`) names an interface that means nothing on
  // another machine, so it cannot be a punch target.
  if (str.includes('%')) return null;
  let tail4 = null;
  const lastColon = str.lastIndexOf(':');
  const maybe4 = str.slice(lastColon + 1);
  if (maybe4.includes('.')) {
    tail4 = parseIp4(maybe4);
    if (!tail4) return null;
    str = str.slice(0, lastColon + 1) + '0:0';
  }
  const halves = str.split('::');
  if (halves.length > 2) return null;
  const groups = (h) => (h === '' ? [] : h.split(':'));
  const head = groups(halves[0]);
  const tail = halves.length === 2 ? groups(halves[1]) : [];
  const total = head.length + tail.length;
  if (halves.length === 1 ? total !== 8 : total > 7) return null;
  const words = [
    ...head,
    ...new Array(8 - total).fill('0'),
    ...tail,
  ];
  if (halves.length === 1 && words.length !== 8) return null;
  const bytes = new Uint8Array(16);
  for (let i = 0; i < 8; i++) {
    if (!/^[0-9a-fA-F]{1,4}$/.test(words[i])) return null;
    const n = parseInt(words[i], 16);
    bytes[i * 2] = n >> 8;
    bytes[i * 2 + 1] = n & 0xff;
  }
  if (tail4) bytes.set(tail4.bytes, 12);
  return { version: 6, bytes };
}

/** `::ffff:a.b.c.d` is an IPv4 address wearing a hat. Go's `Addr.Unmap()`. */
function unmap(ip) {
  if (ip.version !== 6) return ip;
  const b = ip.bytes;
  for (let i = 0; i < 10; i++) if (b[i] !== 0) return ip;
  if (b[10] !== 0xff || b[11] !== 0xff) return ip;
  return { version: 4, bytes: b.slice(12) };
}

function isUnspecified(ip) {
  return ip.bytes.every((b) => b === 0);
}

function isLoopback(ip) {
  if (ip.version === 4) return ip.bytes[0] === 127;
  return ip.bytes.slice(0, 15).every((b) => b === 0) && ip.bytes[15] === 1;
}

function isLinkLocal(ip) {
  if (ip.version === 4) return ip.bytes[0] === 169 && ip.bytes[1] === 254;
  return ip.bytes[0] === 0xfe && (ip.bytes[1] & 0xc0) === 0x80;
}

function isMulticast(ip) {
  if (ip.version === 4) return (ip.bytes[0] & 0xf0) === 0xe0;
  return ip.bytes[0] === 0xff;
}

/**
 * Render bytes back to the textual form a postcard carries. Round-trips
 * through Go's `netip.ParseAddr`, which is what actually has to accept it.
 */
function ipString(ip) {
  if (ip.version === 4) return Array.from(ip.bytes).join('.');
  const words = [];
  for (let i = 0; i < 8; i++) words.push((ip.bytes[i * 2] << 8) | ip.bytes[i * 2 + 1]);
  // Longest run of zeros gets the `::`, RFC 5952 §4.2.
  let bestStart = -1;
  let bestLen = 0;
  let start = -1;
  for (let i = 0; i <= 8; i++) {
    if (i < 8 && words[i] === 0) {
      if (start === -1) start = i;
    } else if (start !== -1) {
      const len = i - start;
      if (len > bestLen && len > 1) {
        bestLen = len;
        bestStart = start;
      }
      start = -1;
    }
  }
  const hex = words.map((w) => w.toString(16));
  if (bestStart === -1) return hex.join(':');
  return `${hex.slice(0, bestStart).join(':')}::${hex.slice(bestStart + bestLen).join(':')}`;
}

// ===========================================================================
// Pure: what goes in the postcard
// ===========================================================================

/**
 * The candidate filter, and it is the mirror image of `usableCandidate` in
 * `krappy-node/internal/punch/sdp.go` — deliberately, because the node applies
 * that filter to whatever this sends, and a candidate the node will drop is a
 * candidate that only costs postcard bytes.
 *
 * The rules, and what each one is actually for:
 *
 *   - **UDP only.** There is no TCP punch. Chrome gathers `tcptype passive`
 *     host candidates by default and they are noise here.
 *   - **Component 1 only.** RTCP-mux, no media; component 2 never appears in a
 *     datachannel session, and if it did it would be the same socket.
 *   - **A parseable IP literal.** This is the one with teeth. Chrome replaces
 *     host candidate addresses with a random `<uuid>.local` mDNS name unless
 *     the page has camera/mic permission, and a node has no way to resolve
 *     that — `netip.ParseAddr` refuses it and `usableCandidate` drops it. So
 *     on Chrome, without STUN, this filter empties the postcard. That is the
 *     concrete reason STUN is load-bearing rather than nice to have.
 *   - **No port 0, unspecified or multicast.** Nonsense targets.
 *   - **No loopback or link-local** unless `allowLocal`, which exists for the
 *     same reason Go's does: a same-machine rehearsal, where they are the only
 *     addresses there are.
 *
 * Ordering is not incidental either. The node walks the list in order and
 * assigns descending priority, so it checks whatever is first. Reflexive
 * candidates go first because the postcard path exists for the case where the
 * two machines are not on the same LAN; host candidates follow, because when
 * they *are*, a host pair is better than a hairpin.
 *
 * @param {SdpCandidate[]} candidates
 * @param {object} [opts]
 * @param {boolean} [opts.allowLocal]
 * @param {number} [opts.max]
 * @returns {Array<{ip: string, port: number}>}
 */
export function postcardCandidates(candidates, { allowLocal = false, max = MAX_POSTCARD_CANDIDATES } = {}) {
  const rank = { srflx: 0, relay: 1, prflx: 2, host: 3 };
  const kept = [];
  for (const c of candidates ?? []) {
    if (!c) continue;
    if (c.protocol !== 'udp') continue;
    if (c.component !== 1) continue;
    if (c.port === 0) continue;
    const parsed = parseIp(c.address);
    if (!parsed) continue;
    const ip = unmap(parsed);
    if (isUnspecified(ip) || isMulticast(ip)) continue;
    if ((isLoopback(ip) || isLinkLocal(ip)) && !allowLocal) continue;
    kept.push({ ip: ipString(ip), port: c.port, _rank: rank[c.type] ?? 9, _prio: c.priority });
  }
  kept.sort((a, b) => a._rank - b._rank || b._prio - a._prio);

  const seen = new Set();
  const out = [];
  for (const c of kept) {
    const key = `${c.ip}:${c.port}`;
    if (seen.has(key)) continue;
    seen.add(key);
    out.push({ ip: c.ip, port: c.port });
    if (out.length >= max) break;
  }
  return out;
}

/**
 * @typedef {object} LocalFacts everything about this browser that goes in a
 *   postcard (§7.6), plus the mid the fabricated answer has to match.
 * @property {Array<{ip: string, port: number}>} cand
 * @property {{ufrag: string, pwd: string}} ice
 * @property {Uint8Array} dtls
 * @property {string} mid
 * @property {number} dropped how many gathered candidates the filter refused
 */

/**
 * Turn this browser's own offer into the postcard's worth of facts.
 *
 * This is the function that exists because a browser will not tell you any of
 * this any other way — see note 4 in the header. It is also the one that can
 * be tested properly offline: give it a session description a real WebRTC
 * stack produced and check that what comes out is what the node needs.
 *
 * @param {string} sdp usually `pc.localDescription.sdp`
 * @param {object} [opts]
 * @param {boolean} [opts.allowLocal]
 * @param {number} [opts.max]
 * @param {SdpCandidate[]} [opts.extraCandidates] candidates seen on
 *   `onicecandidate` that may not have made it into the description yet
 * @returns {LocalFacts}
 */
export function localFromSdp(sdp, { allowLocal = false, max = MAX_POSTCARD_CANDIDATES, extraCandidates = [] } = {}) {
  const parsed = parseSessionDescription(sdp);
  if (!parsed.dtls) {
    throw new PunchError('sdp', 'punch: our own offer carries no sha-256 fingerprint');
  }
  if (!parsed.ice || !parsed.ice.ufrag || !parsed.ice.pwd) {
    throw new PunchError('sdp', 'punch: our own offer carries no ICE credentials');
  }
  const all = [...parsed.candidates, ...extraCandidates];
  const cand = postcardCandidates(all, { allowLocal, max });
  if (cand.length === 0) {
    throw new PunchError(
      'gather',
      `punch: none of the ${all.length} gathered candidates can go in a postcard — ` +
        'with STUN unreachable a browser has only mDNS host candidates, which a node cannot resolve',
      { gathered: all.length },
    );
  }
  return {
    cand,
    ice: parsed.ice,
    dtls: parsed.dtls,
    mid: parsed.mid ?? DEFAULT_MID,
    dropped: all.length - cand.length,
  };
}

/**
 * Check a fabricated or received answer says what we think it says, before
 * handing it to the platform. Cheap, and it turns "the DataChannel never
 * opened" into a sentence.
 *
 * @param {string} sdp
 * @param {object} expect
 * @param {Uint8Array} expect.dtls
 * @param {{ufrag: string, pwd: string}} expect.ice
 * @param {string} [expect.mid]
 */
export function assertAnswerMatches(sdp, { dtls, ice, mid = DEFAULT_MID }) {
  const got = parseSessionDescription(sdp);
  const same = (a, b) => a && b && a.length === b.length && a.every((x, i) => x === b[i]);
  if (!same(got.dtls, dtls)) {
    throw new PunchError('sdp', 'punch: answer fingerprint is not the record\'s `dtls`');
  }
  if (got.ice?.ufrag !== ice.ufrag || got.ice?.pwd !== ice.pwd) {
    throw new PunchError('sdp', 'punch: answer ICE credentials are not the record\'s `ice`');
  }
  if (got.setup !== 'active') {
    throw new PunchError('sdp', `punch: answer says a=setup:${got.setup}, want active — the DTLS roles would deadlock`);
  }
  if (got.mid !== mid) {
    throw new PunchError('sdp', `punch: answer mid is ${got.mid}, our offer used ${mid}`);
  }
  if (got.candidates.length !== 0) {
    throw new PunchError('sdp', 'punch: answer carries candidates — the node has no address to publish');
  }
  return got;
}

// ===========================================================================
// --- browser only ----------------------------------------------------------
//
// Nothing below this line runs under Node: there is no `RTCPeerConnection` in
// Node 22 and this repo deliberately does not install a shim for one, because
// a shim is not Blink and agreement with it would be evidence of nothing.
//
// **It has now executed, in two real browsers** — Chromium 149 and Firefox
// 146, on 2026-08-16, against a real `krappy-bb` and a real
// `krappy-node watch`, ending on an established DataChannel with the selected
// pair's remote candidate reported as `prflx`. `make web-browser-dial`
// (`web/browser-check/dial.mjs`) is that run, and it is the only thing in the
// repo that reaches this code. `make web-punch` does not, and cannot.
//
// What that run settled, of the assumptions the code was written on:
//
//   * the browsers DO accept the node's inbound connectivity check as a
//     peer-reflexive candidate against a remote description carrying no
//     candidates at all — which is the whole protocol, and was a guess;
//   * `a=setup:active` in the fabricated answer is the right way round;
//   * reading the mid back out of our own offer matters — Firefox 146 offered
//     `0`, not `sdparta_0`, so the hard-coded default would have worked here,
//     but the code no longer depends on which.
//
// What it did NOT settle: whether `a=ice-options:trickle` is load-bearing (it
// is emitted, never varied), and anything about a network with a NAT in it.
//
// Keep this section small. If a thing can be decided from strings and bytes,
// it belongs above the line where a test can reach it.
// ===========================================================================

const noopLogger = { debug() {}, info() {}, warn() {} };

/**
 * One browser-side punch. Construct, `gather()`, `expect()`, drop the
 * postcard, `opened()`.
 *
 * The order is `krappy-node/dial.go`'s and it is not cosmetic: `expect()` runs
 * BEFORE the postcard goes out, because the node can answer in milliseconds
 * and an ICE agent with no remote description drops the first connectivity
 * checks on the floor. `dial()` below enforces it; if you drive this class by
 * hand, do not reorder those two.
 */
export class Punch {
  #candidates = [];
  #candidateErrors = [];
  #opened;
  #resolve;
  #reject;

  /**
   * @param {object} args
   * @param {import('./wire.js').DialRecord} args.record a VERIFIED record
   * @param {readonly string[]} [args.stunServers] see DEFAULT_STUN_SERVERS —
   *   this is the third party in the dial path and it is an argument on
   *   purpose. Pass `[]` to gather host candidates only, which on Chrome means
   *   mDNS names and an empty postcard.
   * @param {RTCConfiguration} [args.rtcConfiguration] merged under ours
   * @param {string} [args.label] data channel label
   * @param {boolean} [args.allowLocalCandidates] offer loopback/link-local
   * @param {number} [args.timeoutMs]
   * @param {typeof RTCPeerConnection} [args.PeerConnection] injectable
   * @param {{debug: Function, info: Function, warn: Function}} [args.logger]
   */
  constructor({
    record,
    stunServers = DEFAULT_STUN_SERVERS,
    rtcConfiguration = {},
    label = DEFAULT_CHANNEL_LABEL,
    allowLocalCandidates = false,
    timeoutMs = 30_000,
    PeerConnection,
    logger = noopLogger,
  }) {
    const RTC = PeerConnection ?? globalThis.RTCPeerConnection;
    if (typeof RTC !== 'function') {
      throw new PunchError(
        'platform',
        'punch: no RTCPeerConnection — this needs a browser in a secure context (https, or localhost)',
      );
    }
    if (!record?.dtls) {
      throw new PunchError('record', 'punch: the record has no `dtls`, so this node cannot be punched toward');
    }
    if (!record?.ice?.ufrag || !record?.ice?.pwd) {
      throw new PunchError('record', 'punch: the record has no static `ice` credentials');
    }

    this.record = record;
    this.stunServers = [...stunServers];
    this.label = label;
    this.allowLocalCandidates = allowLocalCandidates;
    this.timeoutMs = timeoutMs;
    this.log = logger;
    this.local = null;

    this.pc = new RTC({
      ...rtcConfiguration,
      iceServers: [
        ...this.stunServers.map((urls) => ({ urls })),
        ...(rtcConfiguration.iceServers ?? []),
      ],
      // No TURN, ever, and `all` rather than `relay` says so out loud. A relay
      // would make the dial work and make the whole premise false.
      iceTransportPolicy: 'all',
      bundlePolicy: 'max-bundle',
    });

    this.pc.addEventListener('icecandidate', (ev) => {
      if (!ev.candidate || !ev.candidate.candidate) return;
      const c = parseCandidateLine(ev.candidate.candidate);
      if (c) this.#candidates.push(c);
    });
    this.pc.addEventListener('icecandidateerror', (ev) => {
      // A STUN failure arrives here and NOWHERE else: gathering still
      // "completes", with nothing reflexive in it. Silence here is how a dial
      // that was doomed at second one looks like a node that never answered.
      this.log.warn('punch: ICE candidate error', {
        url: ev.url,
        errorCode: ev.errorCode,
        errorText: ev.errorText,
      });
      this.#candidateErrors.push({ url: ev.url, code: ev.errorCode, text: ev.errorText });
    });

    this.#opened = new Promise((resolve, reject) => {
      this.#reject = reject;
      this.#resolve = resolve;
    });
    // Nothing awaits this promise until `opened()` is called, and an
    // unhandled rejection in between would be noisy at best.
    this.#opened.catch(() => {});

    this.pc.addEventListener('connectionstatechange', () => {
      const s = this.pc.connectionState;
      this.log.debug('punch: connection state', s);
      if (s === 'failed' || s === 'closed') {
        this.#reject(
          new PunchError('failed', `punch: peer connection ${s}`, {
            candidateErrors: this.#candidateErrors,
          }),
        );
      }
    });

    this.channel = this.pc.createDataChannel(this.label, { ordered: true });
    this.channel.binaryType = 'arraybuffer';
    this.channel.addEventListener('open', () => this.#resolve(this.channel));

    // PARK WHAT ARRIVES BEFORE ANYBODY IS LISTENING.
    //
    // #29's lesson, one layer down and with a different event. `dial()`
    // resolves on the channel's `open`, then awaits `getStats()` for the
    // selected pair, and only then does the caller get to attach a message
    // handler. A peer that speaks first — `krappy-node watch --serve
    // --call-back`, which calls the browser the instant the channel opens —
    // lands in that window, and a DataChannel message dispatched with no
    // listener is gone with no error and no trace. Costing a few bytes to
    // hold them is obviously right; discovering this from a browser is not.
    this.channel.addEventListener('message', (ev) => {
      if (this.#buffering) this.#early.push(ev.data);
    });
  }

  #early = [];
  #buffering = true;

  /**
   * Stop parking messages and hand over what was parked.
   *
   * Call this in the SAME synchronous turn as attaching the real handler —
   * an await in between reopens exactly the window this closes.
   */
  takeEarly() {
    this.#buffering = false;
    return this.#early.splice(0);
  }

  /** The third parties this dial depends on. Print it; do not hide it. */
  thirdParties() {
    return { stun: [...this.stunServers], relay: null };
  }

  /**
   * Create the offer, start gathering, and block until gathering finishes,
   * then return the postcard's worth of facts.
   *
   * @param {object} [opts]
   * @param {AbortSignal} [opts.signal]
   * @returns {Promise<LocalFacts>}
   */
  async gather({ signal } = {}) {
    const offer = await this.pc.createOffer();
    await this.pc.setLocalDescription(offer);
    await this.#gatheringComplete({ signal });

    // Both sources, on purpose. Chrome and Firefox both fold gathered
    // candidates back into `localDescription.sdp`, but that is a convention
    // rather than a guarantee, and the event stream is the normative one.
    this.local = localFromSdp(this.pc.localDescription.sdp, {
      allowLocal: this.allowLocalCandidates,
      extraCandidates: this.#candidates,
    });
    this.log.info('punch: gathered', {
      candidates: this.local.cand.map((c) => `${c.ip}:${c.port}`),
      dropped: this.local.dropped,
      mid: this.local.mid,
      stun: this.stunServers,
    });
    return this.local;
  }

  #gatheringComplete({ signal }) {
    if (this.pc.iceGatheringState === 'complete') return Promise.resolve();
    return new Promise((resolve, reject) => {
      // A browser that cannot reach STUN can sit in `gathering` until its own
      // internal timeout, which is tens of seconds in Chrome. Bounding it here
      // means a dial with no reflexive candidate fails as "no usable
      // candidates" rather than as a timeout with no explanation.
      const timer = setTimeout(() => {
        cleanup();
        resolve();
      }, this.timeoutMs);
      const onState = () => {
        if (this.pc.iceGatheringState === 'complete') {
          cleanup();
          resolve();
        }
      };
      const onAbort = () => {
        cleanup();
        reject(new PunchError('timeout', 'punch: gathering aborted'));
      };
      const cleanup = () => {
        clearTimeout(timer);
        this.pc.removeEventListener('icegatheringstatechange', onState);
        signal?.removeEventListener('abort', onAbort);
      };
      this.pc.addEventListener('icegatheringstatechange', onState);
      signal?.addEventListener('abort', onAbort, { once: true });
    });
  }

  /**
   * Install the node's fabricated session description. `client.Expect` in Go.
   *
   * Must happen before the postcard is dropped. Must happen after `gather`,
   * because the answer's mid has to be the one our own offer used.
   */
  async expect() {
    if (!this.local) throw new PunchError('sdp', 'punch: expect() before gather()');
    const sdp = answerFromRecord({
      dtls: this.record.dtls,
      ice: this.record.ice,
      mid: this.local.mid,
      trickle: true,
    });
    assertAnswerMatches(sdp, { dtls: this.record.dtls, ice: this.record.ice, mid: this.local.mid });
    await this.pc.setRemoteDescription({ type: 'answer', sdp });
    return sdp;
  }

  /**
   * Block until the data channel opens.
   *
   * @param {object} [opts]
   * @param {AbortSignal} [opts.signal]
   * @returns {Promise<RTCDataChannel>}
   */
  opened({ signal } = {}) {
    if (this.channel.readyState === 'open') return Promise.resolve(this.channel);
    return new Promise((resolve, reject) => {
      const timer = setTimeout(() => {
        reject(
          new PunchError('timeout', `punch: no data channel within ${this.timeoutMs}ms`, {
            connectionState: this.pc.connectionState,
            iceConnectionState: this.pc.iceConnectionState,
            candidateErrors: this.#candidateErrors,
          }),
        );
      }, this.timeoutMs);
      const onAbort = () => reject(new PunchError('timeout', 'punch: aborted'));
      signal?.addEventListener('abort', onAbort, { once: true });
      this.#opened.then(resolve, reject).finally(() => {
        clearTimeout(timer);
        signal?.removeEventListener('abort', onAbort);
      });
    });
  }

  /**
   * The candidate pair ICE settled on, and the one assertion worth making
   * about it.
   *
   * `remote.type === 'prflx'` is the load-bearing fact: it means the node's
   * address was never signalled and was learned peer-reflexively from its own
   * connectivity checks. Anything else means the connection went somewhere
   * that had already been published, which is a different protocol.
   *
   * @returns {Promise<{local: object, remote: object, prflx: boolean, text: string}|null>}
   */
  async selectedPair() {
    const stats = await this.pc.getStats();
    let pair = null;
    // Chrome exposes it from the transport; Firefox flags the pair itself.
    for (const r of stats.values()) {
      if (r.type === 'transport' && r.selectedCandidatePairId) {
        pair = stats.get(r.selectedCandidatePairId) ?? pair;
      }
    }
    if (!pair) {
      for (const r of stats.values()) {
        if (r.type === 'candidate-pair' && (r.selected || (r.nominated && r.state === 'succeeded'))) pair = r;
      }
    }
    if (!pair) return null;
    const local = stats.get(pair.localCandidateId) ?? {};
    const remote = stats.get(pair.remoteCandidateId) ?? {};
    const type = normaliseCandidateType(remote.candidateType);
    // Both engines withhold the address of a peer-reflexive remote candidate,
    // in different ways: Chromium 149 reports an EMPTY STRING (so `??` is not
    // enough — it reads as `remote :43508`) and Firefox 146 reports the
    // literal `(redacted)`. Both observed the first time a browser ran this.
    // Say what is missing rather than printing a hole; the assertion that
    // matters is `type`, and the address of a prflx candidate is precisely the
    // thing the protocol never signalled.
    const at = (c) => `${c.address || c.ip || '(address not reported)'}:${c.port ?? '?'}`;
    return {
      local: { address: local.address ?? local.ip, port: local.port, type: normaliseCandidateType(local.candidateType) },
      remote: { address: remote.address ?? remote.ip, port: remote.port, type },
      prflx: type === 'prflx',
      text: `local ${at(local)} <- remote ${at(remote)} (${type})`,
    };
  }

  close() {
    try {
      this.pc.close();
    } catch {
      /* already gone */
    }
  }
}

/**
 * Older Firefox spelled these out in `RTCIceCandidateStats.candidateType`.
 * Normalising means the `prflx` assertion is about what happened rather than
 * about which browser is reporting it.
 */
function normaliseCandidateType(t) {
  switch (t) {
    case 'peer-reflexive':
    case 'peerreflexive':
      return 'prflx';
    case 'server-reflexive':
    case 'serverreflexive':
      return 'srflx';
    case 'relayed':
      return 'relay';
    default:
      return t;
  }
}

/**
 * The whole client sequence, in `krappy-node/dial.go`'s order.
 *
 * @param {object} args
 * @param {import('./wire.js').DialRecord} args.record a VERIFIED Dial Record
 * @param {import('./board.js').BoardClient} args.board where the postcard goes
 * @param {{signPub: Uint8Array, sign: Function}} args.keystore this host
 * @param {readonly string[]} [args.stunServers]
 * @param {boolean} [args.allowLocalCandidates]
 * @param {string} [args.label]
 * @param {number} [args.timeoutMs]
 * @param {AbortSignal} [args.signal]
 * @param {object} [args.logger]
 * @param {typeof RTCPeerConnection} [args.PeerConnection]
 * @returns {Promise<{punch: Punch, channel: RTCDataChannel, seq: number, topic: string, pair: object|null}>}
 */
export async function dial({
  record,
  board,
  keystore,
  stunServers,
  allowLocalCandidates = false,
  label,
  timeoutMs = 30_000,
  signal,
  logger = noopLogger,
  PeerConnection,
}) {
  const punch = new Punch({
    record,
    stunServers,
    label,
    allowLocalCandidates,
    timeoutMs,
    logger,
    PeerConnection,
  });
  try {
    const local = await punch.gather({ signal });

    // BEFORE the drop. See the class comment, and dial.go:131.
    await punch.expect();

    const { seq, topic } = await board.drop({
      record,
      keystore,
      cand: local.cand,
      ice: local.ice,
      dtls: local.dtls,
      signal,
    });
    logger.info?.('punch: postcard dropped', { board: board.origin, topic, seq });

    const channel = await punch.opened({ signal });
    const pair = await punch.selectedPair();
    logger.info?.('punch: data channel open', { label: channel.label, pair: pair?.text, prflx: pair?.prflx });
    return { punch, channel, seq, topic, pair };
  } catch (e) {
    punch.close();
    throw e;
  }
}
