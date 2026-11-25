// Attachments: the diagnostics files a player picked in the app, encrypted there before they were sent.
//
// The envelope (JSON) the app sends, and the one stored:
//   { v: 1, alg: "RSA-OAEP-256+A256GCM", key, iv, data }
//   key   base64: a fresh AES-256 key, encrypted with the developer's public key (RSA-OAEP, SHA-256, 3072 bits)
//   iv    base64: 12 bytes for AES-GCM
//   data  base64: AES-GCM of the gzipped JSON { files: [{ name, text }], includeNames, made }
// This service never has the private key, so it can only check the envelope's shape and store it. The developer opens
// it with feedback-worker\tools\open-files.ps1 <id>.

export const ALG = 'RSA-OAEP-256+A256GCM';
export const MAX_DATA = 14 * 1024 * 1024;   // base64 characters (about 10 MB of encrypted, compressed files)
export const KEEP_SECONDS = 180 * 24 * 3600;
export const TEST_KEEP_SECONDS = 3600;

const B64 = /^[A-Za-z0-9+/]+={0,2}$/;

/** The envelope, checked; or { error } in plain words. `info` is what the app says about it (count and size, unencrypted). */
export function checkAttachment(a, info) {
  if (a == null) return { none: true };
  if (typeof a !== 'object' || a.v !== 1 || a.alg !== ALG) return { error: 'The attached files could not be read.' };
  const ok = (s, min, max) => typeof s === 'string' && s.length >= min && s.length <= max && B64.test(s);
  if (!ok(a.key, 500, 520) || !ok(a.iv, 16, 16) || !ok(a.data, 24, MAX_DATA)) {
    return a && typeof a.data === 'string' && a.data.length > MAX_DATA ? { error: 'The attached files are too big (10 MB at most).' } : { error: 'The attached files could not be read.' };
  }
  const files = Number.isInteger(info && info.files) && info.files > 0 && info.files < 1000 ? info.files : null;
  const bytes = Number.isInteger(info && info.bytes) && info.bytes > 0 ? info.bytes : null;
  return { envelope: { v: 1, alg: ALG, key: a.key, iv: a.iv, data: a.data }, files, bytes };
}

/** A short id nobody can guess: 16 letters and digits (no look-alikes). */
export function newId(test) {
  const abc = 'abcdefghjkmnpqrstuvwxyz23456789';
  const r = crypto.getRandomValues(new Uint8Array(16));
  return (test ? 'test-' : '') + Array.from(r, (b) => abc[b % abc.length]).join('');
}

/** Keep it: Workers KV, deleted by Cloudflare after 180 days (an hour for a test send). */
export async function storeAttachment(env, id, envelope, test) {
  if (!env.FEEDBACK_FILES) throw new Error('The FEEDBACK_FILES store is not bound.');
  await env.FEEDBACK_FILES.put(`files/${id}`, JSON.stringify(envelope), {
    expirationTtl: test ? TEST_KEEP_SECONDS : KEEP_SECONDS,
    metadata: { at: new Date().toISOString() },
  });
}
