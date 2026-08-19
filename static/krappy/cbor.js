// Canonical CBOR for the browser host — RFC 8949 §4.2.1 core deterministic
// encoding, and a deliberately hostile decoder.
//
// This is the JS counterpart of `krappy-proto/cbor.go`, which sits on
// `fxamacker/cbor/v2` with `CoreDetEncOptions()`. The two must produce
// IDENTICAL BYTES, because every signed structure in KRAP-1 is signed over
// this encoding: a signature computed over a non-deterministic encoding
// cannot be reproduced by a verifier that re-encodes, so canonicalization is
// what makes a Dial Record a signed fact rather than a signed guess.
//
// The codec descends from `krappy-proto/testdata/verify-vectors.mjs`, which
// was written from the RFC and is already conformance-checked against Go.
// What changed on the way into a module: no `Buffer` (browsers do not have
// one), no `push(...spread)` (it blows the stack on a large byte string),
// bounds checks on every read, the same nesting/size ceilings the Go decoder
// enforces, and a `Raw` passthrough so a structure can round-trip a nested
// blob it does not understand without corrupting the signature over it.
//
// `verify-vectors.mjs` keeps its own private copy on purpose. It is the
// independent second implementation that catches drift in THIS file; sharing
// code with it would make `make vectors` check that a codec agrees with
// itself. `web/test-wire.mjs` is where this file is held to the golden bytes.
//
// Supported subset, which is all of KRAP-1: unsigned and negative integers,
// byte strings, text strings, arrays, maps, and the three simple values
// (false/true/null). Floats, tags, and indefinite lengths are rejected in
// both directions — the wire format has never used them, and accepting them
// would widen the attack surface of every board and node that parses bytes
// from strangers.

/** Thrown for any malformed, non-canonical, or out-of-policy CBOR. */
export class CborError extends Error {
  constructor(message) {
    super(message);
    this.name = 'CborError';
  }
}

// Ceilings mirroring `krappy-proto/cbor.go`'s DecOptions. A browser parses
// records fetched from static hosts it does not control, so it wants the same
// bounds a board applies to a stranger's drop.
export const LIMITS = {
  maxNestedLevels: 8,
  maxArrayElements: 64,
  maxMapPairs: 64,
};

/**
 * A pre-encoded CBOR item, spliced in verbatim.
 *
 * The counterpart of Go's `cbor.RawMessage`, and it exists for exactly the
 * same reason: KRAP-1 §7.2 mailbox descriptors carry params whose shape
 * differs per kind, and a record must round-trip a descriptor it does not
 * understand without changing the bytes the node's signature covers.
 */
export class Raw {
  /** @param {Uint8Array} bytes a complete, already-canonical CBOR item */
  constructor(bytes) {
    this.bytes = bytes;
  }
}

const textEncoder = new TextEncoder();
// fatal: a text string that is not valid UTF-8 is a decode failure, not a
// string full of replacement characters that would re-encode differently.
const textDecoder = new TextDecoder('utf-8', { fatal: true });

// ---------------------------------------------------------------------------
// encode
// ---------------------------------------------------------------------------

/** Grows geometrically; `push(...spread)` overflows the argument limit. */
class Sink {
  constructor() {
    this.buf = new Uint8Array(256);
    this.len = 0;
  }

  #room(n) {
    if (this.len + n <= this.buf.length) return;
    let cap = this.buf.length * 2;
    while (cap < this.len + n) cap *= 2;
    const next = new Uint8Array(cap);
    next.set(this.buf.subarray(0, this.len));
    this.buf = next;
  }

  byte(b) {
    this.#room(1);
    this.buf[this.len++] = b;
  }

  bytes(b) {
    this.#room(b.length);
    this.buf.set(b, this.len);
    this.len += b.length;
  }

  done() {
    return this.buf.slice(0, this.len);
  }
}

/**
 * Write a major type and its argument in SHORTEST FORM.
 *
 * Shortest form is not an optimization here: RFC 8949 core deterministic
 * requires it, and an encoder that emits `0x1817` where `0x17` would do
 * produces signatures Go rejects.
 */
function writeHead(sink, major, arg) {
  const n = typeof arg === 'bigint' ? arg : BigInt(arg);
  if (n < 0n) throw new CborError('negative argument');
  if (n < 24n) {
    sink.byte((major << 5) | Number(n));
  } else if (n < 0x100n) {
    sink.byte((major << 5) | 24);
    sink.byte(Number(n));
  } else if (n < 0x10000n) {
    sink.byte((major << 5) | 25);
    sink.byte(Number((n >> 8n) & 0xffn));
    sink.byte(Number(n & 0xffn));
  } else if (n < 0x100000000n) {
    sink.byte((major << 5) | 26);
    for (let i = 3; i >= 0; i--) sink.byte(Number((n >> BigInt(i * 8)) & 0xffn));
  } else if (n < 1n << 64n) {
    sink.byte((major << 5) | 27);
    for (let i = 7; i >= 0; i--) sink.byte(Number((n >> BigInt(i * 8)) & 0xffn));
  } else {
    throw new CborError('integer wider than 64 bits');
  }
}

const isPlainObject = (x) => {
  if (x === null || typeof x !== 'object') return false;
  const proto = Object.getPrototypeOf(x);
  return proto === Object.prototype || proto === null;
};

function writeValue(sink, x, depth) {
  if (depth > LIMITS.maxNestedLevels) {
    throw new CborError(`nesting deeper than ${LIMITS.maxNestedLevels}`);
  }

  if (x === null) return sink.byte(0xf6);
  if (x === false) return sink.byte(0xf4);
  if (x === true) return sink.byte(0xf5);

  if (typeof x === 'number') {
    if (!Number.isInteger(x)) throw new CborError(`refusing to encode non-integer ${x}`);
    return writeInteger(sink, BigInt(x));
  }
  if (typeof x === 'bigint') return writeInteger(sink, x);

  if (x instanceof Raw) return sink.bytes(x.bytes);

  if (x instanceof Uint8Array) {
    writeHead(sink, 2, x.length);
    return sink.bytes(x);
  }
  if (ArrayBuffer.isView(x) || x instanceof ArrayBuffer) {
    throw new CborError('pass a Uint8Array, not another view — byte length is ambiguous');
  }

  if (typeof x === 'string') {
    const b = textEncoder.encode(x);
    writeHead(sink, 3, b.length);
    return sink.bytes(b);
  }

  if (Array.isArray(x)) {
    writeHead(sink, 4, x.length);
    for (const item of x) writeValue(sink, item, depth + 1);
    return undefined;
  }

  const entries = x instanceof Map ? [...x.entries()] : isPlainObject(x) ? Object.entries(x) : null;
  if (entries === null) throw new CborError(`cannot encode ${Object.prototype.toString.call(x)}`);
  return writeMap(sink, entries, depth);
}

function writeInteger(sink, n) {
  if (n >= 0n) return writeHead(sink, 0, n);
  // Major 1 encodes -1-n. KRAP-1 never uses it; supported so the codec is
  // RFC-correct rather than quietly wrong for a caller who strays.
  return writeHead(sink, 1, -1n - n);
}

/**
 * Map keys are sorted by their ENCODED bytes: length first, then bytewise.
 *
 * This is RFC 8949 §4.2.1's ordering and the single most likely place for a
 * second implementation to diverge — sorting by the key's *string* value
 * instead looks right and produces different bytes as soon as two keys differ
 * in length (which, in a Dial Record, they always do).
 */
function writeMap(sink, entries, depth) {
  const encoded = entries.map(([k, v]) => {
    const ks = new Sink();
    writeValue(ks, k, depth + 1);
    return [ks.done(), v];
  });
  encoded.sort((a, b) => {
    if (a[0].length !== b[0].length) return a[0].length - b[0].length;
    for (let i = 0; i < a[0].length; i++) {
      if (a[0][i] !== b[0][i]) return a[0][i] - b[0][i];
    }
    return 0;
  });
  for (let i = 1; i < encoded.length; i++) {
    const [p, q] = [encoded[i - 1][0], encoded[i][0]];
    if (p.length === q.length && p.every((byte, j) => byte === q[j])) {
      throw new CborError('duplicate map key');
    }
  }
  writeHead(sink, 5, encoded.length);
  for (const [ke, v] of encoded) {
    sink.bytes(ke);
    writeValue(sink, v, depth + 1);
  }
}

/**
 * Encode a value as RFC 8949 core deterministic CBOR.
 *
 * @param {*} value number | bigint | boolean | null | string | Uint8Array |
 *   Array | Map | plain object | Raw
 * @returns {Uint8Array}
 */
export function encode(value) {
  const sink = new Sink();
  writeValue(sink, value, 1);
  return sink.done();
}

// ---------------------------------------------------------------------------
// decode
// ---------------------------------------------------------------------------

/**
 * Decode one complete CBOR item, rejecting trailing bytes.
 *
 * Maps come back as `Map` rather than objects so that a non-text key, or a key
 * that collides with `__proto__`, cannot be laundered into a JS property.
 *
 * @param {Uint8Array} bytes
 * @param {{maxArrayElements?: number, maxMapPairs?: number, maxNestedLevels?: number}} [limits]
 * @returns {*}
 */
export function decode(bytes, limits = {}) {
  if (!(bytes instanceof Uint8Array)) throw new CborError('decode wants a Uint8Array');
  const lim = { ...LIMITS, ...limits };
  let pos = 0;

  const need = (n) => {
    if (pos + n > bytes.length) throw new CborError('truncated input');
  };

  function readArg(ai) {
    if (ai < 24) return BigInt(ai);
    if (ai === 24) {
      need(1);
      return BigInt(bytes[pos++]);
    }
    if (ai === 25) {
      need(2);
      const v = (BigInt(bytes[pos]) << 8n) | BigInt(bytes[pos + 1]);
      pos += 2;
      return v;
    }
    if (ai === 26) {
      need(4);
      let v = 0n;
      for (let i = 0; i < 4; i++) v = (v << 8n) | BigInt(bytes[pos + i]);
      pos += 4;
      return v;
    }
    if (ai === 27) {
      need(8);
      let v = 0n;
      for (let i = 0; i < 8; i++) v = (v << 8n) | BigInt(bytes[pos + i]);
      pos += 8;
      return v;
    }
    // 28-30 reserved, 31 indefinite. Indefinite lengths allow unbounded
    // streaming and have no canonical form, so they are forbidden outright.
    throw new CborError(`indefinite or reserved additional information (ai=${ai})`);
  }

  /** JS numbers past 2^53 are lossy, and a lossy decode cannot round-trip. */
  const narrow = (v) =>
    v <= BigInt(Number.MAX_SAFE_INTEGER) && v >= BigInt(Number.MIN_SAFE_INTEGER) ? Number(v) : v;

  const asLength = (v) => {
    if (v > BigInt(bytes.length)) throw new CborError('length exceeds input');
    return Number(v);
  };

  function value(depth) {
    if (depth > lim.maxNestedLevels) {
      throw new CborError(`nesting deeper than ${lim.maxNestedLevels}`);
    }
    need(1);
    const b = bytes[pos++];
    const major = b >> 5;
    const ai = b & 0x1f;

    if (major === 7) {
      if (ai === 20) return false;
      if (ai === 21) return true;
      if (ai === 22) return null;
      // 25/26/27 are the float widths; 23 is `undefined`; 24 is a simple
      // value byte. None appear in KRAP-1 and none of them have a use here.
      throw new CborError(`unsupported simple value or float (ai=${ai})`);
    }

    const arg = readArg(ai);

    switch (major) {
      case 0:
        return narrow(arg);
      case 1:
        return narrow(-1n - arg);
      case 2: {
        const n = asLength(arg);
        need(n);
        const out = bytes.slice(pos, pos + n);
        pos += n;
        return out;
      }
      case 3: {
        const n = asLength(arg);
        need(n);
        let out;
        try {
          out = textDecoder.decode(bytes.subarray(pos, pos + n));
        } catch {
          throw new CborError('text string is not valid UTF-8');
        }
        pos += n;
        return out;
      }
      case 4: {
        const n = asLength(arg);
        if (n > lim.maxArrayElements) throw new CborError(`array of ${n} exceeds the cap`);
        const out = [];
        for (let i = 0; i < n; i++) out.push(value(depth + 1));
        return out;
      }
      case 5: {
        const n = asLength(arg);
        if (n > lim.maxMapPairs) throw new CborError(`map of ${n} pairs exceeds the cap`);
        const out = new Map();
        for (let i = 0; i < n; i++) {
          const k = value(depth + 1);
          if (out.has(k)) throw new CborError(`duplicate map key ${String(k)}`);
          out.set(k, value(depth + 1));
        }
        return out;
      }
      default:
        throw new CborError(`unsupported CBOR major type ${major}`);
    }
  }

  const out = value(1);
  if (pos !== bytes.length) throw new CborError(`${bytes.length - pos} trailing bytes`);
  return out;
}

/**
 * Decode, and reject input that was not already in canonical form.
 *
 * The counterpart of Go's `UnmarshalCanonical`, and required for anything a
 * signature covers. Without it a sender could re-encode a validly signed
 * structure into a different but equivalent byte string; a verifier that
 * canonicalizes before checking would then accept bytes it never saw, and two
 * implementations could disagree about whether the same message is authentic.
 *
 * @param {Uint8Array} bytes
 * @returns {*}
 */
export function decodeCanonical(bytes, limits) {
  const value = decode(bytes, limits);
  const round = encode(value);
  if (round.length !== bytes.length || !round.every((b, i) => b === bytes[i])) {
    throw new CborError('input is not canonical CBOR');
  }
  return value;
}
