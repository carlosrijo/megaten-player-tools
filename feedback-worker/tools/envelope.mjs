// Opening an attachment envelope (src/attachment.js says what is in it) with the developer's private key.
import { privateDecrypt, createDecipheriv, constants } from 'node:crypto';
import { gunzipSync } from 'node:zlib';

/** The envelope's files: { files: [{ name, text }], includeNames, made }. Throws if the key is not the right one. */
export function openEnvelope(envelope, privateKeyPem) {
  const e = typeof envelope === 'string' ? JSON.parse(envelope) : envelope;
  if (!e || e.v !== 1 || e.alg !== 'RSA-OAEP-256+A256GCM') throw new Error('This is not a BABEL feedback attachment (or a newer kind this script does not know).');
  const aesKey = privateDecrypt({ key: privateKeyPem, padding: constants.RSA_PKCS1_OAEP_PADDING, oaepHash: 'sha256' }, Buffer.from(e.key, 'base64'));
  const data = Buffer.from(e.data, 'base64');
  const decipher = createDecipheriv('aes-256-gcm', aesKey, Buffer.from(e.iv, 'base64'));
  decipher.setAuthTag(data.subarray(data.length - 16));   // Web Crypto puts the 16-byte tag at the end
  const zipped = Buffer.concat([decipher.update(data.subarray(0, data.length - 16)), decipher.final()]);
  return JSON.parse(gunzipSync(zipped).toString('utf8'));
}

/** A file name from the envelope as a safe relative path: no drive, no "..", no leading slash. */
export function safeName(name) {
  const parts = String(name).replace(/\\/g, '/').split('/').filter((p) => p && p !== '.' && p !== '..').map((p) => p.replace(/[<>:"|?*\u0000-\u001f]/g, '_'));
  return parts.length ? parts.join('/') : 'file.txt';
}
