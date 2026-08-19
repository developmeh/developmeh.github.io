// The KRAP-1 signed structures, for the browser host.
//
// This is the JS counterpart of `krappy-proto/{record,grant,postcard,drop}.go`
// and it must agree with it BYTE FOR BYTE. Everything here is signed over
// canonical CBOR, so "agrees" is not a figure of speech: a field emitted in
// the wrong order, an `omitempty` field written as null, or a uint encoded in
// a wider form than it needs produces a signature no Go verifier reproduces.
// `web/test-wire.mjs` holds every encoder here to the golden vectors in
// `krappy-proto/testdata/vectors.json`, which Go generated.
//
// The strictness is not decoration. Each `decode*` below re-encodes what it
// decoded and rejects any difference, which is how Go's `UnmarshalCanonical`
// plus `ExtraDecErrorUnknownField` behave in one step: an unknown field, a
// non-canonical integer, an explicit zero where `omitempty` would have
// omitted, and a re-ordered map all fail the same comparison. Anything that
// survives it is bytes this implementation could itself have produced, which
// is the only property that makes a signature over them meaningful.
//
// SCOPE: this file is the wire format and nothing else. It performs no I/O,
// knows no URLs, and speaks no HTTP — the board client (§7.9) is a separate
// concern and a separate file.

import { decode, decodeCanonical, encode, Raw } from './cbor.js';
import { nodeIdFromPub, pubFromNodeId } from './proto.js';

/** Wire version carried in every signed structure (Go's `Version`). */
export const VERSION = 1;

/** Go's `MaxSkew`: how far a drop's `ts` may sit from board time. */
export const MAX_SKEW_SECONDS = 120;

/** Go's `DropNonceLen`. */
export const DROP_NONCE_LEN = 16;

const ED25519_KEY_LEN = 32;
const X25519_KEY_LEN = 32;

/**
 * A wire-format failure, with a `code` mirroring `krappy-proto/errors.go`.
 *
 * `signature` is deliberately unspecific about WHICH check failed — whether
 * the key, the payload or the signature was wrong is a distinction only an
 * attacker benefits from, and Go's `ErrBadSignature` makes the same choice.
 *
 * @property {'version'|'key-length'|'signature'|'not-canonical'|'expired'|'malformed'} code
 */
export class WireError extends Error {
  constructor(code, message) {
    super(message);
    this.name = 'WireError';
    this.code = code;
  }
}

const fail = (code, message) => {
  throw new WireError(code, message);
};

const isBytes = (x, len) => x instanceof Uint8Array && (len === undefined || x.length === len);

const bytesEqual = (a, b) => a.length === b.length && a.every((x, i) => x === b[i]);

// ---------------------------------------------------------------------------
// ed25519
// ---------------------------------------------------------------------------

/**
 * Verify a raw-key ed25519 signature. Never throws for a bad signature —
 * a malformed key and a wrong signature both return false.
 *
 * @param {Uint8Array} pub 32 raw bytes
 * @param {Uint8Array} data
 * @param {Uint8Array} sig 64 bytes
 * @returns {Promise<boolean>}
 */
export async function verifyEd25519(pub, data, sig) {
  if (!isBytes(pub, ED25519_KEY_LEN) || !isBytes(sig) || !isBytes(data)) return false;
  try {
    const key = await crypto.subtle.importKey('raw', pub, { name: 'Ed25519' }, false, ['verify']);
    return await crypto.subtle.verify('Ed25519', key, sig, data);
  } catch {
    return false;
  }
}

// ---------------------------------------------------------------------------
// the strict-decode discipline
// ---------------------------------------------------------------------------

/**
 * Decode `bytes`, then prove the result re-encodes to exactly `bytes`.
 *
 * Everything strict about this layer funnels through here. The comparison
 * catches, in one shot: non-canonical CBOR, a field the structure does not
 * define, a field encoded where `omitempty` would have omitted it, and a
 * value whose JS representation is lossy. Go gets the same coverage from
 * `UnmarshalCanonical` plus `ExtraDecErrorUnknownField`.
 */
function strictDecode(bytes, toStruct, fromStruct, what) {
  if (!isBytes(bytes)) fail('malformed', `${what}: expected bytes`);
  let map;
  try {
    map = decodeCanonical(bytes);
  } catch (e) {
    fail('not-canonical', `${what}: ${e.message}`);
  }
  if (!(map instanceof Map)) fail('malformed', `${what}: not a CBOR map`);
  const value = toStruct(map, what);
  const round = fromStruct(value);
  if (!bytesEqual(round, bytes)) {
    fail(
      'not-canonical',
      `${what}: re-encoding the decoded structure does not reproduce the input — ` +
        'it carries an unknown field, an explicitly-encoded empty value, or a ' +
        'non-canonical encoding, and a signature over it is not reproducible',
    );
  }
  return value;
}

/** Read a required unsigned integer. */
function uint(map, key, what) {
  const v = map.get(key);
  if (typeof v === 'bigint') return v;
  if (typeof v !== 'number' || !Number.isInteger(v) || v < 0) {
    fail('malformed', `${what}: \`${key}\` must be an unsigned integer`);
  }
  return v;
}

/** Read an optional unsigned integer; Go's `omitempty` makes absent == 0. */
function optUint(map, key, what) {
  return map.has(key) ? uint(map, key, what) : 0;
}

function text(map, key, what) {
  const v = map.get(key);
  if (typeof v !== 'string') fail('malformed', `${what}: \`${key}\` must be a text string`);
  return v;
}

function blob(map, key, what, len) {
  const v = map.get(key);
  if (!isBytes(v)) fail('malformed', `${what}: \`${key}\` must be a byte string`);
  if (len !== undefined && v.length !== len) {
    fail('key-length', `${what}: \`${key}\` is ${v.length} bytes, want ${len}`);
  }
  return v;
}

/**
 * A field Go declares without `omitempty` but whose Go zero value is nil, so
 * an absent value on the wire is CBOR null rather than omission.
 */
function nullableBlob(map, key, what) {
  const v = map.get(key);
  if (v === null) return null;
  if (!isBytes(v)) fail('malformed', `${what}: \`${key}\` must be a byte string or null`);
  return v;
}

function iceFrom(map, key, what) {
  const v = map.get(key);
  if (!(v instanceof Map)) fail('malformed', `${what}: \`${key}\` must be a map`);
  return { ufrag: text(v, 'ufrag', what), pwd: text(v, 'pwd', what) };
}

const iceTo = (ice) => new Map([['ufrag', ice?.ufrag ?? ''], ['pwd', ice?.pwd ?? '']]);

// ---------------------------------------------------------------------------
// Dial Record (KRAP-1 §4.1)
// ---------------------------------------------------------------------------

/**
 * @typedef {object} DialRecord
 * @property {number|bigint} v
 * @property {Uint8Array} id       ed25519 public key (32)
 * @property {Uint8Array} enc      X25519 public key (32), §7.6's recipient key
 * @property {number|bigint} seq
 * @property {number|bigint} exp   unix seconds
 * @property {Array<{kind: string, address: string, port: number}>|null} eps
 * @property {Uint8Array|null} dtls SHA-256 of the long-lived DTLS cert
 * @property {{ufrag: string, pwd: string}} ice
 * @property {Uint8Array[]} wtch   WebTransport cert hashes; may be empty
 * @property {Array<{kind: string, params: Raw}>} mbx  §7.2 descriptors
 * @property {Uint8Array|null} grant  the open grant (§7.5), or null
 * @property {number|bigint} checkin  stated check-in cadence in seconds, 0 = none
 * @property {Uint8Array|null} sig
 */

/**
 * Encode a Dial Record as canonical CBOR.
 *
 * `wtch`, `mbx`, `grant`, `checkin` and `sig` carry Go's `omitempty` and are
 * OMITTED when empty rather than encoded as null or as an empty container —
 * a distinction that changes the signed bytes. `eps`, `dtls` and `ice` do
 * not: Go declares them without `omitempty`, so a nil slice reaches the wire
 * as CBOR null and a verifier that omitted them would produce a preimage the
 * node never signed.
 *
 * @param {Partial<DialRecord>} rec
 * @returns {Uint8Array}
 */
export function encodeRecord(rec) {
  const m = new Map();
  m.set('v', rec.v ?? VERSION);
  m.set('id', rec.id);
  m.set('enc', rec.enc);
  m.set('seq', rec.seq ?? 0);
  m.set('exp', rec.exp ?? 0);
  m.set('eps', rec.eps ? rec.eps.map((e) => [e.kind, e.address, e.port]) : null);
  m.set('dtls', rec.dtls ?? null);
  m.set('ice', iceTo(rec.ice));
  if (rec.wtch?.length) m.set('wtch', rec.wtch);
  if (rec.mbx?.length) m.set('mbx', rec.mbx.map((b) => [b.kind, b.params]));
  if (rec.grant?.length) m.set('grant', rec.grant);
  if (rec.checkin) m.set('checkin', rec.checkin);
  if (rec.sig?.length) m.set('sig', rec.sig);
  return encode(m);
}

function recordFromMap(map, what) {
  const eps = map.get('eps');
  if (eps !== null && !Array.isArray(eps)) fail('malformed', `${what}: \`eps\` must be an array`);
  const mbx = map.get('mbx');
  if (mbx !== undefined && !Array.isArray(mbx)) fail('malformed', `${what}: \`mbx\` must be an array`);
  const wtch = map.get('wtch');
  if (wtch !== undefined && !Array.isArray(wtch)) fail('malformed', `${what}: \`wtch\` must be an array`);

  return {
    v: uint(map, 'v', what),
    id: blob(map, 'id', what, ED25519_KEY_LEN),
    enc: blob(map, 'enc', what, X25519_KEY_LEN),
    seq: uint(map, 'seq', what),
    exp: uint(map, 'exp', what),
    eps:
      eps === null
        ? null
        : eps.map((e) => {
            if (!Array.isArray(e) || e.length !== 3) {
              fail('malformed', `${what}: an endpoint is [kind, address, port]`);
            }
            return { kind: e[0], address: e[1], port: e[2] };
          }),
    dtls: nullableBlob(map, 'dtls', what),
    ice: iceFrom(map, 'ice', what),
    // §7.2 params stay RAW. The kinds are open-ended, and a record must
    // round-trip a descriptor this implementation does not understand
    // without disturbing the bytes the node's signature covers.
    wtch: wtch ?? [],
    mbx: (mbx ?? []).map((b) => {
      if (!Array.isArray(b) || b.length !== 2) {
        fail('malformed', `${what}: a mailbox descriptor is [kind, params]`);
      }
      return { kind: b[0], params: new Raw(encode(b[1])) };
    }),
    grant: map.has('grant') ? blob(map, 'grant', what) : null,
    checkin: optUint(map, 'checkin', what),
    sig: map.has('sig') ? blob(map, 'sig', what) : null,
  };
}

/**
 * Decode a Dial Record WITHOUT verifying it. Almost every caller wants
 * `parseRecord` instead; this exists for tooling that needs to look at a
 * record it already knows is untrustworthy.
 *
 * @param {Uint8Array} bytes
 * @returns {DialRecord}
 */
export function decodeRecord(bytes) {
  return strictDecode(bytes, recordFromMap, encodeRecord, 'dial record');
}

/** The canonical CBOR a record's signature covers: every field but `sig`. */
export function recordSigningBytes(rec) {
  return encodeRecord({ ...rec, sig: null });
}

/** The base36 NodeID (KRAP-1 §2) implied by a record's `id`. */
export const recordNodeId = (rec) => nodeIdFromPub(rec.id);

/**
 * Freshness is a liveness beacon, not just a validity check: past `exp` the
 * node is down or negligent, and a client should say so rather than spin.
 * Mirrors Go's `Record.Fresh`.
 */
export const recordFresh = (rec, nowSeconds) => BigInt(nowSeconds) < BigInt(rec.exp);

/**
 * Decode and verify a published Dial Record — the thing a browser starts from.
 *
 * Verification is baked into parsing, as in Go's `ParseRecord`, so no caller
 * can accidentally hold an unverified record. The signature is checked
 * against the key the record itself carries in `id`; that is not circular,
 * because `id` is also what the NodeID names, and the caller MUST compare
 * `nodeId` against the identity it actually meant to reach. Pass `expectId`
 * and this does it.
 *
 * @param {Uint8Array} bytes
 * @param {object} [opts]
 * @param {number} [opts.now] unix seconds; when given, an expired record is rejected
 * @param {string} [opts.expectId] base36 NodeID this record must belong to
 * @returns {Promise<DialRecord & {nodeId: string}>}
 */
export async function parseRecord(bytes, { now, expectId } = {}) {
  const rec = decodeRecord(bytes);
  if (Number(rec.v) !== VERSION) fail('version', `dial record: unsupported version ${rec.v}`);

  const ok = await verifyEd25519(rec.id, recordSigningBytes(rec), rec.sig ?? new Uint8Array(0));
  if (!ok) fail('signature', 'dial record: signature does not verify');

  const nodeId = nodeIdFromPub(rec.id);
  if (expectId !== undefined && expectId !== nodeId) {
    fail('signature', `dial record: signed by ${nodeId}, not by ${expectId}`);
  }
  if (now !== undefined && !recordFresh(rec, now)) {
    fail('expired', `dial record: expired at ${rec.exp}, now ${now}`);
  }
  return { ...rec, nodeId };
}

/**
 * Decode a record published in the base64url form KRAP-1 §4.3 allows for DNS
 * TXT, falling back to raw CBOR. Mirrors `krappy-node`'s `parseEither`.
 *
 * @param {Uint8Array|string} published
 * @param {object} [opts] as `parseRecord`
 */
export async function parsePublishedRecord(published, opts) {
  if (published instanceof Uint8Array) {
    try {
      return await parseRecord(published, opts);
    } catch (cborErr) {
      const text = new TextDecoder().decode(published).trim();
      const decoded = tryBase64Url(text);
      // Report the CBOR failure, not the base64 one: raw CBOR is the
      // published form, so that is the error the caller most likely wants.
      if (!decoded) throw cborErr;
      return parseRecord(decoded, opts);
    }
  }
  const decoded = tryBase64Url(String(published).trim());
  if (!decoded) fail('malformed', 'dial record: not base64url');
  return parseRecord(decoded, opts);
}

function tryBase64Url(s) {
  if (!/^[A-Za-z0-9_-]+$/.test(s)) return null;
  const b64 = s.replaceAll('-', '+').replaceAll('_', '/');
  try {
    const bin = atob(b64.padEnd(Math.ceil(b64.length / 4) * 4, '='));
    return Uint8Array.from(bin, (c) => c.charCodeAt(0));
  } catch {
    return null;
  }
}

/**
 * The `bb` board origins a record advertises, in the node's preference order.
 *
 * A record accessor rather than a board client: it reads §7.2 descriptors and
 * returns strings. Descriptors of other kinds are skipped rather than being
 * an error — `mbx` is deliberately extensible, and a client that chokes on a
 * kind it does not know cannot dial a node that lists one first.
 *
 * @param {DialRecord} rec
 * @returns {string[]}
 */
export function boardsFromRecord(rec) {
  const out = [];
  for (const box of rec.mbx ?? []) {
    if (box.kind !== 'bb') continue;
    let params;
    try {
      params = decode(box.params.bytes);
    } catch {
      continue;
    }
    const url = params instanceof Map ? params.get('url') : undefined;
    if (typeof url === 'string' && url !== '') out.push(url.replace(/\/+$/, ''));
  }
  return out;
}

// ---------------------------------------------------------------------------
// Grant (KRAP-1 §7.5)
// ---------------------------------------------------------------------------

/**
 * Encode a grant. A browser never mints one — a grant is signed by the NODE —
 * but it re-encodes one constantly: the drop carries the grant verbatim, and
 * checking that this implementation reproduces the node's exact bytes is what
 * proves the copy it forwards is the copy the board will verify.
 */
export function encodeGrant(g) {
  const m = new Map();
  m.set('v', g.v ?? VERSION);
  m.set('iss', g.iss);
  m.set('brd', g.brd);
  m.set('exp', g.exp);
  m.set('lim', new Map([
    ['rate', g.lim?.rate ?? 0],
    ['size', g.lim?.size ?? 0],
    ['pow', g.lim?.pow ?? 0],
  ]));
  if (g.aud?.length) m.set('aud', g.aud);
  if (g.sig?.length) m.set('sig', g.sig);
  return encode(m);
}

function grantFromMap(map, what) {
  const lim = map.get('lim');
  if (!(lim instanceof Map)) fail('malformed', `${what}: \`lim\` must be a map`);
  return {
    v: uint(map, 'v', what),
    iss: blob(map, 'iss', what, ED25519_KEY_LEN),
    brd: text(map, 'brd', what),
    exp: uint(map, 'exp', what),
    lim: {
      rate: uint(lim, 'rate', what),
      size: uint(lim, 'size', what),
      pow: uint(lim, 'pow', what),
    },
    aud: map.has('aud') ? blob(map, 'aud', what) : null,
    sig: map.has('sig') ? blob(map, 'sig', what) : null,
  };
}

/** @returns {{v, iss: Uint8Array, brd: string, exp, lim: {rate, size, pow}, aud: Uint8Array|null, sig: Uint8Array|null}} */
export function decodeGrant(bytes) {
  return strictDecode(bytes, grantFromMap, encodeGrant, 'grant');
}

export const grantSigningBytes = (g) => encodeGrant({ ...g, sig: null });

/**
 * Decode a grant and verify it was signed by the node that issued it.
 *
 * A client checks this before spending proof of work: a grant that does not
 * verify will be rejected by the board at §7.7 step 8 anyway, and finding
 * that out after minting is pure waste.
 *
 * `iss` is checked against `expectIss` when given — a browser should always
 * pass the record's `id`, because a grant lifted from another node's record
 * verifies perfectly well under ITS issuer and is useless here.
 */
export async function parseGrant(bytes, { expectIss, now } = {}) {
  const g = decodeGrant(bytes);
  if (Number(g.v) !== VERSION) fail('version', `grant: unsupported version ${g.v}`);
  const ok = await verifyEd25519(g.iss, grantSigningBytes(g), g.sig ?? new Uint8Array(0));
  if (!ok) fail('signature', 'grant: signature does not verify');
  if (expectIss !== undefined && !bytesEqual(g.iss, expectIss)) {
    fail('signature', 'grant: issued by a different node than the record it came with');
  }
  if (now !== undefined && BigInt(now) >= BigInt(g.exp)) {
    fail('expired', `grant: expired at ${g.exp}, now ${now}`);
  }
  return g;
}

// ---------------------------------------------------------------------------
// Postcard (KRAP-1 §7.6)
// ---------------------------------------------------------------------------

/**
 * Encode the postcard plaintext — the thing that gets sealed, never the thing
 * that reaches a board.
 *
 * Note the asymmetry between `to` and `from`, and do not "fix" it: `to` is the
 * 50-character base36 NodeID text, `from` is 32 RAW BYTES. §7.6 writes it down
 * rather than correcting it because a postcard is covered by cross-language
 * vectors, so respelling either one is a breaking change for implementations
 * this repo cannot test.
 *
 * @param {object} p
 * @param {string} p.to destination NodeID (base36, 50 chars)
 * @param {Uint8Array} p.from client host ed25519 public key (32 raw bytes)
 * @param {Array<{ip: string, port: number}>|Array<[string, number]>} p.cand
 * @param {{ufrag: string, pwd: string}} p.ice
 * @param {Uint8Array} p.dtls client DTLS cert fingerprint
 * @param {number|bigint} p.ts
 * @param {Uint8Array} p.nonce
 * @returns {Uint8Array}
 */
export function encodePostcard(p) {
  const m = new Map();
  m.set('v', p.v ?? VERSION);
  m.set('to', p.to);
  m.set('from', p.from);
  m.set(
    'cand',
    p.cand ? p.cand.map((c) => (Array.isArray(c) ? [c[0], c[1]] : [c.ip, c.port])) : null,
  );
  m.set('ice', iceTo(p.ice));
  m.set('dtls', p.dtls ?? null);
  m.set('ts', p.ts ?? 0);
  m.set('nonce', p.nonce ?? null);
  return encode(m);
}

function postcardFromMap(map, what) {
  const cand = map.get('cand');
  if (cand !== null && !Array.isArray(cand)) fail('malformed', `${what}: \`cand\` must be an array`);
  return {
    v: uint(map, 'v', what),
    to: text(map, 'to', what),
    from: blob(map, 'from', what, ED25519_KEY_LEN),
    cand:
      cand === null
        ? null
        : cand.map((c) => {
            if (!Array.isArray(c) || c.length !== 2) {
              fail('malformed', `${what}: a candidate is [ip, port]`);
            }
            return { ip: c[0], port: c[1] };
          }),
    ice: iceFrom(map, 'ice', what),
    dtls: nullableBlob(map, 'dtls', what),
    ts: uint(map, 'ts', what),
    nonce: nullableBlob(map, 'nonce', what),
  };
}

/**
 * Decode a postcard opened from a sealed box.
 *
 * The caller MUST also check `to` against its own NodeID. The seal already
 * proves the postcard was addressed to this host's `enc` key, but checking
 * `to` as well means a host never acts on a postcard it merely happens to be
 * able to decrypt.
 */
export function decodePostcard(bytes) {
  const p = strictDecode(bytes, postcardFromMap, encodePostcard, 'postcard');
  if (Number(p.v) !== VERSION) fail('version', `postcard: unsupported version ${p.v}`);
  return p;
}

// ---------------------------------------------------------------------------
// Drop (KRAP-1 §7.7)
// ---------------------------------------------------------------------------

/** Canonical CBOR for a drop. `sig` is omitted when absent, per Go. */
export function encodeDrop(d) {
  const m = new Map();
  m.set('v', d.v ?? VERSION);
  m.set('grant', d.grant);
  m.set('cpk', d.cpk);
  m.set('pow', d.pow);
  m.set('ct', d.ct);
  m.set('ts', d.ts);
  m.set('nonce', d.nonce);
  if (d.sig?.length) m.set('sig', d.sig);
  return encode(m);
}

function dropFromMap(map, what) {
  return {
    v: uint(map, 'v', what),
    grant: blob(map, 'grant', what),
    cpk: blob(map, 'cpk', what, ED25519_KEY_LEN),
    pow: blob(map, 'pow', what),
    ct: blob(map, 'ct', what),
    ts: uint(map, 'ts', what),
    nonce: blob(map, 'nonce', what),
    sig: map.has('sig') ? blob(map, 'sig', what) : null,
  };
}

/** The canonical CBOR a drop's signature covers: every field but `sig`. */
export const dropSigningBytes = (d) => encodeDrop({ ...d, sig: null });

/** Decode a drop body, as a board does. */
export function decodeDrop(bytes) {
  const d = strictDecode(bytes, dropFromMap, encodeDrop, 'drop');
  if (Number(d.v) !== VERSION) fail('version', `drop: unsupported version ${d.v}`);
  if (d.nonce.length !== DROP_NONCE_LEN) {
    fail('malformed', `drop: nonce is ${d.nonce.length} bytes, want ${DROP_NONCE_LEN}`);
  }
  return d;
}

/** Verify a drop's own signature under its `cpk`. @returns {Promise<boolean>} */
export function verifyDrop(d) {
  return verifyEd25519(d.cpk, dropSigningBytes(d), d.sig ?? new Uint8Array(0));
}

/**
 * Sign a drop with the host's durable identity key.
 *
 * `signer` is the `identity.js` Keystore — `{signPub, sign(data)}` — and
 * deliberately not a second keystore. `cpk` MUST be the host's DURABLE
 * per-origin key: §7.7's signature is a rate limit, and a rate limit against
 * a key that costs nothing to rotate is not one.
 *
 * @param {{grant: Uint8Array, pow: Uint8Array, ct: Uint8Array, ts: number|bigint, nonce: Uint8Array}} fields
 * @param {{signPub: Uint8Array, sign(data: Uint8Array): Promise<Uint8Array>}} signer
 * @returns {Promise<{drop: object, bytes: Uint8Array}>}
 */
export async function signDrop(fields, signer) {
  if (!isBytes(signer?.signPub, ED25519_KEY_LEN) || typeof signer?.sign !== 'function') {
    fail('key-length', 'signDrop wants a keystore with a 32-byte signPub and a sign()');
  }
  const drop = {
    v: VERSION,
    grant: fields.grant,
    cpk: signer.signPub,
    pow: fields.pow,
    ct: fields.ct,
    ts: fields.ts,
    nonce: fields.nonce,
    sig: null,
  };
  drop.sig = await signer.sign(dropSigningBytes(drop));
  return { drop, bytes: encodeDrop(drop) };
}

/** A fresh 16-byte §7.7 replay nonce. */
export function newDropNonce() {
  const n = new Uint8Array(DROP_NONCE_LEN);
  crypto.getRandomValues(n);
  return n;
}

/** Mirrors Go's `Drop.FreshEnough`. */
export function dropFreshEnough(d, nowSeconds) {
  const delta = BigInt(nowSeconds) - BigInt(d.ts);
  return (delta < 0n ? -delta : delta) <= BigInt(MAX_SKEW_SECONDS);
}

// ---------------------------------------------------------------------------
// Krappy-Auth token (KRAP-1 §7.9.5)
// ---------------------------------------------------------------------------
// The counterpart of `krappy-proto/auth.go`. It lives here rather than in the
// board client for the same reason everything else here does: it is a signed
// canonical-CBOR structure, and a board re-encodes it to check the signature.
// The board client owns the base64url framing and the header name; this owns
// the bytes.
//
// `topic` is inside the signature and that is load-bearing rather than
// decorative (§7.9.5): without it a token captured from one long-poll is a
// bearer token for every other topic on the same board. §7.7's older prose
// still describes the token as `{ts, nonce, sig}`, which is wrong — see
// `krappy-bb-cf/README.md`, finding 1.

/** Canonical CBOR for an auth token. `sig` is omitted when absent, per Go. */
export function encodeAuth(a) {
  const m = new Map();
  m.set('v', a.v ?? VERSION);
  m.set('topic', a.topic);
  m.set('ts', a.ts);
  m.set('nonce', a.nonce);
  if (a.sig?.length) m.set('sig', a.sig);
  return encode(m);
}

function authFromMap(map, what) {
  return {
    v: uint(map, 'v', what),
    topic: text(map, 'topic', what),
    ts: uint(map, 'ts', what),
    nonce: blob(map, 'nonce', what),
    sig: map.has('sig') ? blob(map, 'sig', what) : null,
  };
}

/** The canonical CBOR an auth token's signature covers: every field but `sig`. */
export const authSigningBytes = (a) => encodeAuth({ ...a, sig: null });

/** Decode an auth token, as a board does. */
export function decodeAuth(bytes) {
  const a = strictDecode(bytes, authFromMap, encodeAuth, 'auth');
  if (Number(a.v) !== VERSION) fail('version', `auth: unsupported version ${a.v}`);
  return a;
}

/** A fresh 16-byte auth nonce. §7.9.5: a client MUST NOT reuse one. */
export function newAuthNonce() {
  const n = new Uint8Array(16);
  crypto.getRandomValues(n);
  return n;
}

/**
 * Mint a `Krappy-Auth` token for one topic, signed by the TENANT key.
 *
 * `signer` is an `identity.js` Keystore, exactly as `signDrop` takes one — but
 * the key is a different one in the general case: a drop is signed by the
 * client host, a read by the node that owns the mailbox. A browser host only
 * ever mints this for topics derived from its own key.
 *
 * @param {{topic: string, ts?: number|bigint, nonce?: Uint8Array}} fields
 * @param {{signPub: Uint8Array, sign(data: Uint8Array): Promise<Uint8Array>}} signer
 * @returns {Promise<{auth: object, bytes: Uint8Array}>}
 */
export async function signAuth(fields, signer) {
  if (!isBytes(signer?.signPub, ED25519_KEY_LEN) || typeof signer?.sign !== 'function') {
    fail('key-length', 'signAuth wants a keystore with a 32-byte signPub and a sign()');
  }
  if (typeof fields?.topic !== 'string' || fields.topic === '') {
    fail('malformed', 'signAuth: a token must name the topic it is spent on');
  }
  const auth = {
    v: VERSION,
    topic: fields.topic,
    ts: fields.ts ?? Math.floor(Date.now() / 1000),
    nonce: fields.nonce ?? newAuthNonce(),
    sig: null,
  };
  auth.sig = await signer.sign(authSigningBytes(auth));
  return { auth, bytes: encodeAuth(auth) };
}

/**
 * Verify an auth token the way a board does: version, topic binding, skew,
 * then the signature under the key that owns the topic.
 *
 * Mirrors Go's `Auth.Verify`, including its refusal to say which check failed
 * — §7.9.8 makes `401` cover both "no header" and "bad signature" because
 * distinguishing them tells a prober that a topic is live.
 *
 * @returns {Promise<boolean>}
 */
export async function verifyAuth(a, tenantPub, topic, nowSeconds) {
  if (Number(a?.v) !== VERSION) return false;
  if (a.topic !== topic) return false;
  const delta = BigInt(nowSeconds) - BigInt(a.ts);
  if ((delta < 0n ? -delta : delta) > BigInt(MAX_SKEW_SECONDS)) return false;
  return verifyEd25519(tenantPub, authSigningBytes(a), a.sig ?? new Uint8Array(0));
}

// ---------------------------------------------------------------------------
// the NodeID seam, re-exported
// ---------------------------------------------------------------------------
// `proto.js` owns these because `identity.js` needed them before this file
// existed. Re-exported so a caller working in the wire format does not have
// to know which of two modules a given helper landed in.

export { nodeIdFromPub, pubFromNodeId };
