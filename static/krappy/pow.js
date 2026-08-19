// KRAP-1 §7.7 step 5: proof of work, for the browser host.
//
// The counterpart of `krappy-proto/pow.go`. One SHA-256 for the board and up
// to 2^difficulty for the sender — which is why §7.7 puts this check before
// any signature verification, and why flooding a board costs the flooder
// rather than the operator.
//
//   digest = SHA-256(topic || ct || u64be(ts) || nonce || pow)
//
// `nonce` is the drop's 16-byte replay nonce; `pow` is the 8-byte big-endian
// counter the sender varies. The digest covers the sealed body and both
// nonces, so work cannot be computed once and reused across drops, topics or
// timestamps.
//
// The search core is pure and synchronous (`powPreimage`, `leadingZeroBits`)
// so it is testable without a clock or a crypto context. The hashing is not:
// `crypto.subtle.digest` is the only SHA-256 a browser hands out and it is
// asynchronous, so `mintPoW` hashes a BATCH of counters concurrently and
// yields between batches. That keeps the main thread responsive and gives an
// AbortSignal somewhere to land — a host that wedges its UI while minting has
// turned a rate limit into a hang.
//
// COST, measured rather than guessed — node 22 on one developer laptop,
// ~430-byte preimage, default batch of 256:
//
//   difficulty 12    23 ms
//   difficulty 16   163 ms
//   difficulty 20  6500 ms
//
// So the reference board's default of 8 is free, 12 (the shipped example
// config) is imperceptible, 16 is a visible pause, and 20 is the point where
// a UI must show progress. Each doubling of difficulty doubles the work;
// `MAX_POW_BITS` (32) is roughly a week and is not reachable in a browser at
// all. `mintPoW` therefore REFUSES above the cap rather than spinning, exactly
// as `pow.go` does — a client that meets an absurd `/policy` should say the
// board is unusable rather than appear to hang. A real browser will be
// slower than these figures, not faster; nothing here has run in one.

import { concat, u64be, utf8Bytes } from './proto.js';

/** Width of the counter a sender varies. Must match Go's `PoWNonceLen`. */
export const POW_NONCE_LEN = 8;

/**
 * Difficulty ceiling, mirroring Go's `MaxPoWBits`.
 *
 * An operator who can set unbounded difficulty can deny service while
 * appearing to be up, so a client refuses an absurd value instead of spinning.
 */
export const MAX_POW_BITS = 32;

/**
 * The exact bytes hashed. Pure, synchronous, and the only place the field
 * order is written down on this side of the wire.
 *
 * @param {string} topic KRAP-1 §7.3 topic the drop is posted to
 * @param {Uint8Array} ct the sealed postcard (§7.6)
 * @param {number|bigint} ts unix seconds
 * @param {Uint8Array} nonce the drop's replay nonce
 * @param {Uint8Array} pow the 8-byte counter
 * @returns {Uint8Array}
 */
export function powPreimage(topic, ct, ts, nonce, pow) {
  return concat(utf8Bytes(topic), ct, u64be(ts), nonce, pow);
}

/** Count zero bits at the front of a digest. Pure, synchronous. */
export function leadingZeroBits(bytes) {
  let n = 0;
  for (const b of bytes) {
    if (b !== 0) return n + Math.clz32(b) - 24;
    n += 8;
  }
  return n;
}

/** @returns {Promise<Uint8Array>} the §7.7 step 5 digest */
export async function powDigest(topic, ct, ts, nonce, pow) {
  return new Uint8Array(await crypto.subtle.digest('SHA-256', powPreimage(topic, ct, ts, nonce, pow)));
}

/**
 * Verify a proof of work, the way a board does.
 *
 * Difficulty 0 passes trivially — that is Go's behaviour, and a board that
 * advertises `minPoWBits: 0` has chosen not to charge for drops.
 *
 * @returns {Promise<boolean>}
 */
export async function checkPoW({ topic, ct, ts, nonce, pow, difficulty }) {
  if (difficulty === 0) return true;
  if (!(pow instanceof Uint8Array) || pow.length !== POW_NONCE_LEN) return false;
  const d = await powDigest(topic, ct, ts, nonce, pow);
  return leadingZeroBits(d) >= difficulty;
}

/** Write a uint64 counter, held as a 32-bit hi/lo pair to stay off BigInt. */
function writeCounter(out, hi, lo) {
  out[0] = (hi >>> 24) & 0xff;
  out[1] = (hi >>> 16) & 0xff;
  out[2] = (hi >>> 8) & 0xff;
  out[3] = hi & 0xff;
  out[4] = (lo >>> 24) & 0xff;
  out[5] = (lo >>> 16) & 0xff;
  out[6] = (lo >>> 8) & 0xff;
  out[7] = lo & 0xff;
}

const randomStart = () => {
  const b = new Uint8Array(POW_NONCE_LEN);
  crypto.getRandomValues(b);
  let v = 0n;
  for (const x of b) v = (v << 8n) | BigInt(x);
  return v;
};

/**
 * Search for a counter whose digest has `difficulty` leading zero bits.
 *
 * Semantics match Go's `MintPoWFrom` exactly: counters are tried in
 * increasing big-endian uint64 order from `start`, and the FIRST satisfying
 * counter wins. Concurrency does not change that — a batch is scanned in
 * index order — so JS and Go mint byte-identical work from the same start,
 * which is what `web/test-wire.mjs` asserts against the golden vector.
 *
 * The default start is random rather than zero, as in Go: two clients minting
 * against the same board at the same difficulty must not walk the same search
 * path, or their work (and the drops built on it) collide.
 *
 * @param {object} args
 * @param {string} args.topic
 * @param {Uint8Array} args.ct
 * @param {number|bigint} args.ts
 * @param {Uint8Array} args.nonce
 * @param {number} args.difficulty leading zero bits required
 * @param {bigint} [args.start] explicit starting counter; tests want this
 * @param {number} [args.batch] counters hashed concurrently per turn
 * @param {AbortSignal} [args.signal]
 * @param {(tried: number) => void} [args.onProgress] called once per batch
 * @returns {Promise<Uint8Array>} the 8-byte counter
 */
export async function mintPoW({
  topic,
  ct,
  ts,
  nonce,
  difficulty,
  start,
  batch = 256,
  signal,
  onProgress,
}) {
  if (!Number.isInteger(difficulty) || difficulty < 0) {
    throw new RangeError(`difficulty must be a non-negative integer, got ${difficulty}`);
  }
  // Zero difficulty returns the all-zero counter, matching Go: the field is
  // still on the wire and still covered by the drop signature.
  if (difficulty === 0) return new Uint8Array(POW_NONCE_LEN);
  if (difficulty > MAX_POW_BITS) {
    throw new RangeError(
      `difficulty ${difficulty} exceeds the ${MAX_POW_BITS}-bit cap: refusing rather than spinning`,
    );
  }

  const from = start === undefined ? randomStart() : BigInt(start) & ((1n << 64n) - 1n);
  let hi = Number((from >> 32n) & 0xffffffffn) >>> 0;
  let lo = Number(from & 0xffffffffn) >>> 0;

  const prefix = concat(utf8Bytes(topic), ct, u64be(ts), nonce);

  // One preimage buffer per batch slot, allocated once and reused. A slot is
  // only ever rewritten after its own digest has been awaited, so no promise
  // can observe a buffer changing under it — and a search at difficulty 20
  // does not allocate a gigabyte of short-lived byte arrays on the way.
  const slots = [];
  for (let i = 0; i < batch; i++) {
    const buf = new Uint8Array(prefix.length + POW_NONCE_LEN);
    buf.set(prefix, 0);
    slots.push({ buf, counter: buf.subarray(prefix.length) });
  }
  const pending = new Array(batch);

  let tried = 0;
  for (;;) {
    signal?.throwIfAborted();

    for (let i = 0; i < batch; i++) {
      writeCounter(slots[i].counter, hi, lo);
      pending[i] = crypto.subtle.digest('SHA-256', slots[i].buf);
      lo = (lo + 1) >>> 0;
      if (lo === 0) hi = (hi + 1) >>> 0;
    }
    const digests = await Promise.all(pending);
    tried += batch;

    // Index order, so the winner is the LOWEST counter in the batch — the
    // same one Go's sequential loop would have stopped on. Concurrency here
    // must not change which counter is minted, or JS and Go stop agreeing.
    for (let i = 0; i < digests.length; i++) {
      if (leadingZeroBits(new Uint8Array(digests[i])) >= difficulty) return slots[i].counter.slice();
    }
    onProgress?.(tried);
  }
}
