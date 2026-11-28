// node --test test/   (no Cloudflare needed: D1 is stood in for by Node's SQLite, with the real schema)
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import worker from '../src/index.js';
import { AUTO_HIDE, CREATES_PER_DAY, newId } from '../src/gallery.js';
import { check, facets, isCode } from '../src/build.js';
import { fakeD1 } from './fake-d1.mjs';

const cases = JSON.parse(readFileSync(new URL('./builds.json', import.meta.url), 'utf8'));
const good = (kind) => structuredClone(cases.good[kind]);

function setAt(v, pointer, to) {
  const parts = pointer.split('/').slice(1);
  const last = parts.pop();
  let o = v;
  for (const p of parts) o = o[p];
  o[last] = to;
  return v;
}

// ---------- the gate ----------

test('the good builds pass, and come back the same', () => {
  for (const kind of ['player', 'demon']) {
    const c = check(good(kind));
    assert.equal(c.error, undefined, kind);
    assert.deepEqual(c.build, good(kind));
  }
});

test('every bad case is refused', () => {
  for (const b of cases.bad) {
    const c = check(setAt(good(b.base), b.path, b.to));
    assert.ok(c.error, `not refused: ${b.why}`);
  }
});

test('the copy holds only known fields, in a fixed order', () => {
  const p = good('player');
  const shuffled = Object.fromEntries(Object.entries(p).reverse());
  assert.deepEqual(Object.keys(check(shuffled).build), Object.keys(p));
});

test('build codes: shape and checksum', () => {
  const code = good('player').code;
  assert.ok(isCode(code));
  // one letter changed in the middle (the last letter can carry only padding bits)
  assert.ok(!isCode(code.slice(0, 20) + (code[20] === 'A' ? 'B' : 'A') + code.slice(21)));
  assert.ok(!isCode('BABEL2-' + code.slice(7)));
  assert.ok(!isCode(code + 'A'.repeat(1100)));
});

test('facets: what the lists filter on', () => {
  const f = facets(good('player'));
  assert.equal(f.weapon, good('player').gear[0].id);
  assert.equal(f.digitalize, good('player').digitalize.type);
  assert.equal(f.expertise.length, 3);
  assert.deepEqual(facets(good('demon')), { type: good('demon').demon.type, mitama: good('demon').demon.mitamaType });
});

test('ids: 10 letters and digits', () => {
  const ids = new Set(Array.from({ length: 500 }, newId));
  assert.equal(ids.size, 500);
  for (const id of ids) assert.match(id, /^[A-Za-z0-9]{10}$/);
});

// ---------- the service ----------

function service() {
  const env = { DB: fakeD1(), ADMIN_KEY: 'admin-key-for-tests', IP_SALT: 'salt' };
  const call = async (method, path, { body, secret, ip = '1.2.3.4', origin } = {}) => {
    const headers = { 'CF-Connecting-IP': ip };
    if (body !== undefined) headers['Content-Type'] = 'application/json';
    if (secret) headers.Authorization = `Bearer ${secret}`;
    if (origin) headers.Origin = origin;
    const res = await worker.fetch(new Request(`https://gallery.test${path}`, { method, headers, body: body === undefined ? undefined : typeof body === 'string' ? body : JSON.stringify(body) }), env);
    return { status: res.status, headers: res.headers, data: res.headers.get('Content-Type')?.includes('json') ? await res.json() : await res.text() };
  };
  return { env, call };
}

test('publish, list, show, update, delete', async () => {
  const { call } = service();
  const made = await call('POST', '/builds', { body: good('player'), origin: 'https://imagine.app' });
  assert.equal(made.status, 201, JSON.stringify(made.data));
  assert.match(made.data.id, /^[A-Za-z0-9]{10}$/);
  assert.equal(made.headers.get('Access-Control-Allow-Origin'), 'https://imagine.app');
  const { id, secret } = made.data;

  const list = await call('GET', '/builds?kind=player');
  assert.equal(list.status, 200);
  assert.equal(list.headers.get('Access-Control-Allow-Origin'), '*');
  assert.equal(list.data.builds.length, 1);
  const s = list.data.builds[0];
  assert.deepEqual(Object.keys(s), ['id', 'kind', 'name', 'title', 'level', 'created', 'updated', 'facets']);
  assert.equal(s.name, 'Rubito');
  assert.equal(s.level, 95);
  assert.match(s.updated, /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}Z$/);
  assert.equal((await call('GET', '/builds?kind=demon')).data.builds.length, 0);

  const shown = await call('GET', `/builds/${id}`);
  assert.deepEqual(shown.data.build, good('player'));
  assert.ok(!JSON.stringify(shown.data).includes(secret), 'the secret never comes back');

  // only with the secret
  const changed = { ...good('player'), about: { name: 'Rubito', title: 'New title', note: '' } };
  assert.equal((await call('PUT', `/builds/${id}`, { body: changed })).status, 403);
  assert.equal((await call('PUT', `/builds/${id}`, { body: changed, secret: 'x'.repeat(43) })).status, 403);
  assert.equal((await call('PUT', `/builds/${id}`, { body: changed, secret })).status, 200);
  assert.equal((await call('GET', `/builds/${id}`)).data.build.about.title, 'New title');
  // a player build stays one
  assert.equal((await call('PUT', `/builds/${id}`, { body: good('demon'), secret })).status, 400);

  assert.equal((await call('DELETE', `/builds/${id}`)).status, 403);
  assert.equal((await call('DELETE', `/builds/${id}`, { secret })).status, 200);
  assert.equal((await call('GET', `/builds/${id}`)).status, 404);
});

test('what is stored is the clean copy; bad builds and other pages are turned away', async () => {
  const { env, call } = service();
  const r = await call('POST', '/builds', { body: setAt(good('player'), '/dataDir', 'C:\\Users\\x') });
  assert.equal(r.status, 400);
  assert.match(r.data.error, /unknown field/);
  assert.equal((await call('POST', '/builds', { body: 'not json' })).status, 400);
  const noAbout = good('demon');
  delete noAbout.about;
  assert.match((await call('POST', '/builds', { body: noAbout })).data.error, /display name/);
  assert.equal((await call('POST', '/builds', { body: { ...good('demon'), pad: 'x'.repeat(50000) } })).status, 413);
  // only the app's page (or no browser) may publish: not the website, not another site
  assert.equal((await call('POST', '/builds', { body: good('demon'), origin: 'https://shinbabel.pages.dev' })).status, 403);
  assert.equal((await call('POST', '/builds', { body: good('demon'), origin: 'https://evil.example' })).status, 403);
  assert.equal((await call('OPTIONS', '/builds', { origin: 'https://evil.example' })).status, 403);
  assert.equal((await call('OPTIONS', '/builds', { origin: 'https://imagine.app' })).status, 204);
  assert.equal(env.DB.db.prepare('SELECT COUNT(*) AS n FROM builds').get().n, 0);
  // the address is never stored as it is
  await call('POST', '/builds', { body: good('demon'), ip: '9.9.9.9' });
  const dump = JSON.stringify([env.DB.db.prepare('SELECT * FROM builds').all(), env.DB.db.prepare('SELECT * FROM creates').all()]);
  assert.ok(!dump.includes('9.9.9.9'));
});

test(`at most ${CREATES_PER_DAY} new builds a day from one address`, async () => {
  const { call } = service();
  for (let i = 0; i < CREATES_PER_DAY; i++) assert.equal((await call('POST', '/builds', { body: good('demon') })).status, 201);
  assert.equal((await call('POST', '/builds', { body: good('demon') })).status, 429);
  assert.equal((await call('POST', '/builds', { body: good('demon'), ip: '5.6.7.8' })).status, 201);
});

test('reports: one per person, a few hide the build, the developer shows it again', async () => {
  const { call } = service();
  const { id } = (await call('POST', '/builds', { body: good('demon') })).data;
  assert.equal((await call('POST', `/builds/${id}/report`, { body: { reason: 'nope' } })).status, 400);
  assert.equal((await call('POST', `/builds/${id}/report`, { body: { reason: 'spam' }, origin: 'https://evil.example' })).status, 403);
  for (let i = 0; i < AUTO_HIDE - 1; i++) {
    assert.equal((await call('POST', `/builds/${id}/report`, { body: { reason: 'rude' }, ip: '10.0.0.1', origin: 'https://shinbabel.pages.dev' })).status, 200);
  }
  assert.equal((await call('GET', `/builds/${id}`)).status, 200, 'the same person twice counts once');
  for (let i = 2; i <= AUTO_HIDE; i++) await call('POST', `/builds/${id}/report`, { body: { reason: 'spam' }, ip: `10.0.0.${i}`, origin: 'https://x.shinbabel.pages.dev' });
  assert.equal((await call('GET', `/builds/${id}`)).status, 404, 'hidden after enough reports');
  assert.equal((await call('GET', '/builds?kind=demon')).data.builds.length, 0);

  // the developer
  assert.equal((await call('GET', '/admin/reports')).status, 404);
  assert.equal((await call('GET', '/admin/reports', { secret: 'wrong' })).status, 404);
  const rep = await call('GET', '/admin/reports', { secret: 'admin-key-for-tests' });
  assert.deepEqual(rep.data.builds.map((b) => [b.id, b.hidden, b.reports]), [[id, true, { rude: 1, spam: AUTO_HIDE - 1 }]]);
  assert.equal((await call('POST', `/admin/builds/${id}`, { body: { hidden: false }, secret: 'admin-key-for-tests' })).status, 200);
  assert.equal((await call('GET', `/builds/${id}`)).status, 200);
  // the old reports were seen: one more does not hide it again
  await call('POST', `/builds/${id}/report`, { body: { reason: 'spam' }, ip: '10.0.1.1' });
  assert.equal((await call('GET', `/builds/${id}`)).status, 200);
  assert.equal((await call('DELETE', `/admin/builds/${id}`, { secret: 'admin-key-for-tests' })).status, 200);
  assert.equal((await call('GET', `/builds/${id}`)).status, 404);
});

test('a list is kept a little while, and dropped when this instance changes something', async () => {
  const { env, call } = service();
  let reads = 0;
  const prepare = env.DB.prepare;
  env.DB.prepare = (sql) => {
    if (sql.startsWith('SELECT id, kind, name')) reads++;
    return prepare(sql);
  };
  await call('GET', '/builds?kind=demon');
  await call('GET', '/builds?kind=demon');
  assert.equal(reads, 1, 'the second list came from the kept copy');
  const { id } = (await call('POST', '/builds', { body: good('demon') })).data;
  const after = await call('GET', '/builds?kind=demon');
  assert.equal(reads, 2);
  assert.deepEqual(after.data.builds.map((b) => b.id), [id]);
  // a turned-down request changes nothing, so the copy stays
  await call('POST', '/builds', { body: 'not json' });
  await call('GET', '/builds?kind=demon');
  assert.equal(reads, 2);
});

test('several builds at once, for one page of cards', async () => {
  const { call } = service();
  const a = (await call('POST', '/builds', { body: good('player') })).data.id;
  const b = (await call('POST', '/builds', { body: good('demon') })).data.id;
  const r = await call('GET', `/builds?ids=${a},${b},AAAAAAAAAA`);
  assert.equal(r.status, 200);
  assert.deepEqual(r.data.builds.map((x) => x.id).sort(), [a, b].sort());
  assert.deepEqual(r.data.builds.find((x) => x.id === a).build, good('player'));
  assert.equal((await call('GET', '/builds?ids=')).status, 400);
  assert.equal((await call('GET', '/builds?ids=bad!')).status, 400);
  assert.equal((await call('GET', `/builds?ids=${Array.from({ length: 101 }, (_, i) => String(i).padStart(10, 'A')).join(',')}`)).status, 400);
});

test('not for search engines', async () => {
  const { call } = service();
  assert.equal((await call('GET', '/builds?kind=player')).headers.get('X-Robots-Tag'), 'noindex, nofollow');
  assert.match((await call('GET', '/robots.txt')).data, /Disallow: \//);
  assert.equal((await call('GET', '/builds?kind=items')).status, 400);
  assert.equal((await call('GET', '/nothing')).status, 404);
});
