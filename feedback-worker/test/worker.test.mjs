// node --test test/   (no Cloudflare or GitHub needed: GitHub's API is stood in for)
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { generateKeyPairSync, createVerify } from 'node:crypto';
import worker from '../src/index.js';
import { check, makeIssue, quiet } from '../src/issue.js';
import { appJwt } from '../src/github.js';

const { privateKey, publicKey } = generateKeyPairSync('rsa', {
  modulusLength: 2048,
  privateKeyEncoding: { type: 'pkcs1', format: 'pem' },   // what GitHub gives: BEGIN RSA PRIVATE KEY
  publicKeyEncoding: { type: 'spki', format: 'pem' },
});
const env = { REPO: 'owner/repo', APP_ID: '123', INSTALLATION_ID: '456', GITHUB_APP_KEY: privateKey };
const about = { version: '0.8.0', commit: 'abc1234', built: '2026-11-25', windows: 'Windows 11 (26300)', webview2: '142.0' };

test('the app signs in with GitHub\'s PKCS#1 key', async () => {
  const jwt = await appJwt(env);
  const [h, p, s] = jwt.split('.');
  const v = createVerify('RSA-SHA256'); v.update(`${h}.${p}`);
  assert.ok(v.verify(publicKey, Buffer.from(s, 'base64url')));
  const claims = JSON.parse(Buffer.from(p, 'base64url'));
  assert.equal(claims.iss, '123');
  assert.ok(claims.exp - claims.iat <= 600);
});

test('a short message, or no kind, is turned down with plain words', () => {
  assert.match(check({ kind: 'bug', message: 'short' }).error, /at least 10/);
  assert.match(check({ message: 'long enough message' }).error, /bug, an idea or a question/);
});

test('the issue: title, labels, no pings, no HTML, version table', () => {
  const f = check({ kind: 'idea', message: 'Please add @someone <b>bold</b> stuff\nsecond line', contact: 'gh: @me', about });
  const i = makeIssue(f);
  assert.equal(i.title, '[Idea] Please add @someone <b>bold</b> stuff');   // a title is plain text on GitHub: no pings, no HTML
  assert.match(i.body, /^Please add @​someone &lt;b>bold&lt;\/b> stuff\nsecond line\n/);
  assert.deepEqual(i.labels, ['feedback', 'enhancement']);
  assert.ok(!/@[a-z]/.test(i.body), 'no @mention left');
  assert.match(i.body, /\| Version \| 0\.8\.0, commit abc1234, built 2026-11-25 \|/);
  assert.match(i.body, /\*\*Contact:\*\* gh: @​me/);
});

test('page/tool sections: one heading each, title from the first when there is no message', () => {
  const f = check({ kind: 'bug', message: '', sections: [{ page: 'Demon Database', text: 'Filter by race forgets the choice' }, { page: 'Gear ## <x>', text: 'Tooltip cut off' }] });
  assert.equal(f.error, undefined);
  const i = makeIssue(f);
  assert.equal(i.title, '[Bug] Demon Database: Filter by race forgets the choice');
  assert.match(i.body, /^### Demon Database\n\nFilter by race forgets the choice\n\n### Gear x\n\nTooltip cut off\n\n---/);
  assert.match(check({ kind: 'bug', message: 'long enough message', sections: [{ page: 'Gear', text: '' }] }).error, /about Gear/);
  assert.match(check({ kind: 'bug', message: 'long enough message', sections: [{ page: '', text: 'x' }] }).error, /Pick a page/);
});

// GitHub stood in for: records what was asked
function fakeGitHub() {
  const calls = [];
  globalThis.fetch = async (url, init) => {
    calls.push({ url, init });
    if (url.endsWith('/access_tokens')) return Response.json({ token: 'tok', expires_at: new Date(Date.now() + 3600e3).toISOString() });
    if (url.endsWith('/issues')) return Response.json({ number: 7, html_url: 'https://github.com/owner/repo/issues/7' }, { status: 201 });
    return new Response('?', { status: 404 });
  };
  return calls;
}
const post = (body, origin = 'https://imagine.app', extra = {}) => worker.fetch(new Request('https://x.workers.dev/feedback', {
  method: 'POST', headers: { 'Content-Type': 'application/json', Origin: origin, 'CF-Connecting-IP': '1.2.3.4' }, body: JSON.stringify(body),
}), { ...env, ...extra });

test('a send opens the issue and answers with its number', async () => {
  const calls = fakeGitHub();
  const r = await post({ kind: 'bug', message: 'The Overview page is empty after a snapshot', about });
  assert.equal(r.status, 200);
  assert.equal(r.headers.get('Access-Control-Allow-Origin'), 'https://imagine.app');
  assert.deepEqual(await r.json(), { ok: true, number: 7, url: 'https://github.com/owner/repo/issues/7' });
  const made = calls.find((c) => c.url === 'https://api.github.com/repos/owner/repo/issues');
  assert.equal(made.init.headers.Authorization, 'Bearer tok');
  assert.equal(JSON.parse(made.init.body).title, '[Bug] The Overview page is empty after a snapshot');
});

test('test mode signs in but opens no issue', async () => {
  const calls = fakeGitHub();
  const r = await post({ kind: 'question', message: 'How do I take a snapshot?', test: true }, 'http://localhost:18765');
  const j = await r.json();
  assert.equal(j.test, true);
  assert.equal(j.issue.title, '[Question] How do I take a snapshot?');
  assert.ok(!calls.some((c) => c.url.endsWith('/issues')));
});

test('the hidden field filled in: answered as sent, nothing opened', async () => {
  const calls = fakeGitHub();
  const r = await post({ kind: 'bug', message: 'buy cheap things here now', website: 'spam.example' });
  assert.equal((await r.json()).ok, true);
  assert.equal(calls.length, 0);
});

test('too many sends: 429', async () => {
  fakeGitHub();
  const r = await post({ kind: 'bug', message: 'long enough message' }, undefined, { PER_IP: { limit: async () => ({ success: false }) } });
  assert.equal(r.status, 429);
});

test('other web pages get no CORS answer', async () => {
  const r = await worker.fetch(new Request('https://x.workers.dev/feedback', { method: 'OPTIONS', headers: { Origin: 'https://evil.example' } }), env);
  assert.equal(r.status, 403);
  const ok = await worker.fetch(new Request('https://x.workers.dev/feedback', { method: 'OPTIONS', headers: { Origin: 'https://imagine.app' } }), env);
  assert.equal(ok.status, 204);
});

test('the issue shows the player\'s text as plain text: no links, images, references or pings', () => {
  const message = [
    'see [click](https://x.y) and ![i](https://x.y/a.png)',
    'ref [text][r1] and \\[escaped](http://x.y)',
    '[r1]: ftp://x.y',
    'https://x.y, HTTP://X.Y and www.x.y, mailto:a@b.c',
    'owner/repo#12 and #12 and GH-12 and gh-7',
    'owner/repo@abc1234 and a@b',
  ].join('\n');
  const i = makeIssue(check({ kind: 'bug', message, contact: '[me](https://x.y) #3', about }));
  const body = i.body.split('\n---')[0];
  assert.equal(body, [
    'see \\[click\\](https[:]//x.y) and !\\[i\\](https[:]//x.y/a.png)',
    'ref \\[text\\]\\[r1\\] and \\\\\\[escaped\\](http[:]//x.y)',
    '\\[r1\\]: ftp[:]//x.y',
    'https[:]//x.y, HTTP[:]//X.Y and www[.]x.y, mailto[:]a@\u200bb.c',
    'owner/repo#\u200b12 and #\u200b12 and GH-\u200b12 and gh-\u200b7',
    'owner/repo@\u200babc1234 and a@\u200bb',
    '',
  ].join('\n'));
  assert.match(i.body, /\*\*Contact:\*\* \\\[me\\\]\(https\[:\]\/\/x\.y\) #\u200b3/);
  assert.ok(!/(?<!\\)\]\(/.test(i.body), 'no link left');
  assert.ok(!/https?:/i.test(i.body) && !/#\d/.test(i.body) && !/GH-\d/i.test(i.body));
});

test('ordinary text is left as written', () => {
  const text = 'I lost 50% HP at level 99, why? Seen in Tokyo (Ginza) at 3:45 - the "boss" fight; HP/MP & stuff.';
  assert.equal(quiet(text), text);
  assert.equal(makeIssue(check({ kind: 'bug', message: text })).body.split('\n')[0], text);
  assert.equal(quiet('Please add @someone <b>bold</b>'), 'Please add @\u200bsomeone &lt;b>bold&lt;/b>');
});
