// BABEL feedback service (Cloudflare Worker).
//
// POST /feedback  { kind, title?, message, contact?, about?, test?, website? }  ->  { ok, number, url }
//   kind     "bug" | "idea" | "question"
//   message  what the player wrote (10 to 4000 characters)
//   contact  optional: how to reach them (the form asks for a GitHub name; the issue is public)
//   about    optional: { version, commit, built, windows, webview2 } from the app's "About this version"
//   test     true: check everything (including signing in to GitHub) and answer with the issue it would open, but open
//            none. The browser preview of the app always sends test.
//   website  a field the form hides: people leave it empty, form-filling bots don't.
//
// It opens the issue as the "BABEL feedback" GitHub App (Issues: read and write on this one repo). The app's private key
// is the Worker secret GITHUB_APP_KEY; APP_ID, INSTALLATION_ID and REPO are in wrangler.toml.

import { check, makeIssue } from './issue.js';
import { gh, installationToken } from './github.js';
import { checkAttachment, newId, storeAttachment, MAX_DATA } from './attachment.js';

// bytes: a message, up to 20 page/tool sections of 4000 characters each, and the encrypted attachment
const MAX_REQUEST = MAX_DATA + 200000;

// Who may call it from a browser: the app's own page and the browser preview on this PC. (Anyone can still call it
// without a browser; the rate limit and the checks below are what keep it in hand.)
function allowedOrigin(origin) {
  if (!origin) return null;
  if (origin === 'https://imagine.app') return origin;
  if (/^http:\/\/(localhost|127\.0\.0\.1)(:\d+)?$/.test(origin)) return origin;
  return null;
}

function reply(status, data, origin) {
  const headers = { 'Content-Type': 'application/json; charset=utf-8', 'Cache-Control': 'no-store' };
  if (origin) {
    headers['Access-Control-Allow-Origin'] = origin;
    headers['Vary'] = 'Origin';
  }
  return new Response(JSON.stringify(data), { status, headers });
}
const fail = (status, error, origin) => reply(status, { ok: false, error }, origin);

export default {
  async fetch(request, env) {
    const url = new URL(request.url);
    const origin = allowedOrigin(request.headers.get('Origin'));

    if (request.method === 'OPTIONS') {
      if (!origin) return new Response(null, { status: 403 });
      return new Response(null, {
        status: 204,
        headers: {
          'Access-Control-Allow-Origin': origin,
          'Access-Control-Allow-Methods': 'POST, OPTIONS',
          'Access-Control-Allow-Headers': 'Content-Type',
          'Access-Control-Max-Age': '86400',
          'Vary': 'Origin',
        },
      });
    }
    if (url.pathname === '/' && request.method === 'GET') {
      return new Response('BABEL feedback service. The app posts to /feedback.\n', { headers: { 'Content-Type': 'text/plain' } });
    }
    if (url.pathname !== '/feedback') return fail(404, 'Not found.', origin);
    if (request.method !== 'POST') return fail(405, 'Use POST.', origin);

    // one address, at most 2 sends a minute
    if (env.PER_IP) {
      const ip = request.headers.get('CF-Connecting-IP') || 'unknown';
      const { success } = await env.PER_IP.limit({ key: ip });
      if (!success) return fail(429, 'That was a lot of feedback at once. Wait a minute, then send it again.', origin);
    }

    const length = Number(request.headers.get('Content-Length') || 0);
    if (length > MAX_REQUEST) return fail(413, 'The message and its files are too big.', origin);
    let input;
    try {
      const text = await request.text();
      if (text.length > MAX_REQUEST) return fail(413, 'The message and its files are too big.', origin);
      input = JSON.parse(text);
    } catch {
      return fail(400, 'The service could not read what was sent.', origin);
    }
    if (!input || typeof input !== 'object') return fail(400, 'The service could not read what was sent.', origin);

    const checked = check(input);
    if (checked.error) return fail(400, checked.error, origin);
    const att = checkAttachment(input.attachment, input.attachmentInfo);
    if (att.error) return fail(400, att.error, origin);
    if (att.envelope) checked.attachment = { id: newId(checked.test), files: att.files, bytes: att.bytes };
    let issue = makeIssue(checked);

    // a filled-in hidden field: a bot. Answer as if it worked, open nothing.
    if (checked.honeypot) return reply(200, { ok: true, number: 0, url: '' }, origin);

    try {
      const token = await installationToken(env);   // in test mode too: proves the key and the app's install work
      // the files first (a test send's for an hour), so the issue never names files that aren't there. When they can't
      // be stored (the free plan's 1,000 writes a day, or KV is down), the feedback still goes: the issue says the
      // files were sent but not kept, instead of the whole send failing.
      let filesStored = null;
      if (att.envelope) {
        try {
          await storeAttachment(env, checked.attachment.id, att.envelope, checked.test);
          filesStored = true;
        } catch (e) {
          console.error(`attachment not stored: ${String(e && e.message || e)}`);
          checked.attachment = { failed: true, files: att.files, bytes: att.bytes };
          issue = makeIssue(checked);
          filesStored = false;
        }
      }
      if (checked.test) return reply(200, { ok: true, test: true, issue, filesStored, attachment: filesStored ? checked.attachment.id : null }, origin);
      const res = await gh(`https://api.github.com/repos/${env.REPO}/issues`, token, { method: 'POST', body: JSON.stringify(issue) });
      if (!res.ok) throw new Error(`GitHub said ${res.status} to the new issue: ${(await res.text()).slice(0, 300)}`);
      const made = await res.json();
      return reply(200, { ok: true, number: made.number, url: made.html_url, ...(filesStored === null ? {} : { filesStored }) }, origin);
    } catch (e) {
      console.error(String(e && e.stack || e));
      return fail(502, 'The feedback service could not reach GitHub just now. Try again in a few minutes.', origin);
    }
  },
};
