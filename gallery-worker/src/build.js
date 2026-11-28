// The gate for a published build: the same rules as BABEL's own check (crates\imagine\src\build\community.rs, `check`;
// the format is written at the top of that file). test\builds.json holds cases both run, so the two stay the same.
//
// check(build) -> { build } a clean copy (only the known fields, in a fixed order), or { error } saying what is wrong.
// Whatever reaches the gallery may come from anywhere: only the copy is ever stored.

export const VERSION = 1;
export const MAX_BYTES = 16 * 1024;
export const MAX_NAME = 24;
export const MAX_TITLE = 60;
export const MAX_NOTE = 500;
export const STATS = ['str', 'mag', 'vit', 'int', 'spd', 'luck', 'clsr', 'lngr', 'spell', 'support', 'pdef', 'mdef'];
export const ALIGNMENTS = ['NEUTRAL', 'LAW', 'CHAOS'];

const SLOTS = 15;
const MAX_NUMBER = (1 << 18) - 1;
const MAX_EXPERTISE_ID = 127;
const MAX_EXPERTISES = 63;
const MAX_POINTS = 1_000_000;
const MAX_CAP = (1 << 24) - 1;
const MAX_CODE_CHARS = 1024;
const MAX_ID = 1_000_000;
const MAX_STAT = 99_999;
const MAX_FUSE = 50;
const MAX_TOGGLES = 64;

class Bad extends Error {}
const bad = (why) => { throw new Bad(why); };

export function check(v) {
  try {
    const size = new TextEncoder().encode(JSON.stringify(v)).length;
    if (size > MAX_BYTES) bad(`${size} bytes, more than ${MAX_BYTES}`);
    const o = object(v, 'the build');
    if (o.v !== VERSION) bad('not a version 1 build');
    if (o.kind === 'player') return { build: player(o) };
    if (o.kind === 'demon') return { build: demonBuild(o) };
    bad('kind is neither player nor demon');
  } catch (e) {
    if (e instanceof Bad) return { error: e.message };
    throw e;
  }
}

function player(o) {
  keys(o, ['v', 'kind', 'takenAt', 'level', 'alignment', 'maxHp', 'maxMp', 'statsLive', 'stats', 'gear', 'gearPartial', 'expertise', 'cap', 'toggles', 'code', 'partner', 'digitalize'], ['about'], 'the build');
  time(o.takenAt, 'takenAt');
  int(o.level, 1, 99, 'level');
  if (!ALIGNMENTS.includes(o.alignment)) bad("alignment is not one of the game's");
  int(o.maxHp, 0, MAX_STAT, 'maxHp');
  int(o.maxMp, 0, MAX_STAT, 'maxMp');
  boolean(o.statsLive, 'statsLive');
  const st = object(o.stats, 'stats');
  keys(st, STATS, [], 'stats');
  const stats = {};
  for (const s of STATS) {
    const pair = list(st[s], 2, 2, s);
    stats[s] = [int(pair[0], 0, MAX_STAT, s), int(pair[1], -MAX_STAT, MAX_STAT, s)];
  }
  let lastSlot = -1;
  const gear = list(o.gear, 0, SLOTS, 'gear').map((p) => {
    p = object(p, 'a piece');
    keys(p, ['slot', 'id', 'basic', 'special', 'tarot', 'soul', 'fuse', 'mods'], [], 'a piece');
    const slot = int(p.slot, 0, SLOTS - 1, "a piece's slot");
    if (slot <= lastSlot) bad('pieces not in slot order, or two in one slot');
    lastSlot = slot;
    return {
      slot,
      id: int(p.id, 1, MAX_NUMBER, "a piece's id"),
      basic: int(p.basic, 1, MAX_NUMBER, "a piece's id"),
      special: int(p.special, 1, MAX_NUMBER, "a piece's id"),
      tarot: int(p.tarot, 0, MAX_NUMBER, "a piece's crystal"),
      soul: int(p.soul, 0, MAX_NUMBER, "a piece's crystal"),
      fuse: ints(p.fuse, 3, 0, MAX_FUSE, "a piece's fusion bonuses"),
      mods: ints(p.mods, 5, 0, 0xFFFF, "a piece's mods"),
    };
  });
  boolean(o.gearPartial, 'gearPartial');
  let lastId = -1;
  const expertise = list(o.expertise, 0, MAX_EXPERTISES, 'expertise').map((e) => {
    const pair = list(e, 2, 2, 'an expertise');
    const id = int(pair[0], 0, MAX_EXPERTISE_ID, 'an expertise id');
    if (id <= lastId) bad('expertises not in id order');
    lastId = id;
    return [id, int(pair[1], 1, MAX_POINTS, 'expertise points')];
  });
  const cap = o.cap === null ? null : int(o.cap, 0, MAX_CAP, 'cap');
  const toggles = list(o.toggles, 0, MAX_TOGGLES, 'toggles').map((t) => int(t, 1, MAX_ID, 'a switch skill'));
  if (!isCode(o.code)) bad('code is not a build code');
  const partner = o.partner === null ? null : demon(o.partner, 'partner');
  const digitalize = o.digitalize === null ? null : demon(o.digitalize, 'digitalize');
  const out = {
    v: VERSION, kind: 'player', takenAt: o.takenAt, level: o.level, alignment: o.alignment, maxHp: o.maxHp, maxMp: o.maxMp,
    statsLive: o.statsLive, stats, gear, gearPartial: o.gearPartial, expertise, cap, toggles, code: o.code, partner, digitalize,
  };
  return withAbout(o, out);
}

function demonBuild(o) {
  keys(o, ['v', 'kind', 'seenAt', 'demon'], ['about'], 'the build');
  if (o.seenAt !== null) time(o.seenAt, 'seenAt');
  return withAbout(o, { v: VERSION, kind: 'demon', seenAt: o.seenAt, demon: demon(o.demon, 'demon') });
}

function demon(v, what) {
  const d = object(v, what);
  keys(d, ['type', 'level', 'growthType', 'reunion', 'mitamaType', 'mitamaRank', 'mitamaSlots', 'skills', 'forceStackIds', 'equipmentIds', 'stats'], [], what);
  return {
    type: int(d.type, 1, MAX_ID, 'a demon type'),
    level: int(d.level, 1, 99, 'a demon level'),
    growthType: int(d.growthType, 0, MAX_ID, 'a growth type'),
    reunion: ints(d.reunion, 12, 0, 100, 'rebirth ranks'),
    mitamaType: int(d.mitamaType, 0, 16, 'a Mitama type'),
    mitamaRank: int(d.mitamaRank, 0, 100, 'a Mitama rank'),
    mitamaSlots: ints(d.mitamaSlots, 96, 0, MAX_ID, 'Mitama slots'),
    skills: ints(d.skills, 8, 0, MAX_ID, 'demon skills'),
    forceStackIds: ints(d.forceStackIds, 8, 0, MAX_ID, 'the force stack'),
    equipmentIds: ints(d.equipmentIds, 4, 0, MAX_ID, 'demon equipment'),
    stats: ints(d.stats, 14, 0, MAX_STAT, 'demon stats'),
  };
}

function withAbout(o, out) {
  if (!('about' in o)) return out;
  const a = object(o.about, 'about');
  keys(a, ['name', 'title', 'note'], [], 'about');
  out.about = {
    name: text(a.name, MAX_NAME, false, 'the display name'),
    title: text(a.title, MAX_TITLE, false, 'the title'),
    note: text(a.note, MAX_NOTE, true, 'the note'),
  };
  if (!out.about.name.trim()) bad('the display name is empty');
  return out;
}

function object(v, what) {
  if (v === null || typeof v !== 'object' || Array.isArray(v)) bad(`${what} is not an object`);
  return v;
}

function keys(o, must, may, what) {
  const k = must.find((k) => !Object.hasOwn(o, k));
  if (k) bad(`${what} has no ${k}`);
  const extra = Object.keys(o).find((k) => !must.includes(k) && !may.includes(k));
  if (extra !== undefined) bad(`${what} has an unknown field (${[...extra].length} characters)`);
}

function int(v, lo, hi, what) {
  if (!Number.isSafeInteger(v) || v < lo || v > hi) bad(`${what} is not a whole number from ${lo} to ${hi}`);
  return v;
}

function ints(v, n, lo, hi, what) {
  return list(v, n, n, what).map((x) => int(x, lo, hi, what));
}

function list(v, min, max, what) {
  if (!Array.isArray(v) || v.length < min || v.length > max) bad(`${what} is not a list of ${min} to ${max}`);
  return v;
}

function boolean(v, what) {
  if (typeof v !== 'boolean') bad(`${what} is not true or false`);
}

function time(v, what) {
  if (typeof v !== 'string' || !/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}$/.test(v)) bad(`${what} is not a time`);
}

// plain text the player typed: no control characters (line breaks only in the note), no text-direction marks
function text(v, max, lines, what) {
  if (typeof v !== 'string') bad(`${what} is not text`);
  if ([...v].length > max) bad(`${what} is longer than ${max} characters`);
  const control = lines ? /[\p{Cc}--\n]/v : /\p{Cc}/u;
  if (control.test(v) || /[‪-‮⁦-⁩]/u.test(v)) bad(`${what} has characters that aren't text`);
  return v;
}

// A Build Planner code: "BABEL1-" + base64url, its first two bytes a CRC-16/CCITT-FALSE of the rest. The app decodes
// it fully when a player imports it (crates\imagine\src\build\share_code.rs); here only its shape and checksum.
export function isCode(c) {
  if (typeof c !== 'string' || c.length > MAX_CODE_CHARS || !c.startsWith('BABEL1-')) return false;
  const body = c.slice(7);
  if (!/^[A-Za-z0-9_-]{4,}$/.test(body) || body.length % 4 === 1) return false;
  const bytes = base64url(body);
  if (bytes.length < 3) return false;
  return ((bytes[0] << 8) | bytes[1]) === crc16(bytes.subarray(2));
}

function base64url(s) {
  const bin = atob(s.replace(/-/g, '+').replace(/_/g, '/') + '='.repeat((4 - (s.length % 4)) % 4));
  return Uint8Array.from(bin, (ch) => ch.charCodeAt(0));
}

function crc16(bytes) {
  let crc = 0xFFFF;
  for (const b of bytes) {
    crc ^= b << 8;
    for (let i = 0; i < 8; i++) crc = crc & 0x8000 ? ((crc << 1) ^ 0x1021) & 0xFFFF : (crc << 1) & 0xFFFF;
  }
  return crc;
}

// What the lists show and filter on, so the gallery page needs no build body for them.
export function facets(b) {
  if (b.kind === 'demon') return { type: b.demon.type, mitama: b.demon.mitamaType };
  const top = [...b.expertise].sort((x, y) => y[1] - x[1] || x[0] - y[0]).slice(0, 3).map((e) => e[0]);
  return {
    weapon: b.gear.find((p) => p.slot === 0)?.id ?? null,
    partner: b.partner?.type ?? null,
    digitalize: b.digitalize?.type ?? null,
    expertise: top,
  };
}
