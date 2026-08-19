// Turning a NodeID into a Dial Record.
//
// This is the first thing a browser does and the last piece `web/` was
// missing: `wire.js` could parse a published record and `board.js` could drop
// a postcard to the node it names, but nothing fetched the bytes, so the only
// way to dial was to already hold the record — which is not a thing a user
// has.
//
// The security story here is short, and it is the reason KRAP-1 inverts the
// CA model. A Dial Record is self-authenticating: it carries the ed25519 key
// its NodeID is derived from and a signature over its own canonical CBOR. So
// the host serving it is **untrusted**, and so is the transport. We do not
// care who answered the request; we care that the bytes verify as the node we
// asked for. That check is `expectId`, and it is not optional — without it a
// static host could serve any node's record for any name and the caller would
// dial the wrong machine while believing the signature had protected it.
//
// What this deliberately does NOT do:
//
//   - No DNS TXT (§4.3's other publication target). That is #27, and it needs
//     DoH in a browser, which is a different dependency argument.
//   - No caching. `host-core.js` already owns a record cache behind
//     `putRecord`, and a second one here would be the copy that goes stale.
//     `resolveRecord` fetches; the host decides what to keep.

import { parsePublishedRecord, WireError } from './wire.js';

/** §4.3: where a record lives under a site root. Mirrors Go's `WellKnownPath`. */
export const wellKnownPath = (nodeId) => `/.well-known/krappy/${nodeId}`;

/**
 * A record could not be resolved.
 *
 * `kind` separates the three things a caller can actually do something about,
 * because "there is no record" and "there is a record and it is somebody
 * else's" are very different problems and a single failure type hides that:
 *
 *   - `transport` — the request never produced a response. Retryable.
 *   - `http`      — the host answered, and not with a record. `status` is set.
 *                   404 means this node publishes nowhere here.
 *   - `untrusted` — bytes arrived and did not verify as the node asked for:
 *                   bad signature, wrong NodeID, expired, malformed. NOT
 *                   retryable, and the interesting one — it is what a
 *                   compromised or confused record host looks like from here.
 */
export class ResolveError extends Error {
  constructor(kind, message, { nodeId, url, status, cause } = {}) {
    super(message, cause === undefined ? undefined : { cause });
    this.name = 'ResolveError';
    this.kind = kind;
    this.nodeId = nodeId;
    this.url = url;
    this.status = status;
  }

  get retryable() {
    if (this.kind === 'transport') return true;
    if (this.kind !== 'http') return false;
    return this.status === 429 || this.status >= 500;
  }
}

const isAbort = (e) => e?.name === 'AbortError' || e?.name === 'TimeoutError';

/** Records are small (§4.1 says ~300–500 bytes). Anything larger is not one. */
const MAX_RECORD_BYTES = 8192;

/**
 * Fetch and verify the Dial Record for `nodeId`, published under `origin`.
 *
 * @param {string} nodeId base36 NodeID — the identity you mean to reach
 * @param {string} origin site root serving `/.well-known/krappy/`
 * @param {object} [opts]
 * @param {number} [opts.now] unix seconds; defaults to the local clock
 * @param {AbortSignal} [opts.signal]
 * @param {typeof fetch} [opts.fetch] injectable for tests
 * @returns {Promise<import('./wire.js').DialRecord & {nodeId: string}>}
 */
export async function resolveRecord(nodeId, origin, { now, signal, fetch: f = fetch } = {}) {
  if (!nodeId) throw new ResolveError('untrusted', 'resolve: no NodeID given');
  const base = String(origin).replace(/\/+$/, '');
  const url = base + wellKnownPath(nodeId);

  let resp;
  try {
    resp = await f(url, { signal, redirect: 'follow', headers: { Accept: 'application/cbor' } });
  } catch (e) {
    if (isAbort(e)) throw new ResolveError('transport', `resolve ${nodeId}: aborted`, { nodeId, url, cause: e });
    throw new ResolveError('transport', `resolve ${nodeId}: ${e.message}`, { nodeId, url, cause: e });
  }

  if (!resp.ok) {
    const hint = resp.status === 404 ? ' — this node publishes no record here' : '';
    throw new ResolveError('http', `resolve ${nodeId}: ${resp.status}${hint}`, {
      nodeId,
      url,
      status: resp.status,
    });
  }

  const buf = await resp.arrayBuffer();
  if (buf.byteLength > MAX_RECORD_BYTES) {
    throw new ResolveError('untrusted', `resolve ${nodeId}: ${buf.byteLength} bytes is not a Dial Record`, {
      nodeId,
      url,
    });
  }

  // `expectId` is load-bearing, not a convenience: it is the whole reason the
  // host serving these bytes does not have to be trusted.
  try {
    return await parsePublishedRecord(new Uint8Array(buf), {
      now: now ?? Math.floor(Date.now() / 1000),
      expectId: nodeId,
    });
  } catch (e) {
    const why = e instanceof WireError ? `${e.code}: ${e.message}` : e.message;
    throw new ResolveError('untrusted', `resolve ${nodeId}: ${why}`, { nodeId, url, cause: e });
  }
}
