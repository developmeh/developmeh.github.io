// A KRAP-1 §7.9 board client for the browser host.
//
// The browser half of the postcard path: fetch a board's policy, drop a
// sealed postcard on a node's topic, and — when this host is itself a
// tenant — long-poll a topic for its own mail. It moves bytes to and from a
// board and does nothing else. No WebRTC, no SDP, no candidate gathering
// (that is #46), and no wire format of its own: every signed byte here comes
// from `wire.js`, `pow.js` and `proto.js`, which are held to Go's golden
// vectors by `make web-wire`.
//
// The Go counterpart is `krappy-node/internal/mailbox` plus `dropPostcard` in
// `krappy-node/dial.go`. This mirrors their BEHAVIOUR, not their structure —
// the board API is a wire contract, so agreeing about HTTP is the whole job
// and agreeing about types is not available to us anyway.
//
// Four things this file treats as normal rather than exceptional, because
// §7.7 and §7.9 say a conformant board may do them:
//
//   - `wait` is a hint. A board may answer early, immediately, or with
//     nothing. A client that read that as an error would spin or die against
//     a perfectly conformant serverless board.
//   - `maxWaitSeconds: 0` means the board cannot hold a connection at all,
//     and the client short-polls instead.
//   - A board may add JSON keys and may not send the ones you expect;
//     §7.9.1 says ignore what you do not know and never require an order.
//   - A topic's sequence can go BACKWARDS. See `watch` and #37.
//
// ERRORS ARE THE POINT OF HALF THIS FILE. #40 exists because a board that
// drops its CORS headers on an unintended 500 makes a bug look like the board
// being down, and in a browser those two are the same `TypeError`. A client
// cannot fix that, but it can refuse to blur it: every failure here is a
// `BoardError` that says whether a response was ever produced (`transport`),
// whether the board answered and with what (`http` + `status`), whether the
// answer was not §7.9 (`malformed`), or whether this client declined to send
// at all (`refused`). §7.9.8's remedies are attached as `advice`, because the
// status codes are deliberately coarse and the remedy is not.

import { deriveTopic, epochOf, sealBox, DEFAULT_EPOCH_LEN, nodeIdFromPub } from './proto.js';
import { mintPoW, MAX_POW_BITS } from './pow.js';
import {
  encodePostcard,
  newDropNonce,
  parseGrant,
  signAuth,
  signDrop,
  WireError,
  DROP_NONCE_LEN,
} from './wire.js';

// ---------------------------------------------------------------------------
// base64url, RFC 4648 §5, no padding (§7.9.1)
// ---------------------------------------------------------------------------
// Written out rather than reached for: `atob`/`btoa` are standard base64 and
// exist in a SharedWorker but not in every runtime a host might run under,
// and Node's Buffer is not available in a browser at all. Both directions are
// a dozen lines and neither can drift.

const B64URL = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_';
const B64URL_REV = (() => {
  const t = new Int16Array(128).fill(-1);
  for (let i = 0; i < B64URL.length; i++) t[B64URL.charCodeAt(i)] = i;
  // Tolerate the standard alphabet on the way IN only. §7.9.1 requires
  // base64url and both boards send it, but a decoder that rejects `+`/`/`
  // turns another implementation's minor sin into an unreadable mailbox,
  // while an encoder that emits them would be our own violation.
  t['+'.charCodeAt(0)] = 62;
  t['/'.charCodeAt(0)] = 63;
  return t;
})();

/** @param {Uint8Array} bytes @returns {string} */
export function base64url(bytes) {
  let out = '';
  let i = 0;
  for (; i + 2 < bytes.length; i += 3) {
    const n = (bytes[i] << 16) | (bytes[i + 1] << 8) | bytes[i + 2];
    out += B64URL[(n >>> 18) & 63] + B64URL[(n >>> 12) & 63] + B64URL[(n >>> 6) & 63] + B64URL[n & 63];
  }
  const left = bytes.length - i;
  if (left === 1) {
    const n = bytes[i] << 16;
    out += B64URL[(n >>> 18) & 63] + B64URL[(n >>> 12) & 63];
  } else if (left === 2) {
    const n = (bytes[i] << 16) | (bytes[i + 1] << 8);
    out += B64URL[(n >>> 18) & 63] + B64URL[(n >>> 12) & 63] + B64URL[(n >>> 6) & 63];
  }
  return out;
}

/** @param {string} s @returns {Uint8Array|null} null when it is not base64url */
export function unbase64url(s) {
  if (typeof s !== 'string') return null;
  const t = s.replace(/=+$/, '');
  const full = t.length >> 2;
  const rem = t.length & 3;
  if (rem === 1) return null;
  const out = new Uint8Array(full * 3 + (rem === 2 ? 1 : rem === 3 ? 2 : 0));
  let o = 0;
  let acc = 0;
  let bits = 0;
  for (let i = 0; i < t.length; i++) {
    const c = t.charCodeAt(i);
    const v = c < 128 ? B64URL_REV[c] : -1;
    if (v < 0) return null;
    acc = (acc << 6) | v;
    bits += 6;
    if (bits >= 8) {
      bits -= 8;
      out[o++] = (acc >>> bits) & 0xff;
    }
  }
  return out;
}

// ---------------------------------------------------------------------------
// origins (§7.5 normal form)
// ---------------------------------------------------------------------------

/**
 * A board origin in §7.5 normal form: lowercase scheme, lowercase host, no
 * trailing slash, and no port when it is the default for the scheme.
 *
 * This is the string a grant's `brd` must equal, and §7.5 spells it out
 * because it is compared by exact string equality — so `https://board.example`
 * and `https://Board.example/` are otherwise three different boards and the
 * whole grant design fails silently with a uniform 403.
 *
 * Note the asymmetry with the boards: `krappy-bb-cf` normalises its configured
 * origin, `krappy-bb` does not (its README's finding 6). Comparing normalised
 * forms here is therefore the FORGIVING choice — a board doing an exact
 * comparison can still reject a grant this client was willing to spend work
 * on, which is what §7.9.8's `403` advice covers.
 *
 * @param {string} origin
 * @returns {string}
 * @throws {BoardError} kind `refused` when it is not a URL at all
 */
export function normalizeOrigin(origin) {
  let u;
  try {
    u = new URL(String(origin));
  } catch {
    throw new BoardError('refused', `not a board origin: ${String(origin)}`, { advice: 'fix-config' });
  }
  const scheme = u.protocol.toLowerCase();
  const host = u.hostname.toLowerCase();
  const port = u.port;
  const isDefault =
    port === '' || (scheme === 'https:' && port === '443') || (scheme === 'http:' && port === '80');
  return `${scheme}//${host}${isDefault ? '' : ':' + port}`;
}

// ---------------------------------------------------------------------------
// errors
// ---------------------------------------------------------------------------

/**
 * §7.9.8's remedies, keyed by status. A client "MUST key on the status code
 * and MUST NOT parse the reason", so this is the only mapping there is.
 */
const ADVICE = {
  400: 'fix-encoder',
  401: 'reauthenticate',
  403: 'refetch-record',
  404: 'topic-unknown',
  409: 'retry-with-fresh-nonce',
  413: 'shrink-body',
  429: 'refetch-policy-and-back-off',
  503: 'try-another-board',
};

/**
 * Everything this client throws.
 *
 * `kind` is the distinction #40 says a caller needs and cannot otherwise make:
 *
 *   `transport` — no response was produced. In a browser this is also what a
 *                 CORS rejection and a board's un-headered 500 look like, and
 *                 the platform does not let anyone tell them apart; saying
 *                 "the board did not answer" is the honest ceiling.
 *   `http`      — the board answered. `status` is set, `advice` is §7.9.8's.
 *   `malformed` — it answered, and the answer is not the document §7.9
 *                 describes. A board bug, not a network one.
 *   `refused`   — this client declined to send. A grant that will not verify,
 *                 a grant for another board, `rate: 0`, work above the PoW
 *                 cap, a body over the ceiling. Cheaper to catch here than to
 *                 mint proof of work for.
 *   `aborted`   — the caller's AbortSignal fired.
 *
 * @property {'transport'|'http'|'malformed'|'refused'|'aborted'} kind
 * @property {number} [status]
 * @property {string} [advice]
 * @property {number} [retryAfterSeconds]
 */
export class BoardError extends Error {
  constructor(kind, message, { origin, url, method, status, advice, retryAfterSeconds, cause } = {}) {
    super(message, cause === undefined ? undefined : { cause });
    this.name = 'BoardError';
    this.kind = kind;
    this.origin = origin;
    this.url = url;
    this.method = method;
    this.status = status;
    this.advice = advice ?? (status !== undefined ? ADVICE[status] : undefined);
    this.retryAfterSeconds = retryAfterSeconds;
  }

  /** True when retrying the same request later could plausibly work. */
  get retryable() {
    if (this.kind === 'transport') return true;
    if (this.kind !== 'http') return false;
    return this.status === 409 || this.status === 429 || this.status >= 500;
  }
}

const isAbort = (e) => e?.name === 'AbortError' || e?.name === 'TimeoutError';

/** `Retry-After` as seconds. Accepts the integer form and the HTTP-date form. */
function retryAfterSeconds(header, nowSeconds) {
  if (!header) return undefined;
  const n = Number(header);
  if (Number.isFinite(n) && n >= 0) return Math.ceil(n);
  const when = Date.parse(header);
  if (Number.isNaN(when)) return undefined;
  return Math.max(0, Math.ceil(when / 1000 - nowSeconds));
}

// ---------------------------------------------------------------------------
// policy (§7.9.2)
// ---------------------------------------------------------------------------

/**
 * A board's `/policy`, with the defaults a client falls back on when a board
 * omits a field.
 *
 * The raw document is kept: §7.9.1 says boards MAY add keys, and a client that
 * discarded them would make a board's own extensions unreadable to the host
 * embedding it.
 */
export class Policy {
  constructor(doc, origin) {
    this.raw = doc ?? {};
    const num = (k, fallback) => {
      const v = this.raw[k];
      return typeof v === 'number' && Number.isFinite(v) && v >= 0 ? Math.floor(v) : fallback;
    };
    this.version = num('version', 1);
    // A board that omits `origin` is not conformant, but falling back to the
    // origin we dialled is strictly better than failing every grant check.
    this.origin = typeof this.raw.origin === 'string' && this.raw.origin ? this.raw.origin : origin;
    this.epochLenSeconds = num('epochLenSeconds', DEFAULT_EPOCH_LEN) || DEFAULT_EPOCH_LEN;
    this.maxBodyBytes = num('maxBodyBytes', 4096);
    this.minPoWBits = num('minPoWBits', 0);
    this.dropTTLSeconds = num('dropTTLSeconds', 600);
    this.maxWaitSeconds = num('maxWaitSeconds', 0);
    this.maxSkewSeconds = num('maxSkewSeconds', 120);
    this.tenancy = typeof this.raw.tenancy === 'string' ? this.raw.tenancy : 'allowlist';
    this.acceptingRegistrations = this.raw.acceptingRegistrations === true;
  }

  /**
   * Clamp a requested long-poll to what the board will actually hold, in
   * INTEGER SECONDS (§7.9.4 — the Go duration string that used to go on the
   * wire cost the second board a duration parser written in JavaScript).
   * `0` out means the board cannot hold connections and the caller must
   * short-poll.
   */
  clampWait(requestedSeconds) {
    if (this.maxWaitSeconds <= 0) return 0;
    const want = Number(requestedSeconds);
    if (!Number.isFinite(want) || want <= 0) return this.maxWaitSeconds;
    return Math.min(Math.floor(want), this.maxWaitSeconds);
  }

  /**
   * How long a cursor may sit still before `watch` assumes it points at swept
   * storage and rewinds (#37).
   *
   * Derived from `dropTTLSeconds`, because that is exactly how long a drop can
   * exist: a cursor that has not moved for longer than the TTL is pointing at
   * nothing, so rewinding costs nothing. Clamped so a board advertising
   * nonsense can make the client neither spin nor go blind. Same reasoning and
   * same bounds as `krappy-node/internal/watch`'s `rewindAfter`.
   */
  rewindAfterSeconds() {
    const ttl = this.dropTTLSeconds > 0 ? this.dropTTLSeconds : 600;
    return Math.min(Math.max(ttl, 60), 3600);
  }
}

// ---------------------------------------------------------------------------
// the client
// ---------------------------------------------------------------------------

const sleep = (seconds, signal) =>
  new Promise((resolve, reject) => {
    if (signal?.aborted) return reject(signal.reason ?? new DOMException('aborted', 'AbortError'));
    const t = setTimeout(done, Math.max(0, seconds * 1000));
    function done() {
      signal?.removeEventListener('abort', onAbort);
      resolve();
    }
    function onAbort() {
      clearTimeout(t);
      reject(signal.reason ?? new DOMException('aborted', 'AbortError'));
    }
    signal?.addEventListener('abort', onAbort, { once: true });
  });

/** A bounded FIFO of digests, so a rewind does not re-deliver a whole topic. */
class SeenSet {
  #order = [];
  #set = new Set();
  constructor(limit = 256) {
    this.limit = limit;
  }
  async add(bytes) {
    const d = new Uint8Array(await crypto.subtle.digest('SHA-256', bytes));
    let key = '';
    for (let i = 0; i < 16; i++) key += d[i].toString(16).padStart(2, '0');
    if (this.#set.has(key)) return false;
    this.#set.add(key);
    this.#order.push(key);
    if (this.#order.length > this.limit) this.#set.delete(this.#order.shift());
    return true;
  }
}

/**
 * One board.
 *
 * @param {string} origin the board's origin; normalised per §7.5
 * @param {object} [opts]
 * @param {typeof globalThis.fetch} [opts.fetch]
 * @param {() => number} [opts.now] unix SECONDS
 * @param {number} [opts.policyTTLSeconds] how long a fetched `/policy` is reused
 */
export class BoardClient {
  #fetch;
  #now;
  #policy = null;
  #policyAt = 0;
  #policyInFlight = null;

  constructor(origin, { fetch: fetchImpl, now, policyTTLSeconds = 300 } = {}) {
    this.origin = normalizeOrigin(origin);
    this.policyTTLSeconds = policyTTLSeconds;
    this.#now = now ?? (() => Math.floor(Date.now() / 1000));
    const f = fetchImpl ?? globalThis.fetch;
    if (typeof f !== 'function') {
      throw new BoardError('refused', 'no fetch available; pass one', { advice: 'fix-config' });
    }
    // Bound so `globalThis.fetch` keeps its receiver in every runtime.
    this.#fetch = fetchImpl ? fetchImpl : f.bind(globalThis);
  }

  /** Unix seconds, from the injected clock. */
  now() {
    return this.#now();
  }

  // -- the one place HTTP happens -----------------------------------------

  /**
   * @returns {Promise<{status: number, headers: Headers, text: string}>}
   * @throws {BoardError} `transport` if no response was produced, `http` for
   *   any status not in `expect`, `aborted` for the caller's signal.
   */
  async #request(method, path, { headers, body, signal, expect } = {}) {
    const url = this.origin + path;
    let res;
    try {
      res = await this.#fetch(url, {
        method,
        headers,
        body,
        signal,
        // A board holds no cookies, no sessions and no per-origin state
        // (§7.7), and MUST NOT decide anything on ambient credentials. Not
        // sending any is this side of that bargain.
        credentials: 'omit',
        // `/policy` is the document a client caches; we cache it ourselves,
        // with a TTL, and an HTTP cache underneath would make that a lie.
        cache: 'no-store',
        redirect: 'follow',
      });
    } catch (e) {
      if (isAbort(e) || signal?.aborted) {
        throw new BoardError('aborted', `${method} ${url}: aborted`, {
          origin: this.origin,
          url,
          method,
          cause: e,
        });
      }
      throw new BoardError(
        'transport',
        `${method} ${url}: no response — the board is unreachable, or it answered ` +
          'without the CORS headers §7.7 requires (a browser cannot tell those apart; see #40)',
        { origin: this.origin, url, method, cause: e },
      );
    }

    let text = '';
    try {
      text = await res.text();
    } catch (e) {
      // Headers arrived, the body did not. That is still a response.
      throw new BoardError('transport', `${method} ${url}: response body truncated`, {
        origin: this.origin,
        url,
        method,
        status: res.status,
        cause: e,
      });
    }

    if (expect && !expect.includes(res.status)) {
      throw new BoardError(
        'http',
        // The body is advisory and boards disagree about its shape (§7.9.8),
        // so it is quoted for a human and never parsed.
        `${method} ${url}: ${res.status}${text ? ` — ${text.slice(0, 200).trim()}` : ''}`,
        {
          origin: this.origin,
          url,
          method,
          status: res.status,
          retryAfterSeconds: retryAfterSeconds(res.headers.get('retry-after'), this.#now()),
        },
      );
    }
    return { status: res.status, headers: res.headers, text };
  }

  #json(where, { status, text }, url, method) {
    try {
      const doc = JSON.parse(text);
      if (doc === null || typeof doc !== 'object' || Array.isArray(doc)) {
        throw new Error('not a JSON object');
      }
      return doc;
    } catch (e) {
      throw new BoardError('malformed', `${where}: ${e.message}`, {
        origin: this.origin,
        url,
        method,
        status,
        cause: e,
      });
    }
  }

  // -- §7.9.2 --------------------------------------------------------------

  /**
   * `GET /policy` — the document that makes operators interchangeable.
   *
   * Cached for `policyTTLSeconds`, and concurrent callers share one request.
   * A client MUST NOT take difficulty, size or epoch length from a grant
   * instead (§7.9.2): the grant has not been verified when the board's step 5
   * runs, so work minted for the grant's number can simply be rejected.
   *
   * @returns {Promise<Policy>}
   */
  async policy({ refresh = false, signal } = {}) {
    const fresh = this.#policy && this.#now() - this.#policyAt < this.policyTTLSeconds;
    if (!refresh && fresh) return this.#policy;
    if (this.#policyInFlight) return this.#policyInFlight;

    this.#policyInFlight = (async () => {
      const res = await this.#request('GET', '/policy', { signal, expect: [200] });
      const doc = this.#json('board policy', res, this.origin + '/policy', 'GET');
      const pol = new Policy(doc, this.origin);
      this.#policy = pol;
      this.#policyAt = this.#now();
      return pol;
    })();
    try {
      return await this.#policyInFlight;
    } finally {
      this.#policyInFlight = null;
    }
  }

  /** `GET /healthz` (§7.9.7). Returns the board's JSON, whatever it says. */
  async healthz({ signal } = {}) {
    const res = await this.#request('GET', '/healthz', { signal, expect: [200] });
    return this.#json('board healthz', res, this.origin + '/healthz', 'GET');
  }

  // -- §7.3 ----------------------------------------------------------------

  /**
   * The `{prev, cur, next}` topic window for a node key, at this board's
   * advertised epoch length.
   *
   * A board MUST accept all three (§7.3), which is what absorbs clock skew and
   * an epoch rollover landing between minting and posting.
   *
   * @param {Uint8Array} nodePub the node's ed25519 key — a record's `id`
   * @returns {Promise<{prev: string, cur: string, next: string, epoch: number, epochLenSeconds: number}>}
   */
  async topicWindow(nodePub, { at, signal } = {}) {
    const pol = await this.policy({ signal });
    const epoch = epochOf(at ?? this.#now(), pol.epochLenSeconds);
    const [prev, cur, next] = await Promise.all([
      deriveTopic(nodePub, epoch - 1),
      deriveTopic(nodePub, epoch),
      deriveTopic(nodePub, epoch + 1),
    ]);
    return { prev, cur, next, epoch, epochLenSeconds: pol.epochLenSeconds };
  }

  // -- §7.9.3 --------------------------------------------------------------

  /**
   * `POST /mbx/{topic}` with bytes the caller already built.
   *
   * The escape hatch under `drop`, and the only way to post something `drop`
   * would refuse to build — which is exactly what a test proving the board
   * rejects bad work needs.
   *
   * @returns {Promise<number>} the `seq` receipt (§7.9.3)
   */
  async postDropBytes(topic, bytes, { signal } = {}) {
    const path = `/mbx/${encodeURIComponent(topic)}`;
    const res = await this.#request('POST', path, {
      headers: { 'content-type': 'application/cbor' },
      body: bytes,
      signal,
      expect: [201],
    });
    const doc = this.#json('board drop receipt', res, this.origin + path, 'POST');
    const seq = doc.seq;
    if (typeof seq !== 'number' || !Number.isFinite(seq)) {
      throw new BoardError('malformed', `board drop receipt: no numeric \`seq\` (got ${JSON.stringify(seq)})`, {
        origin: this.origin,
        url: this.origin + path,
        method: 'POST',
        status: res.status,
      });
    }
    return seq;
  }

  /**
   * §7.6 and §7.7 from the client's side: verify the grant, derive the topic,
   * seal, mint the work, sign, POST.
   *
   * The same sequence as `krappy-node/dial.go:dropPostcard`, in the same
   * order, and the order is not incidental:
   *
   *   - the grant is verified BEFORE any work is minted, because a grant that
   *     will not verify is rejected at the board's step 8 and everything spent
   *     getting there is wasted;
   *   - the difficulty is the HIGHER of `/policy.minPoWBits` and the grant's
   *     `lim.pow`. The board checks its own floor at step 5 and the grant's
   *     raise at step 9, so minting for either alone can get the work spent
   *     and then rejected;
   *   - the body is measured against `maxBodyBytes` and `lim.size` before it
   *     is sent, because a 413 is a round trip that says what a subtraction
   *     already knew.
   *
   * @param {object} args
   * @param {object} [args.record] a verified Dial Record from `wire.parseRecord`
   * @param {Uint8Array} [args.grant] the node's grant, if not taking a record
   * @param {Uint8Array} [args.recipientEnc] the node's X25519 `enc` key
   * @param {Uint8Array} [args.tenantPub] the node's ed25519 key
   * @param {string} [args.to] destination NodeID (base36, 50 chars)
   * @param {{signPub: Uint8Array, sign: Function}} args.keystore this host's identity
   * @param {Array<{ip: string, port: number}>|Array<[string, number]>} args.cand
   * @param {{ufrag: string, pwd: string}} args.ice
   * @param {Uint8Array} args.dtls this host's DTLS fingerprint
   * @param {string} [args.topic] override the derived topic (the §7.3 window)
   * @param {number} [args.ts] unix seconds; the seal's AAD and the drop's `ts`
   * @param {AbortSignal} [args.signal] aborts the PoW search too
   * @param {(tried: number) => void} [args.onProgress]
   * @returns {Promise<{seq: number, topic: string, ts: number, difficulty: number, bytes: Uint8Array}>}
   */
  async drop({
    record,
    grant,
    recipientEnc,
    tenantPub,
    to,
    keystore,
    cand,
    ice,
    dtls,
    topic,
    ts,
    signal,
    onProgress,
  }) {
    const grantBytes = grant ?? record?.grant;
    const enc = recipientEnc ?? record?.enc;
    const issuer = tenantPub ?? record?.id;
    const dest = to ?? record?.nodeId ?? (issuer ? nodeIdFromPub(issuer) : undefined);
    if (!(grantBytes instanceof Uint8Array)) {
      throw new BoardError('refused', 'drop: no grant — the record must carry one (§4.1)', {
        origin: this.origin,
        advice: 'refetch-record',
      });
    }
    if (!(enc instanceof Uint8Array) || !(issuer instanceof Uint8Array) || typeof dest !== 'string') {
      throw new BoardError('refused', 'drop: need the node\'s `enc` key, `id` and NodeID', {
        origin: this.origin,
        advice: 'refetch-record',
      });
    }

    const pol = await this.policy({ signal });
    const now = ts ?? this.#now();

    // Grant first: §7.9.2's warning in reverse. The board will check this at
    // step 8 and the cost of finding out there is one full proof of work.
    let g;
    try {
      g = await parseGrant(grantBytes, { expectIss: issuer, now });
    } catch (e) {
      if (!(e instanceof WireError)) throw e;
      throw new BoardError('refused', `drop: the record's grant is not usable: ${e.message}`, {
        origin: this.origin,
        advice: 'refetch-record',
        cause: e,
      });
    }

    const brd = normalizeOrigin(g.brd);
    const mine = normalizeOrigin(pol.origin);
    if (brd !== mine) {
      throw new BoardError(
        'refused',
        `drop: the grant is minted for ${brd}, this board serves ${mine} — ` +
          '§7.5 compares `brd` by exact string equality, so spending work here would buy a 403',
        { origin: this.origin, advice: 'try-another-board' },
      );
    }
    if (Number(g.lim.rate) === 0) {
      // §7.5's trap, and the reason a board MUST answer 429 for it: a node
      // that omits `lim.rate` publishes a record that verifies, parses, looks
      // healthy, and rejects every dial.
      throw new BoardError(
        'refused',
        'drop: the grant carries `lim.rate: 0`, which permits no drops at all (§7.5) — ' +
          'the node published an undialable record',
        { origin: this.origin, advice: 'refetch-record' },
      );
    }

    const dropTopic = topic ?? (await deriveTopic(issuer, epochOf(now, pol.epochLenSeconds)));

    const plaintext = encodePostcard({
      to: dest,
      from: keystore.signPub,
      cand,
      ice,
      dtls,
      ts: now,
      nonce: newDropNonce(),
    });
    const ct = await sealBox({ recipientPub: enc, topic: dropTopic, ts: now, plaintext });

    const difficulty = Math.max(pol.minPoWBits, Number(g.lim.pow));
    if (difficulty > MAX_POW_BITS) {
      throw new BoardError(
        'refused',
        `drop: this board and grant want ${difficulty} bits of work, above the ${MAX_POW_BITS}-bit cap — ` +
          'refusing rather than appearing to hang',
        { origin: this.origin, advice: 'try-another-board' },
      );
    }
    const nonce = newDropNonce();
    let pow;
    try {
      pow = await mintPoW({ topic: dropTopic, ct, ts: now, nonce, difficulty, signal, onProgress });
    } catch (e) {
      if (isAbort(e) || signal?.aborted) {
        throw new BoardError('aborted', 'drop: proof of work aborted', { origin: this.origin, cause: e });
      }
      throw e;
    }

    const { bytes } = await signDrop({ grant: grantBytes, pow, ct, ts: now, nonce }, keystore);

    if (pol.maxBodyBytes > 0 && bytes.length > pol.maxBodyBytes) {
      throw new BoardError(
        'refused',
        `drop: ${bytes.length} bytes exceeds the board's maxBodyBytes of ${pol.maxBodyBytes}`,
        { origin: this.origin, advice: 'shrink-body' },
      );
    }
    const limSize = Number(g.lim.size);
    if (limSize > 0 && bytes.length > limSize) {
      throw new BoardError(
        'refused',
        `drop: ${bytes.length} bytes exceeds the grant's lim.size of ${limSize}`,
        { origin: this.origin, advice: 'shrink-body' },
      );
    }

    const seq = await this.postDropBytes(dropTopic, bytes, { signal });
    return { seq, topic: dropTopic, ts: now, difficulty, bytes };
  }

  // -- §7.9.4 / §7.9.5 -----------------------------------------------------

  /**
   * One authenticated read of one topic — `GET /mbx/{topic}?since=&wait=`.
   *
   * TENANT ONLY: the `Krappy-Auth` header is signed by the key that owns the
   * topic, which is the only thing stopping any party draining a node's
   * mailbox (§7.7). The token is minted fresh per request, bound to this
   * topic, and never reused (§7.9.5).
   *
   * The cursor is NOT clamped to `since`. See `watch` and #37: a board that
   * renumbered a swept topic returns a lower `next`, and a client that treats
   * its cursor as a ratchet goes silently blind. This layer reports what it
   * saw and lets the caller decide.
   *
   * @param {string} topic
   * @param {object} args
   * @param {{signPub: Uint8Array, sign: Function}} args.keystore the TENANT key
   * @param {number} [args.since] 0-based cursor; items with `seq >` come back
   * @param {number} [args.wait] requested long-poll seconds; clamped by policy
   * @returns {Promise<{items: Array<{seq: number, ts: number, body: Uint8Array}>, next: number, waitSeconds: number, malformedItems: number}>}
   */
  async read(topic, { keystore, since = 0, wait, signal } = {}) {
    const pol = await this.policy({ signal });
    const waitSeconds = pol.clampWait(wait);
    const { bytes } = await signAuth({ topic, ts: this.#now() }, keystore);
    const path =
      `/mbx/${encodeURIComponent(topic)}` +
      `?since=${encodeURIComponent(String(since))}&wait=${encodeURIComponent(String(waitSeconds))}`;
    const res = await this.#request('GET', path, {
      headers: { 'krappy-auth': base64url(bytes) },
      signal,
      expect: [200],
    });
    const doc = this.#json('board read', res, this.origin + path, 'GET');

    const rows = Array.isArray(doc.items) ? doc.items : [];
    const items = [];
    let malformedItems = 0;
    for (const row of rows) {
      const body = unbase64url(row?.body);
      if (!body || typeof row.seq !== 'number') {
        // A board that cannot base64 its own storage is broken, but one bad
        // row is not a reason to stop reading the mailbox.
        malformedItems++;
        continue;
      }
      items.push({ seq: row.seq, ts: typeof row.ts === 'number' ? row.ts : 0, body });
    }
    // §7.9.4: `next` is the highest seq returned, or the `since` we sent when
    // items is empty. Recomputing when a board omits it keeps a client that
    // never reasons about sequence arithmetic from having to start.
    let next = typeof doc.next === 'number' && Number.isFinite(doc.next) ? doc.next : undefined;
    if (next === undefined) next = items.length ? Math.max(...items.map((i) => i.seq)) : since;
    return { items, next, waitSeconds, malformedItems };
  }

  /**
   * Long-poll a topic forever, yielding each stored drop once.
   *
   * An async generator rather than a callback so that back-pressure is the
   * caller's: opening a seal and punching takes seconds, and a `for await`
   * that has not come back yet is a mailbox that is not being drained.
   *
   * THE #37 RESILIENCE, and why it is here rather than only on the board.
   * `krappy-bb` renumbers a topic's sequences from 1 when the topic is swept
   * empty, and the board filters reads on `seq > since` — so a client holding
   * cursor 3 against a renumbered topic is not re-reading, it is BLIND, and
   * nothing on either side logs anything. A browser will meet boards that
   * have not been fixed, so this client handles it three ways, all of which
   * mirror `krappy-node/internal/watch`:
   *
   *   - a cursor that has not moved for longer than the board's own
   *     `dropTTLSeconds` is pointing at storage that no longer exists, so it
   *     is rewound to zero — which costs nothing, because there is nothing
   *     there to re-read;
   *   - a `next` that comes back LOWER than the cursor is followed down
   *     rather than clamped: the board is the authority on its own sequence,
   *     and the alternative is never seeing that topic again;
   *   - duplicates are dropped here, by digest, so following a board
   *     backwards does not re-deliver a whole topic. The node has a second
   *     defence one layer down (the guard's postcard-nonce replay set); a
   *     browser host has no such layer, so this one is not optional.
   *
   * Errors: a transport failure or a 5xx is a hiccup and is retried with
   * backoff, because a board being briefly unreachable is the normal case on
   * the postcard path. A 4xx is not — it means the auth key, the topic or the
   * tenancy is wrong, and retrying forever would hide that — so it is thrown.
   *
   * @param {string} topic
   * @param {object} args
   * @param {{signPub: Uint8Array, sign: Function}} args.keystore the TENANT key
   * @param {number} [args.since]
   * @param {number} [args.wait] requested long-poll seconds
   * @param {AbortSignal} [args.signal] the only way to stop
   * @param {number} [args.pollIntervalSeconds] pause when the board cannot hold
   * @param {(e: BoardError, backoffSeconds: number) => void} [args.onError]
   * @param {(info: object) => void} [args.onCursor] rewinds and renumbers
   * @yields {{seq: number, ts: number, body: Uint8Array, topic: string}}
   */
  async *watch(topic, {
    keystore,
    since = 0,
    wait,
    signal,
    pollIntervalSeconds = 5,
    onError,
    onCursor,
  } = {}) {
    const pol = await this.policy({ signal });
    const rewindAfter = pol.rewindAfterSeconds();
    const seen = new SeenSet();
    let cursor = since;
    let lastMove = this.#now();
    let backoff = 1;

    for (;;) {
      if (signal?.aborted) return;

      const before = this.#now();
      if (cursor > 0 && before - lastMove >= rewindAfter) {
        onCursor?.({ topic, reason: 'rewind', was: cursor, now: 0, idleSeconds: before - lastMove });
        cursor = 0;
        lastMove = before;
      }

      let batch;
      const started = Date.now();
      try {
        batch = await this.read(topic, { keystore, since: cursor, wait, signal });
      } catch (e) {
        if (signal?.aborted || e.kind === 'aborted') return;
        const fatal = e.kind === 'http' && e.status >= 400 && e.status < 500;
        if (fatal || e.kind === 'malformed') throw e;
        onError?.(e, backoff);
        await sleep(e.retryAfterSeconds ?? backoff, signal);
        backoff = Math.min(backoff * 2, 30);
        continue;
      }
      backoff = 1;

      if (batch.items.length > 0) {
        if (batch.next < cursor) {
          onCursor?.({ topic, reason: 'renumbered', was: cursor, now: batch.next });
        }
        cursor = batch.next;
        lastMove = this.#now();
        for (const item of batch.items) {
          if (!(await seen.add(item.body))) continue;
          yield { ...item, topic };
        }
        continue;
      }

      // Nothing there. `wait` is a hint and a board may answer instantly
      // (§7.7); without a floor here a conformant board that returns early
      // would be polled at whatever rate the network allows.
      const elapsed = (Date.now() - started) / 1000;
      if (batch.waitSeconds === 0 || elapsed < 1) {
        await sleep(pollIntervalSeconds, signal).catch(() => {});
        if (signal?.aborted) return;
      }
    }
  }

  // -- §7.9.6 --------------------------------------------------------------

  /**
   * `POST /register` — the open-mode tenancy claim.
   *
   * The token is an ordinary §7.9.5 token whose `topic` slot carries the
   * claimant's own NodeID. That reuse is the whole authentication story and
   * is unguessable from §7.4, so it is worth naming here: registration proves
   * possession of the key whose topics are being claimed, which is the same
   * proof a read requires.
   *
   * On an allowlist board this MUST exist and MUST answer 403 (§7.9.6) — a
   * `403` here is a conformant board saying no, not a broken one.
   *
   * @returns {Promise<{topics: string[], expiresIn: number, epochLen: number}>}
   */
  async register({ keystore, signal } = {}) {
    const nodeId = keystore.nodeId ?? nodeIdFromPub(keystore.signPub);
    const { bytes } = await signAuth({ topic: nodeId, ts: this.#now() }, keystore);
    const res = await this.#request('POST', '/register', {
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify({ nodeId, auth: base64url(bytes) }),
      signal,
      expect: [201],
    });
    const doc = this.#json('board registration', res, this.origin + '/register', 'POST');
    return {
      topics: Array.isArray(doc.topics) ? doc.topics.filter((t) => typeof t === 'string') : [],
      expiresIn: typeof doc.expiresIn === 'number' ? doc.expiresIn : 0,
      epochLen: typeof doc.epochLen === 'number' ? doc.epochLen : DEFAULT_EPOCH_LEN,
    };
  }
}

/** The drop's replay-nonce width, re-exported so a caller needs one import. */
export { DROP_NONCE_LEN };
