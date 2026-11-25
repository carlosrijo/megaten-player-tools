// Attachments: locked as the app's page locks them (app.js fbSeal, the same Web Crypto calls), checked and stored by the
// service, opened with the private key (tools/envelope.mjs). No Cloudflare or GitHub needed.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { generateKeyPairSync } from 'node:crypto';
import worker from '../src/index.js';
import { checkAttachment } from '../src/attachment.js';
import { openEnvelope, safeName } from '../tools/envelope.mjs';

const { publicKey, privateKey } = generateKeyPairSync('rsa', {
  modulusLength: 3072, publicKeyEncoding: { type: 'spki', format: 'der' }, privateKeyEncoding: { type: 'pkcs8', format: 'pem' },
});
const PUB = publicKey.toString('base64');

// app.js fbSeal, with the key passed in
const b64 = (buf) => Buffer.from(buf instanceof ArrayBuffer ? buf : buf.buffer).toString('base64');
async function seal(payload, pubB64) {
  const zipped = await new Response(new Blob([JSON.stringify(payload)]).stream().pipeThrough(new CompressionStream('gzip'))).arrayBuffer();
  const pub = await crypto.subtle.importKey('spki', Uint8Array.from(atob(pubB64), (c) => c.charCodeAt(0)), { name: 'RSA-OAEP', hash: 'SHA-256' }, false, ['encrypt']);
  const aes = await crypto.subtle.generateKey({ name: 'AES-GCM', length: 256 }, true, ['encrypt']);
  const iv = crypto.getRandomValues(new Uint8Array(12));
  const data = await crypto.subtle.encrypt({ name: 'AES-GCM', iv }, aes, zipped);
  const key = await crypto.subtle.encrypt({ name: 'RSA-OAEP' }, pub, await crypto.subtle.exportKey('raw', aes));
  return { v: 1, alg: 'RSA-OAEP-256+A256GCM', key: b64(key), iv: b64(iv), data: b64(data) };
}
const payload = { files: [{ name: 'about.txt', text: 'BABEL 0.8.0' }, { name: 'logs/babel.log', text: 'line\r\n'.repeat(5000) }], includeNames: false, made: '2026-11-25T10:00:00Z' };

test('what the page locks, only the private key opens', async () => {
  const env = await seal(payload, PUB);
  assert.equal(env.key.length, 512);   // RSA 3072: 384 bytes
  assert.ok(!JSON.stringify(env).includes('BABEL 0.8.0'));
  assert.deepEqual(openEnvelope(JSON.stringify(env), privateKey), payload);
  const other = generateKeyPairSync('rsa', { modulusLength: 3072, privateKeyEncoding: { type: 'pkcs8', format: 'pem' }, publicKeyEncoding: { type: 'spki', format: 'pem' } });
  assert.throws(() => openEnvelope(env, other.privateKey));
});

test('a changed byte is caught (AES-GCM)', async () => {
  const env = await seal(payload, PUB);
  const d = Buffer.from(env.data, 'base64'); d[5] ^= 1;
  assert.throws(() => openEnvelope({ ...env, data: d.toString('base64') }, privateKey));
});

test('the service checks the envelope\'s shape', async () => {
  const env = await seal(payload, PUB);
  assert.ok(checkAttachment(env, { files: 2, bytes: 30000 }).envelope);
  assert.ok(checkAttachment(null).none);
  assert.match(checkAttachment({ ...env, v: 2 }).error, /could not be read/);
  assert.match(checkAttachment({ ...env, key: 'abc' }).error, /could not be read/);
  assert.match(checkAttachment({ ...env, data: 'A'.repeat(15 * 1024 * 1024) }).error, /too big/);
});

test('file names from an envelope can\'t leave the folder', () => {
  assert.equal(safeName('../../Windows/x.txt'), 'Windows/x.txt');
  assert.equal(safeName('C:\\evil\\a.log'), 'C_/evil/a.log');
  assert.equal(safeName('logs/babel.log'), 'logs/babel.log');
});

test('a send with files: stored for 180 days, named in the issue; a test send kept an hour', async () => {
  const stored = [];
  const kv = { put: async (k, v, o) => { stored.push({ k, v, o }); } };
  const { privateKey: appKey } = generateKeyPairSync('rsa', { modulusLength: 2048, privateKeyEncoding: { type: 'pkcs1', format: 'pem' }, publicKeyEncoding: { type: 'spki', format: 'pem' } });
  const env = { REPO: 'owner/repo', APP_ID: '1', INSTALLATION_ID: '2', GITHUB_APP_KEY: appKey, FEEDBACK_FILES: kv };
  let made;
  globalThis.fetch = async (url, init) => {
    if (url.endsWith('/access_tokens')) return Response.json({ token: 't', expires_at: new Date(Date.now() + 3600e3).toISOString() });
    made = JSON.parse(init.body);
    return Response.json({ number: 9, html_url: 'u' }, { status: 201 });
  };
  const attachment = await seal(payload, PUB);
  const send = (test) => worker.fetch(new Request('https://x/feedback', { method: 'POST', headers: { 'Content-Type': 'application/json', Origin: 'https://imagine.app' },
    body: JSON.stringify({ kind: 'bug', message: 'Crashes when I take a snapshot', attachment, attachmentInfo: { files: 2, bytes: 30011 }, test }) }), env);

  const r = await (await send(false)).json();
  assert.equal(r.number, 9);
  assert.equal(stored.length, 1);
  const id = stored[0].k.replace('files/', '');
  assert.match(id, /^[a-z0-9]{16}$/);
  assert.equal(stored[0].o.expirationTtl, 180 * 24 * 3600);
  assert.match(made.body, new RegExp(`Diagnostics attached\\*\\*, encrypted .*\`${id}\` \\(2 files, 29 KB\\)`));
  assert.deepEqual(openEnvelope(stored[0].v, privateKey), payload);

  const t = await (await send(true)).json();
  assert.match(t.attachment, /^test-[a-z0-9]{16}$/);
  assert.equal(stored[1].o.expirationTtl, 3600);

  // the store refuses (the free plan's 1,000 writes a day): the feedback still goes, the issue says so
  kv.put = async () => { throw new Error('KV put() limit exceeded for the day.'); };
  const f = await (await send(false)).json();
  assert.equal(f.number, 9);
  assert.equal(f.filesStored, false);
  assert.match(made.body, /Diagnostics files were sent \(2 files, 29 KB\) but \*\*could not be stored\*\*/);
  assert.doesNotMatch(made.body, /Diagnostics attached/);
});

test('every file written is plain text: the names the app sends stay, anything else gets .txt', () => {
  for (const n of ['about.txt', 'doctor.txt', 'logs/imagine-20261124-000000-000.log', 'logs/crash-20261124-000000-000.txt', 'data/config.json', 'data/characters.json', 'settings.json', 'data-listing.txt']) {
    assert.equal(safeName(n), n);
  }
  assert.equal(safeName('x.url'), 'x.url.txt');
  assert.equal(safeName('desktop.ini'), 'desktop.ini.txt');
  assert.equal(safeName('a/b.lnk'), 'a/b.lnk.txt');
  assert.equal(safeName('run.bat'), 'run.bat.txt');
  assert.equal(safeName('NOTES.TXT'), 'NOTES.TXT');
  assert.equal(safeName('x.json.url'), 'x.json.url.txt');
  assert.equal(safeName('noextension'), 'noextension.txt');
  assert.equal(safeName('../../x.json'), 'x.json');
  assert.equal(safeName('a.url.'), 'a.url.txt');
  assert.equal(safeName('a.url. . '), 'a.url.txt');
  assert.equal(safeName('a.txt.'), 'a.txt');
  assert.equal(safeName('dir /b.log'), 'dir/b.log');
  assert.equal(safeName('a.txt:stream'), 'a.txt_stream.txt');
  assert.equal(safeName('..'), 'file.txt');
  assert.equal(safeName('. . '), 'file.txt');
});

test('Windows device names get a leading underscore', () => {
  assert.equal(safeName('con'), '_con.txt');
  assert.equal(safeName('CON.txt'), '_CON.txt');
  assert.equal(safeName('logs/nul.log'), 'logs/_nul.log');
  assert.equal(safeName('com1.json'), '_com1.json');
  assert.equal(safeName('LPT9.x'), '_LPT9.x.txt');
  assert.equal(safeName('aux/b.txt'), '_aux/b.txt');
  assert.equal(safeName('console.txt'), 'console.txt');
  assert.equal(safeName('com10.txt'), 'com10.txt');
});

test('an attachment that unpacks to more than the cap is not opened', async () => {
  const env = await seal(payload, PUB);
  assert.deepEqual(openEnvelope(env, privateKey, { maxBytes: 1024 * 1024 }), payload);
  assert.throws(() => openEnvelope(env, privateKey, { maxBytes: 1000 }), /larger than 1000 bytes unpacked; not opened/);
  const big = await seal({ files: [{ name: 'big.txt', text: 'x'.repeat(3 * 1024 * 1024) }] }, PUB);   // small when zipped
  assert.throws(() => openEnvelope(big, privateKey, { maxBytes: 2 * 1024 * 1024 }), /The attachment is larger than 2 MB unpacked; not opened\./);
  assert.equal(openEnvelope(big, privateKey).files[0].text.length, 3 * 1024 * 1024);
});
