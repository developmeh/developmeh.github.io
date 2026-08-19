// The host's entire persistence dependency, as one injected interface.
//
// `host-core.js` is deliberately platform-free: it takes a MessagePort-shaped
// object and a way to instantiate a component, and now a Storage. IndexedDB is
// a platform API, so it does NOT belong in host-core — it lives in
// `idb-storage.js`, behind this interface, and `host-worker.js` (the platform
// seam) is the only file that knows the browser implementation exists.
// `MemoryStorage` below is the Node-harness implementation and doubles as the
// fallback for a context with no IndexedDB (private-mode Safari has shipped
// that state more than once).
//
// ---------------------------------------------------------------------------
// interface Storage {
//   get(ns, key)   -> Promise<value | undefined>
//   put(ns, key, v) -> Promise<void>
//   del(ns, key)   -> Promise<void>
//   entries(ns)    -> Promise<Array<[key, value]>>   // key order, ascending
//   namespaces()   -> Promise<string[]>
// }
// ---------------------------------------------------------------------------
//
// Namespaces the host uses:
//
//   identity          the per-origin key material (KRAP-2 §8)
//   records           Dial Records the host has been told about, by node id
//   store:<appId>     one app's KV (the `store` interface)
//   outbox:<appId>    one app's outbox rows, keyed by idem (KRAP-2 §6)
//   outseq:<appId>    per-target `seq` counters, so ordering survives restart
//
// Values must survive a structured clone: Uint8Array and plain
// objects/strings/numbers only. Notably NOT BigInt — IndexedDB implementations
// have historically disagreed about it — so the outbox serialises `seq`, `ts`
// and `exp` as decimal strings and converts at the boundary.

const clone = (v) => (typeof structuredClone === 'function' ? structuredClone(v) : v);

/** In-memory Storage. Share one instance between two Hosts to simulate a restart. */
export class MemoryStorage {
  #ns = new Map(); // ns -> Map(key -> value)

  #bucket(ns) {
    let m = this.#ns.get(ns);
    if (!m) {
      m = new Map();
      this.#ns.set(ns, m);
    }
    return m;
  }

  async get(ns, key) {
    const v = this.#ns.get(ns)?.get(key);
    return v === undefined ? undefined : clone(v);
  }

  async put(ns, key, value) {
    // Clone on write as IndexedDB does, so a caller mutating the object it
    // handed us does not retroactively edit what is "on disk".
    this.#bucket(ns).set(key, clone(value));
  }

  async del(ns, key) {
    this.#ns.get(ns)?.delete(key);
  }

  async entries(ns) {
    const m = this.#ns.get(ns);
    if (!m) return [];
    return [...m.entries()].sort(([a], [b]) => (a < b ? -1 : a > b ? 1 : 0)).map(([k, v]) => [k, clone(v)]);
  }

  async namespaces() {
    return [...this.#ns.keys()].filter((n) => this.#ns.get(n).size > 0).sort();
  }
}

/**
 * Serialises writes and lets a caller await quiescence.
 *
 * The WIT's `store` and `outbox` are synchronous and IndexedDB is not, so the
 * host is hydrate-then-write-through: reads are served from memory, writes
 * update memory synchronously and are persisted on a queue. The durability
 * this buys is "a write is on disk shortly after it returns", not "on disk
 * when it returns" — a tab killed in that window loses the last write. Making
 * it stronger requires an async `store.set`, i.e. a WIT change.
 */
export class WriteQueue {
  #tail = Promise.resolve();
  #onError;
  errors = 0;

  constructor(onError = () => {}) {
    this.#onError = onError;
  }

  /** Queue `fn`; never rejects (a failed persist is traced, not thrown at an app). */
  push(fn) {
    this.#tail = this.#tail.then(fn).catch((e) => {
      this.errors++;
      this.#onError(e);
    });
    return this.#tail;
  }

  /** Resolve once every queued write has settled. */
  settled() {
    return this.#tail;
  }
}
