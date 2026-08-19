// The host's per-origin identity (KRAP-2 §8) and the `identity` oracle.
//
// Two keys, generated together at first run and persisted through the Storage
// seam:
//
//   ed25519  the identity. `self-id` is base36 of its public key (KRAP-1 §2),
//            and it must be STABLE across restarts — KRAP-1 §7.7's drop
//            signature is a rate limit, and a rate limit against an identity
//            that is new every session is not one.
//   X25519   an INDEPENDENT `enc` key for seal/open (KRAP-1 §4.1, KRAP-2 §8).
//            Not a birational conversion of the ed25519 key: conversion needs
//            the raw private scalar, and the whole reason §8 wants two keys is
//            to keep that unnecessary.
//
// Both private keys are NON-EXTRACTABLE CryptoKeys, generated that way and
// persisted as CryptoKey objects through the Storage seam (a CryptoKey is
// structured-clonable, which is exactly the property IndexedDB persistence
// needs). No raw private byte ever exists in this worker's JS heap — §8's
// property holds against the host's own context, not just against apps.
//
// This is what ADR-0004 bought. The previous revision of this file sat on
// 698 lines of hand-written crypto (`crypto-min.js`, now deleted) because
// the WIT's identity calls were synchronous and `crypto.subtle` is not.
// With the calls callback-async, every primitive is the platform's own.

import {
  nodeIdFromPub,
  pubFromNodeId,
  importSignKey,
  importEncKey,
  sealBox,
  openBox,
} from './proto.js';
import { WitError } from './wit-error.js';

const subtle = crypto.subtle;

const NS = 'identity';

// v1 was the held branch's raw-seed layout. Not migrated, deliberately:
// nothing was ever deployed, and importing a seed that once sat extractable
// in storage would launder it into a key that merely *looks* protected.
const KEYSTORE_VERSION = 2;

export class Keystore {
  /** @type {string} base36 node id (KRAP-1 §2) */
  nodeId;
  /** @type {Uint8Array} ed25519 public key */
  signPub;
  /** @type {Uint8Array} X25519 public key, the `enc` field of a Dial Record */
  encPub;
  /** True if this run generated the keys rather than loading them. */
  fresh = false;

  #signKey; // CryptoKey, ed25519, non-extractable
  #encKey; // CryptoKey, X25519, non-extractable

  constructor(signKey, signPub, encKey, encPub, fresh) {
    this.#signKey = signKey;
    this.#encKey = encKey;
    this.signPub = Uint8Array.from(signPub);
    this.encPub = Uint8Array.from(encPub);
    this.fresh = fresh;
    this.nodeId = nodeIdFromPub(this.signPub);
  }

  /**
   * Load the origin's keys, generating them on first run.
   *
   * MUST be awaited before any component is instantiated: `identity.self-id`
   * is the one call that stayed synchronous (it is a constant), so the id has
   * to exist before an app can ask for it.
   */
  static async open(storage) {
    const have = await storage.get(NS, 'v');
    if (have === KEYSTORE_VERSION) {
      const [signKey, signPub, encKey, encPub] = await Promise.all([
        storage.get(NS, 'sign.key'),
        storage.get(NS, 'sign.pub'),
        storage.get(NS, 'enc.key'),
        storage.get(NS, 'enc.pub'),
      ]);
      if (signKey && encKey && signPub?.length === 32 && encPub?.length === 32) {
        return new Keystore(signKey, signPub, encKey, encPub, false);
      }
    }
    const sign = await subtle.generateKey({ name: 'Ed25519' }, false, ['sign']);
    const enc = await subtle.generateKey({ name: 'X25519' }, false, ['deriveBits']);
    const signPub = new Uint8Array(await subtle.exportKey('raw', sign.publicKey));
    const encPub = new Uint8Array(await subtle.exportKey('raw', enc.publicKey));
    const ks = new Keystore(sign.privateKey, signPub, enc.privateKey, encPub, true);
    // Order matters: write the version marker last, so a run interrupted
    // mid-generation regenerates rather than loading half a keypair.
    await storage.put(NS, 'sign.key', sign.privateKey);
    await storage.put(NS, 'sign.pub', signPub);
    await storage.put(NS, 'enc.key', enc.privateKey);
    await storage.put(NS, 'enc.pub', encPub);
    await storage.put(NS, 'v', KEYSTORE_VERSION);
    return ks;
  }

  /**
   * A keystore over KNOWN key bytes — the test harness loading the
   * cross-language vectors' keys. They are still imported non-extractable;
   * what is different is only that the caller once saw the bytes. Never on
   * the production path. `encSk`/`encPub` are required; the signing pair is
   * generated fresh unless `signSeed`/`signPub` are given.
   */
  static async fromRaw({ signSeed, signPub, encSk, encPub }) {
    let signKey;
    if (signSeed) {
      signKey = await importSignKey(Uint8Array.from(signSeed));
    } else {
      const pair = await subtle.generateKey({ name: 'Ed25519' }, false, ['sign']);
      signKey = pair.privateKey;
      signPub = new Uint8Array(await subtle.exportKey('raw', pair.publicKey));
    }
    const encKey = await importEncKey(Uint8Array.from(encSk));
    return new Keystore(signKey, signPub, encKey, encPub, true);
  }

  /** @returns {Promise<Uint8Array>} 64-byte ed25519 signature */
  async sign(data) {
    return new Uint8Array(await subtle.sign('Ed25519', this.#signKey, Uint8Array.from(data)));
  }

  /** @param {string} signer base36 node id @returns {Promise<boolean>} */
  async verify(signer, data, sig) {
    const pub = pubFromNodeId(signer);
    if (!pub) return false;
    try {
      const key = await subtle.importKey('raw', pub, { name: 'Ed25519' }, false, ['verify']);
      return await subtle.verify('Ed25519', key, Uint8Array.from(sig), Uint8Array.from(data));
    } catch {
      return false;
    }
  }

  /**
   * KRAP-1 §7.6 seal to a recipient's `enc` key. `topic` and `ts` are the
   * caller's choice (ADR-0004): near an epoch rollover the right topic may be
   * the previous or next one in the §7.3 window, so neither is derivable
   * from the clock here.
   */
  sealTo({ recipientEnc, topic, ts, plaintext }) {
    return sealBox({ recipientPub: recipientEnc, topic, ts, plaintext: Uint8Array.from(plaintext) });
  }

  /** Inverse of `sealTo` for boxes addressed to this host's `enc` key. */
  open({ topic, ts, sealed }) {
    return openBox({
      myKey: this.#encKey,
      myPub: this.encPub,
      topic,
      ts,
      sealed: Uint8Array.from(sealed),
    });
  }
}

/**
 * One app's audit record. §8 claims per-app oracle usage is auditable by the
 * host; this is what makes that a fact rather than an intention.
 */
export class OracleAudit {
  counts = { selfId: 0, sign: 0, verify: 0, seal: 0, open: 0 };
  /** Bounded ring — an app that signs in a loop must not grow the worker. */
  recent = [];
  static LIMIT = 32;

  record(op, detail, nowMs) {
    this.counts[op] = (this.counts[op] ?? 0) + 1;
    this.recent.push({ op, detail, ts: nowMs });
    if (this.recent.length > OracleAudit.LIMIT) this.recent.shift();
  }

  toJSON() {
    return { counts: { ...this.counts }, recent: this.recent.slice(-8) };
  }
}

/**
 * The `krappy:host/identity` oracle for one principal (an app, or the host
 * itself). `selfId` is synchronous; everything else returns a promise and
 * throws `WitError` on failure — the request-id correlation that turns these
 * into WIT-shaped calls lives in host-core, because it owns the app's id
 * space and the callback queue.
 *
 * `resolveEnc(nodeId)` returns that node's `enc` key from the host's Dial
 * Record cache, or null. Passing the host's own id resolves without a record.
 */
export function identityOracle({ keystore, audit, now, resolveEnc, trace }) {
  const ms = () => now();
  return {
    selfId: () => {
      audit.record('selfId', '', ms());
      return keystore.nodeId;
    },

    sign: async (data) => {
      audit.record('sign', `${data.length} bytes`, ms());
      return keystore.sign(data);
    },

    verify: async (signer, data, sig) => {
      const ok = await keystore.verify(signer, data, sig);
      audit.record('verify', `${signer.slice(0, 8)}… -> ${ok}`, ms());
      return ok;
    },

    seal: async (to, topic, ts, data) => {
      const resolved = resolveEnc(to);
      if (!resolved) {
        audit.record('seal', `${to.slice(0, 8)}… no record`, ms());
        throw new WitError(
          'invalid',
          `no Dial Record for ${to.slice(0, 8)}…: the host cannot resolve its enc key`,
        );
      }
      audit.record('seal', `${to.slice(0, 8)}… ${data.length} bytes @${topic.slice(0, 8)}…`, ms());
      try {
        return await keystore.sealTo({ recipientEnc: resolved, topic, ts, plaintext: data });
      } catch (e) {
        trace?.(`identity.seal failed: ${e?.message ?? e}`);
        throw new WitError('invalid', `seal failed: ${e?.message ?? e}`);
      }
    },

    open: async (topic, ts, sealed) => {
      const out = await keystore.open({ topic, ts, sealed });
      audit.record('open', out ? `${out.length} bytes` : 'rejected', ms());
      if (!out) {
        throw new WitError(
          'invalid',
          'not a box sealed to this host under that (topic, ts), or it was tampered with',
        );
      }
      return out;
    },
  };
}
