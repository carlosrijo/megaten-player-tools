// BABEL Community Builds (Cloudflare Worker + D1; the entry is index.js). Players publish builds from BABEL; the website lists and shows them.
//
// Publishing (from the app's page, https://imagine.app):
//   POST   /builds            body: a build (src/build.js)       -> 201 { ok, id, secret }
//   PUT    /builds/<id>       Authorization: Bearer <secret>       -> { ok, id }     (same kind as before)
//   DELETE /builds/<id>       Authorization: Bearer <secret>       -> { ok }
// Reading (anyone):
//   GET    /builds?kind=player|demon                              -> { ok, builds: [ { id, kind, name, title, level, created, updated, facets } ] }
//   GET    /builds?ids=<id>,<id>,...  (up to 100)                     -> { ok, builds: [ { id, kind, created, updated, build } ] }   (the cards of one page)
//   GET    /builds/<id>                                           -> { ok, id, kind, created, updated, build }
//   POST   /builds/<id>/report  { reason }                        -> { ok }   (from the website or the app)
// The developer (Authorization: Bearer <ADMIN_KEY>, a Worker secret):
//   GET    /admin/reports                                         -> { ok, builds: [ ...summary, hidden, reports: { reason: count } ] }
//   POST   /admin/builds/<id>  { hidden: true|false }             -> { ok }
//   DELETE /admin/builds/<id>                                     -> { ok }
//
// The secret is made here and only its SHA-256 is kept: BABEL keeps the secret, so only the app that published a
// build can change or delete it. A build is stored as check() cleaned it, never as sent. Addresses are never stored:
// the daily limits and one-report-per-person use a keyed hash of the address (IP_SALT, a Worker secret).

import { check, facets } from './build.js';

export const MAX_REQUEST = 40 * 1024;
export const MAX_BUILDS = 5000;            // visible builds of each kind; past it new ones wait for room
export const CREATES_PER_DAY = 10;         // new builds from one address in 24 hours
export const AUTO_HIDE = 3;                // reports from different people that hide a build until it is checked
export const REASONS = ['spam', 'rude', 'fake', 'other'];
const DAY = 24 * 3600 * 1000;

const APP_ORIGIN = 'https://imagine.app';

// Which browser pages may send: the app's page for publishing; the website too for reports. (Anyone can still call
// without a browser; the checks, limits and secrets are what keep it in hand.)
function siteOrigins(env) {
  return String(env.SITE_ORIGINS || 'https://babel-7vg.pages.dev').split(',').map((s) => s.trim()).filter(Boolean);
}
function local(origin) {
  return /^http:\/\/(localhost|127\.0\.0\.1)(:\d+)?$/.test(origin);
}
function siteOrigin(origin, env) {
  return siteOrigins(env).some((s) => origin === s || (origin.endsWith('.' + new URL(s).host) && origin.startsWith('https://')));
}

function reply(status, data, headers = {}) {
  return new Response(JSON.stringify(data), {
    status,
    headers: { 'Content-Type': 'application/json; charset=utf-8', 'Cache-Control': 'no-store', 'X-Robots-Tag': 'noindex, nofollow', ...headers },
  });
}

export default {
  async fetch(request, env) {
    const origin = request.headers.get('Origin') || '';
    const res = await route(request, env, origin).catch((e) => {
      console.error(e && e.stack ? e.stack : String(e));
      return reply(500, { ok: false, error: 'Something went wrong in the gallery. Try again in a minute.' });
    });
    // anything that went through may have changed a list (a new build, a hidden one, a new title)
    if (request.method !== 'GET' && res.status < 400) forget(env);
    // reading is open to every page; sending only to the app's page (and the website, for reports) or no browser
    const h = new Headers(res.headers);
    const reading = request.method === 'GET';
    if (reading) h.set('Access-Control-Allow-Origin', '*');
    else if (origin && (origin === APP_ORIGIN || local(origin) || siteOrigin(origin, env))) {
      h.set('Access-Control-Allow-Origin', origin);
      h.set('Vary', 'Origin');
    }
    return new Response(res.body, { status: res.status, headers: h });
  },
};

async function route(request, env, origin) {
  const url = new URL(request.url);
  const path = url.pathname.replace(/\/+$/, '') || '/';
  const m = request.method;

  if (m === 'OPTIONS') {
    if (!origin || !(origin === APP_ORIGIN || local(origin) || siteOrigin(origin, env))) return new Response(null, { status: 403 });
    return new Response(null, {
      status: 204,
      headers: {
        'Access-Control-Allow-Origin': origin,
        'Access-Control-Allow-Methods': 'GET, POST, PUT, DELETE, OPTIONS',
        'Access-Control-Allow-Headers': 'Content-Type, Authorization',
        'Access-Control-Max-Age': '86400',
        'Vary': 'Origin',
      },
    });
  }
  if (path === '/' && m === 'GET') {
    return new Response('BABEL Community Builds. The app publishes to /builds; the website reads them.\n', { headers: { 'Content-Type': 'text/plain', 'X-Robots-Tag': 'noindex, nofollow' } });
  }
  if (path === '/robots.txt') return new Response('User-agent: *\nDisallow: /\n', { headers: { 'Content-Type': 'text/plain' } });

  // a browser page other than the app's may not publish, change or delete
  const fromApp = !origin || origin === APP_ORIGIN || local(origin);

  if (path === '/builds') {
    if (m === 'GET') return url.searchParams.has('ids') ? many(env, url.searchParams.get('ids')) : list(env, url.searchParams.get('kind'));
    if (m === 'POST') return fromApp ? create(request, env) : reply(403, { ok: false, error: 'Builds are published from BABEL.' });
    return reply(405, { ok: false, error: 'Use GET or POST.' });
  }
  let mm = /^\/builds\/([A-Za-z0-9]{10})(\/report)?$/.exec(path);
  if (mm) {
    const [, id, report] = mm;
    if (report) return m === 'POST' ? reportBuild(request, env, id) : reply(405, { ok: false, error: 'Use POST.' });
    if (m === 'GET') return show(env, id);
    if (!fromApp) return reply(403, { ok: false, error: 'Builds are changed from BABEL.' });
    if (m === 'PUT') return update(request, env, id);
    if (m === 'DELETE') return remove(request, env, id);
    return reply(405, { ok: false, error: 'Use GET, PUT or DELETE.' });
  }
  if (path.startsWith('/admin/')) return admin(request, env, path);
  return reply(404, { ok: false, error: 'Not found.' });
}

// ---------- reading ----------

// A list reads every visible build of its kind, and the free plan allows 5 million rows read a day: so each Worker
// instance keeps the answer for LIST_SECONDS (one read per instance and kind in that time, not one per visitor). A
// change made through this instance drops its copy at once; other instances catch up within LIST_SECONDS.
export const LIST_SECONDS = 30;
const lists = new WeakMap();   // env.DB -> Map(kind -> { at, text })

function forget(env) {
  lists.delete(env.DB);
}

async function list(env, kind) {
  if (kind !== 'player' && kind !== 'demon') return reply(400, { ok: false, error: 'Ask for kind=player or kind=demon.' });
  const headers = { 'Cache-Control': `public, max-age=${LIST_SECONDS}` };
  const kept = lists.get(env.DB)?.get(kind);
  if (kept && Date.now() - kept.at < LIST_SECONDS * 1000) return reply(200, JSON.parse(kept.text), headers);
  const { results } = await env.DB.prepare(
    'SELECT id, kind, name, title, level, created, updated, facets FROM builds WHERE kind = ? AND hidden = 0 ORDER BY updated DESC LIMIT ?',
  ).bind(kind, MAX_BUILDS).all();
  const builds = results.map((r) => ({ ...r, created: iso(r.created), updated: iso(r.updated), facets: JSON.parse(r.facets) }));
  const data = { ok: true, builds };
  if (!lists.has(env.DB)) lists.set(env.DB, new Map());
  lists.get(env.DB).set(kind, { at: Date.now(), text: JSON.stringify(data) });
  return reply(200, data, headers);
}

async function show(env, id) {
  const r = await env.DB.prepare('SELECT id, kind, created, updated, body FROM builds WHERE id = ? AND hidden = 0').bind(id).first();
  if (!r) return reply(404, { ok: false, error: 'That build is not in the gallery (any more).' });
  return reply(200, { ok: true, id: r.id, kind: r.kind, created: iso(r.created), updated: iso(r.updated), build: JSON.parse(r.body) }, { 'Cache-Control': 'public, max-age=60' });
}

// The builds of one page of cards at once (a card draws the build itself: its pieces, its demon's bars). Hidden ones and
// ids that aren't there are left out.
export const MAX_IDS = 100;
async function many(env, text) {
  const ids = [...new Set(String(text || '').split(',').map((s) => s.trim()).filter(Boolean))];
  if (!ids.length || ids.length > MAX_IDS || ids.some((id) => !/^[A-Za-z0-9]{10}$/.test(id))) return reply(400, { ok: false, error: `Ask for 1 to ${MAX_IDS} build ids.` });
  const { results } = await env.DB.prepare(`SELECT id, kind, created, updated, body FROM builds WHERE hidden = 0 AND id IN (${ids.map(() => '?').join(', ')})`).bind(...ids).all();
  const builds = results.map((r) => ({ id: r.id, kind: r.kind, created: iso(r.created), updated: iso(r.updated), build: JSON.parse(r.body) }));
  return reply(200, { ok: true, builds }, { 'Cache-Control': 'public, max-age=60' });
}

// ---------- publishing ----------

async function readBuild(request) {
  const length = Number(request.headers.get('Content-Length') || 0);
  if (length > MAX_REQUEST) return { error: [413, 'That build is too big.'] };
  let text;
  try {
    text = await request.text();
  } catch {
    return { error: [400, 'The gallery could not read what was sent.'] };
  }
  if (text.length > MAX_REQUEST) return { error: [413, 'That build is too big.'] };
  let input;
  try {
    input = JSON.parse(text);
  } catch {
    return { error: [400, 'The gallery could not read what was sent.'] };
  }
  const c = check(input);
  if (c.error) return { error: [400, `This build can't be published: ${c.error}. Update BABEL, then try again.`] };
  if (!c.build.about) return { error: [400, 'A build needs a display name.'] };
  return { build: c.build };
}

async function limited(request, env) {
  if (!env.PER_IP) return false;
  const { success } = await env.PER_IP.limit({ key: request.headers.get('CF-Connecting-IP') || 'unknown' });
  return !success;
}

async function create(request, env) {
  if (await limited(request, env)) return reply(429, { ok: false, error: 'That was a lot at once. Wait a minute, then try again.' });
  const r = await readBuild(request);
  if (r.error) return reply(r.error[0], { ok: false, error: r.error[1] });
  const b = r.build;
  const now = Date.now();
  const who = await addressHash(request, env);
  await env.DB.prepare('DELETE FROM creates WHERE at < ?').bind(now - DAY).run();
  const made = await env.DB.prepare('SELECT COUNT(*) AS n FROM creates WHERE who = ?').bind(who).first();
  if (made.n >= CREATES_PER_DAY) return reply(429, { ok: false, error: `That's ${CREATES_PER_DAY} new builds today from here. Update one of yours instead, or try again tomorrow.` });
  const count = await env.DB.prepare('SELECT COUNT(*) AS n FROM builds WHERE kind = ? AND hidden = 0').bind(b.kind).first();
  if (count.n >= MAX_BUILDS) return reply(503, { ok: false, error: 'The gallery is full for now. Try again later.' });

  const id = newId();
  const secret = newSecret();
  await env.DB.batch([
    env.DB.prepare('INSERT INTO builds (id, kind, secret_hash, name, title, level, created, updated, facets, body) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)')
      .bind(id, b.kind, await sha256(secret), b.about.name.trim(), b.about.title.trim(), levelOf(b), now, now, JSON.stringify(facets(b)), JSON.stringify(b)),
    env.DB.prepare('INSERT INTO creates (who, at) VALUES (?, ?)').bind(who, now),
  ]);
  return reply(201, { ok: true, id, secret });
}

async function owned(request, env, id) {
  const auth = /^Bearer ([A-Za-z0-9_-]{20,100})$/.exec(request.headers.get('Authorization') || '');
  const row = await env.DB.prepare('SELECT id, kind, secret_hash FROM builds WHERE id = ?').bind(id).first();
  if (!row) return { error: [404, 'That build is not in the gallery (any more). Publish it again.'] };
  if (!auth || (await sha256(auth[1])) !== row.secret_hash) return { error: [403, 'Only the BABEL that published this build can change it.'] };
  return { row };
}

async function update(request, env, id) {
  if (await limited(request, env)) return reply(429, { ok: false, error: 'That was a lot at once. Wait a minute, then try again.' });
  const o = await owned(request, env, id);
  if (o.error) return reply(o.error[0], { ok: false, error: o.error[1] });
  const r = await readBuild(request);
  if (r.error) return reply(r.error[0], { ok: false, error: r.error[1] });
  const b = r.build;
  if (b.kind !== o.row.kind) return reply(400, { ok: false, error: 'A player build stays a player build, and a demon build a demon build.' });
  await env.DB.prepare('UPDATE builds SET name = ?, title = ?, level = ?, updated = ?, facets = ?, body = ? WHERE id = ?')
    .bind(b.about.name.trim(), b.about.title.trim(), levelOf(b), Date.now(), JSON.stringify(facets(b)), JSON.stringify(b), id).run();
  return reply(200, { ok: true, id });
}

async function remove(request, env, id) {
  if (await limited(request, env)) return reply(429, { ok: false, error: 'That was a lot at once. Wait a minute, then try again.' });
  const o = await owned(request, env, id);
  if (o.error) return reply(o.error[0], { ok: false, error: o.error[1] });
  await env.DB.batch([env.DB.prepare('DELETE FROM reports WHERE build = ?').bind(id), env.DB.prepare('DELETE FROM builds WHERE id = ?').bind(id)]);
  return reply(200, { ok: true });
}

// ---------- reports ----------

async function reportBuild(request, env, id) {
  const origin = request.headers.get('Origin') || '';
  if (origin && !(origin === APP_ORIGIN || local(origin) || siteOrigin(origin, env))) return reply(403, { ok: false, error: 'Reports come from the website or BABEL.' });
  if (await limited(request, env)) return reply(429, { ok: false, error: 'That was a lot at once. Wait a minute, then try again.' });
  let input;
  try {
    const text = await request.text();
    if (text.length > 1000) return reply(413, { ok: false, error: 'Too much was sent.' });
    input = JSON.parse(text || '{}');
  } catch {
    return reply(400, { ok: false, error: 'The gallery could not read what was sent.' });
  }
  const reason = input && typeof input === 'object' ? input.reason : undefined;
  if (!REASONS.includes(reason)) return reply(400, { ok: false, error: `Pick a reason: ${REASONS.join(', ')}.` });
  const row = await env.DB.prepare('SELECT id, checked FROM builds WHERE id = ? AND hidden = 0').bind(id).first();
  if (!row) return reply(404, { ok: false, error: 'That build is not in the gallery (any more).' });
  const who = await addressHash(request, env);
  // one report per person and build: a second one changes nothing
  await env.DB.prepare('INSERT OR IGNORE INTO reports (build, who, reason, at) VALUES (?, ?, ?, ?)').bind(id, who, reason, Date.now()).run();
  const n = await env.DB.prepare('SELECT COUNT(*) AS n FROM reports WHERE build = ?').bind(id).first();
  if (n.n - row.checked >= AUTO_HIDE) await env.DB.prepare('UPDATE builds SET hidden = 1 WHERE id = ?').bind(id).run();
  return reply(200, { ok: true });
}

// ---------- the developer ----------

async function admin(request, env, path) {
  const auth = /^Bearer (.+)$/.exec(request.headers.get('Authorization') || '');
  if (!env.ADMIN_KEY || !auth || (await sha256(auth[1])) !== (await sha256(env.ADMIN_KEY))) return reply(404, { ok: false, error: 'Not found.' });
  const m = request.method;
  if (path === '/admin/reports' && m === 'GET') {
    const { results } = await env.DB.prepare(
      `SELECT b.id, b.kind, b.name, b.title, b.level, b.created, b.updated, b.hidden, b.checked, r.reason, COUNT(*) AS n
         FROM builds b JOIN reports r ON r.build = b.id GROUP BY b.id, r.reason ORDER BY b.hidden DESC, b.updated DESC`,
    ).all();
    const by = new Map();
    for (const r of results) {
      const e = by.get(r.id) || { id: r.id, kind: r.kind, name: r.name, title: r.title, level: r.level, created: iso(r.created), updated: iso(r.updated), hidden: !!r.hidden, checked: r.checked, reports: {} };
      e.reports[r.reason] = r.n;
      by.set(r.id, e);
    }
    return reply(200, { ok: true, builds: [...by.values()] });
  }
  const mm = /^\/admin\/builds\/([A-Za-z0-9]{10})$/.exec(path);
  if (mm) {
    const id = mm[1];
    if (m === 'DELETE') {
      await env.DB.batch([env.DB.prepare('DELETE FROM reports WHERE build = ?').bind(id), env.DB.prepare('DELETE FROM builds WHERE id = ?').bind(id)]);
      return reply(200, { ok: true });
    }
    if (m === 'POST') {
      let input;
      try {
        input = JSON.parse(await request.text());
      } catch {
        return reply(400, { ok: false, error: 'Send { hidden: true|false }.' });
      }
      if (typeof input?.hidden !== 'boolean') return reply(400, { ok: false, error: 'Send { hidden: true|false }.' });
      // shown again after a check: the reports so far are counted as seen, so it takes AUTO_HIDE new ones to hide it
      await env.DB.prepare('UPDATE builds SET hidden = ?, checked = (SELECT COUNT(*) FROM reports WHERE build = ?) WHERE id = ?').bind(input.hidden ? 1 : 0, id, id).run();
      return reply(200, { ok: true });
    }
  }
  return reply(404, { ok: false, error: 'Not found.' });
}

// ---------- helpers ----------

function levelOf(b) {
  return b.kind === 'demon' ? b.demon.level : b.level;
}

function iso(ms) {
  return new Date(ms).toISOString().slice(0, 16) + 'Z';
}

const ALPHABET = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789';

// 10 letters and digits (about 59 bits): too many to guess, short enough for a link
export function newId() {
  let id = '';
  while (id.length < 10) {
    for (const b of crypto.getRandomValues(new Uint8Array(16))) {
      if (b < 248 && id.length < 10) id += ALPHABET[b % 62];   // 248 = 4 * 62: no letter more likely than another
    }
  }
  return id;
}

function newSecret() {
  return btoa(String.fromCharCode(...crypto.getRandomValues(new Uint8Array(32)))).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '');
}

async function sha256(text) {
  const d = await crypto.subtle.digest('SHA-256', new TextEncoder().encode(text));
  return [...new Uint8Array(d)].map((b) => b.toString(16).padStart(2, '0')).join('');
}

// the address, never stored as it is: a keyed hash, the same for one address all day long
async function addressHash(request, env) {
  const ip = request.headers.get('CF-Connecting-IP') || 'unknown';
  return (await sha256(`${env.IP_SALT || ''}|${ip}`)).slice(0, 32);
}
