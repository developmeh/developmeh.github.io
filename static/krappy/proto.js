// KRAP-1 primitives for the browser host, on WebCrypto.
//
// This file replaces `crypto-min.js`, and the difference is the whole point
// of ADR-0004: every primitive here — Ed25519, X25519, HKDF-SHA256,
// AES-256-GCM — is `crypto.subtle`, reachable now that the WIT's identity
// calls are callback-async. Nothing in this file touches raw private key
// material: private keys arrive as CryptoKey objects and leave as CryptoKey
// objects, which is what lets `identity.js` keep them non-extractable
// (KRAP-2 §8). What remains hand-written is encoding — base36 node ids,
// base32 topics, byte concatenation — which has test vectors and no secrets.
//
// Everything here must agree byte-for-byte with `krappy-proto` (the
// normative Go implementation); `test-host.mjs` checks that against the
// committed cross-language vectors.

const subtle = crypto.subtle;

// ---------------------------------------------------------------------------
// bytes and encodings
// ---------------------------------------------------------------------------

export function concat(...parts) {
  let n = 0;
  for (const p of parts) n += p.length;
  const out = new Uint8Array(n);
  let off = 0;
  for (const p of parts) {
    out.set(p, off);
    off += p.length;
  }
  return out;
}

export function u64be(n) {
  const out = new Uint8Array(8);
  let v = BigInt(n);
  for (let i = 7; i >= 0; i--) {
    out[i] = Number(v & 0xffn);
    v >>= 8n;
  }
  return out;
}

export const utf8Bytes = (s) => new TextEncoder().encode(s);

/** KRAP-1 §2: a NodeID is the 32-byte ed25519 public key as base36, 50 chars. */
export function nodeIdFromPub(pub) {
  let n = 0n;
  for (const b of pub) n = (n << 8n) | BigInt(b);
  return n.toString(36).padStart(50, '0');
}

/** Inverse of `nodeIdFromPub`. Returns null if it does not decode to 32 bytes. */
export function pubFromNodeId(id) {
  if (typeof id !== 'string' || !/^[0-9a-z]{1,50}$/.test(id)) return null;
  let n = 0n;
  for (const ch of id) {
    const d = BigInt(parseInt(ch, 36));
    if (Number.isNaN(Number(d))) return null;
    n = n * 36n + d;
  }
  if (n >= 1n << 256n) return null;
  const out = new Uint8Array(32);
  for (let i = 31; i >= 0; i--) {
    out[i] = Number(n & 0xffn);
    n >>= 8n;
  }
  return out;
}

const BASE32_LOWER = 'abcdefghijklmnopqrstuvwxyz234567';

export function base32nopad(bytes) {
  let bits = 0;
  let value = 0;
  let out = '';
  for (const b of bytes) {
    value = (value << 8) | b;
    bits += 8;
    while (bits >= 5) {
      out += BASE32_LOWER[(value >>> (bits - 5)) & 31];
      bits -= 5;
    }
  }
  if (bits > 0) out += BASE32_LOWER[(value << (5 - bits)) & 31];
  return out;
}

// ---------------------------------------------------------------------------
// pkcs8 import, for keys whose raw bytes are already known
// ---------------------------------------------------------------------------
// WebCrypto imports raw PUBLIC keys but only pkcs8/jwk PRIVATE ones. These
// prefixes are the fixed DER framing for a 32-byte key of each type; the
// production path never comes through here (it generates non-extractable
// pairs directly), but the test harness must be able to load the vectors'
// known keys — as non-extractable CryptoKeys, same as the real thing.

const PKCS8_ED25519 = Uint8Array.from([
  0x30, 0x2e, 0x02, 0x01, 0x00, 0x30, 0x05, 0x06, 0x03, 0x2b, 0x65, 0x70, 0x04, 0x22, 0x04, 0x20,
]);
const PKCS8_X25519 = Uint8Array.from([
  0x30, 0x2e, 0x02, 0x01, 0x00, 0x30, 0x05, 0x06, 0x03, 0x2b, 0x65, 0x6e, 0x04, 0x22, 0x04, 0x20,
]);

/** Import a raw 32-byte ed25519 seed as a non-extractable signing key. */
export function importSignKey(seed) {
  return subtle.importKey('pkcs8', concat(PKCS8_ED25519, seed), { name: 'Ed25519' }, false, ['sign']);
}

/** Import a raw 32-byte X25519 scalar as a non-extractable agreement key. */
export function importEncKey(scalar) {
  return subtle.importKey('pkcs8', concat(PKCS8_X25519, scalar), { name: 'X25519' }, false, ['deriveBits']);
}

// ---------------------------------------------------------------------------
// KRAP-1 protocol constants and derivations
// ---------------------------------------------------------------------------

export const TOPIC_DOMAIN = 'krappy-mbx/1';
export const SEAL_INFO = 'krappy-postcard/1';
export const DEFAULT_EPOCH_LEN = 86400;

/** Epoch containing a Unix-seconds timestamp. */
export const epochOf = (unixSeconds, epochLen = DEFAULT_EPOCH_LEN) =>
  Math.floor(unixSeconds / epochLen);

/** KRAP-1 §7.3 topic derivation: sha256("krappy-mbx/1" || pub || u64be(epoch))[:16], base32. */
export async function deriveTopic(pub, epoch) {
  const sum = new Uint8Array(
    await subtle.digest('SHA-256', concat(utf8Bytes(TOPIC_DOMAIN), pub, u64be(epoch))),
  );
  return base32nopad(sum.subarray(0, 16));
}

// ---------------------------------------------------------------------------
// the §7.6 sealed box
// ---------------------------------------------------------------------------

/** AAD = topic || u64be(ts): a board cannot move a box to another topic (§7.6). */
const sealAad = (topic, ts) => concat(utf8Bytes(topic), u64be(ts));

/** Nonce is 12 zero bytes: the AEAD key is fresh per box (ephemeral ECDH). */
const SEAL_NONCE = new Uint8Array(12);

async function sealKey(shared, epk, recipientPub, usage) {
  const ikm = await subtle.importKey('raw', shared, 'HKDF', false, ['deriveKey']);
  return subtle.deriveKey(
    { name: 'HKDF', hash: 'SHA-256', salt: concat(epk, recipientPub), info: utf8Bytes(SEAL_INFO) },
    ikm,
    { name: 'AES-GCM', length: 256 },
    false,
    [usage],
  );
}

/**
 * KRAP-1 §7.6 seal: `epk(32) || AES-256-GCM(ct||tag)`, keyed by
 * HKDF-SHA256(X25519(eph, recipient), salt = epk||recipientPub).
 *
 * @param {Uint8Array} recipientPub the recipient's X25519 `enc` key (§4.1)
 * @param {string} topic KRAP-1 §7.3 topic the drop will be posted to
 * @param {number|bigint} ts unix seconds (the drop's §7.7 `ts`)
 * @param {Uint8Array} plaintext
 * @returns {Promise<Uint8Array>}
 */
export async function sealBox({ recipientPub, topic, ts, plaintext }) {
  const recipient = await subtle.importKey('raw', recipientPub, { name: 'X25519' }, false, []);
  const eph = await subtle.generateKey({ name: 'X25519' }, false, ['deriveBits']);
  const epk = new Uint8Array(await subtle.exportKey('raw', eph.publicKey));
  const shared = new Uint8Array(
    await subtle.deriveBits({ name: 'X25519', public: recipient }, eph.privateKey, 256),
  );
  const key = await sealKey(shared, epk, recipientPub, 'encrypt');
  const ct = new Uint8Array(
    await subtle.encrypt(
      { name: 'AES-GCM', iv: SEAL_NONCE, additionalData: sealAad(topic, ts), tagLength: 128 },
      key,
      plaintext,
    ),
  );
  return concat(epk, ct);
}

/**
 * Inverse of `sealBox` for boxes addressed to `myKey`. Returns null on any
 * failure — a corrupt box, a wrong recipient and a wrong (topic, ts) are
 * deliberately indistinguishable, because the AAD is part of the tag.
 *
 * @param {CryptoKey} myKey the X25519 private key (non-extractable is fine)
 * @param {Uint8Array} myPub its public half, for the HKDF salt
 */
export async function openBox({ myKey, myPub, topic, ts, sealed }) {
  if (!(sealed?.length >= 32 + 16)) return null;
  const epk = sealed.subarray(0, 32);
  const ct = sealed.subarray(32);
  try {
    const ephPub = await subtle.importKey('raw', epk, { name: 'X25519' }, false, []);
    const shared = new Uint8Array(
      await subtle.deriveBits({ name: 'X25519', public: ephPub }, myKey, 256),
    );
    const key = await sealKey(shared, epk, myPub, 'decrypt');
    return new Uint8Array(
      await subtle.decrypt(
        { name: 'AES-GCM', iv: SEAL_NONCE, additionalData: sealAad(topic, ts), tagLength: 128 },
        key,
        ct,
      ),
    );
  } catch {
    return null;
  }
}
