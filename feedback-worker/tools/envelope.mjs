// Opening an attachment envelope (src/attachment.js says what is in it) with the developer's private key.
import { privateDecrypt, createDecipheriv, constants } from 'node:crypto';
import { gunzipSync } from 'node:zlib';

const MB = 1024 * 1024;

/** The envelope's files: { files: [{ name, text }], includeNames, made }. Throws if the key is not the right one,
 *  or if the files add up to more than maxBytes unpacked (64 MB unless given). */
export function openEnvelope(envelope, privateKeyPem, { maxBytes = 64 * MB } = {}) {
  const e = typeof envelope === 'string' ? JSON.parse(envelope) : envelope;
  if (!e || e.v !== 1 || e.alg !== 'RSA-OAEP-256+A256GCM') throw new Error('This is not a BABEL feedback attachment (or a newer kind this script does not know).');
  const aesKey = privateDecrypt({ key: privateKeyPem, padding: constants.RSA_PKCS1_OAEP_PADDING, oaepHash: 'sha256' }, Buffer.from(e.key, 'base64'));
  const data = Buffer.from(e.data, 'base64');
  const decipher = createDecipheriv('aes-256-gcm', aesKey, Buffer.from(e.iv, 'base64'));
  decipher.setAuthTag(data.subarray(data.length - 16));   // Web Crypto puts the 16-byte tag at the end
  const zipped = Buffer.concat([decipher.update(data.subarray(0, data.length - 16)), decipher.final()]);
  let unpacked;
  try { unpacked = gunzipSync(zipped, { maxOutputLength: maxBytes }); }
  catch (err) {
    if (err && err.code === 'ERR_BUFFER_TOO_LARGE') {
      throw new Error(`The attachment is larger than ${maxBytes >= MB ? `${Math.round(maxBytes / MB)} MB` : `${maxBytes} bytes`} unpacked; not opened.`);
    }
    throw err;
  }
  return JSON.parse(unpacked.toString('utf8'));
}

const DEVICE = /^(con|prn|aux|nul|conin\$|conout\$|com[1-9¹²³]|lpt[1-9¹²³])$/i;

/** A file name from the envelope as a safe relative path: no drive, no "..", no leading slash, no trailing dots or
 *  spaces, no device names, and always a plain text file (.txt, .log or .json; anything else gets ".txt" added). */
export function safeName(name) {
  const parts = String(name).replace(/\\/g, '/').split('/')
    .map((p) => p.replace(/[<>:"|?*\u0000-\u001f]/g, '_').replace(/[. ]+$/, ''))
    .filter((p) => p && p !== '.' && p !== '..')
    .map((p) => (DEVICE.test(p.split('.')[0].trimEnd()) ? `_${p}` : p));
  if (!parts.length) return 'file.txt';
  if (!/\.(txt|log|json)$/i.test(parts[parts.length - 1])) parts[parts.length - 1] += '.txt';
  return parts.join('/');
}
