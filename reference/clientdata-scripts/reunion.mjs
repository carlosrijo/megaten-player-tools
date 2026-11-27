// Makes the rebirth (Reunion) and Mitama reference tables the Rebirth Planner's data is built from:
//   reference\reunion-growth.tsv   DevilLVUpRateData: every growth type of the 12 runes (groupID 1-12): its rank (subID),
//                                  name and the items that can pay for a rebirth into it (reunionConditions: any ONE of
//                                  them, in its amount); the rank-1 row carries the client's description of the rune
//                                  (CMessageData_reunion 215-226)
//   reference\level-xp.tsv         comp_hack LEVEL_XP_REQUIREMENTS: the XP from level L to L+1 (L = 0..98)
//   reference\correct-table.tsv    comp_hack CorrectTbl: id -> name (shared.xml), for stat ids the other tables use
//   reference\mitama-pg.tsv        NPCBarterData: the Summon PG: <kind> Mitama items and what each trade asks for
//
//   node reference\clientdata-scripts\reunion.mjs <smt-decoded\server> <comp_hack repo> [reference dir]
//
// Node only (no Python on this PC). Reads the decoded client tables (DevilLVUpRateData.xml, CMessageData_reunion.xml,
// NPCBarterData.xml) and the server source; writes nothing anywhere else.
import { readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

const [decoded, compHack, outDir = 'reference'] = process.argv.slice(2);
if (!decoded || !compHack) {
  console.error('usage: node reunion.mjs <smt-decoded\\server> <comp_hack repo> [reference dir]');
  process.exit(2);
}
const read = (p) => readFileSync(p, 'utf8');
const member = (o, n) => { const m = o.match(new RegExp(`<member name="${n}">([^<]*)<`)); return m ? m[1] : ''; };
const cdata = (o, n) => { const m = o.match(new RegExp(`<member name="${n}"><!\\[CDATA\\[([\\s\\S]*?)\\]\\]>`)); return m ? m[1] : ''; };
const objects = (xml, name) => xml.split(`<object name="${name}">`).slice(1);
const tsv = (rows) => rows.map((r) => r.join('\t')).join('\n') + '\n';

// ---- reunion-growth.tsv ----
const descriptions = new Map();   // rune index (0-11) -> the client's text, from CMessageData_reunion ids 215..226
for (const o of objects(read(join(decoded, 'CMessageData_reunion.xml')), 'MiCMessageData')) {
  const id = +member(o, 'ID');
  if (id >= 215 && id <= 226) descriptions.set(id - 215, cdata(o, 'message').replace(/\s*\r?\n\s*/g, ' ').trim());
}
const growth = [];
for (const o of objects(read(join(decoded, 'DevilLVUpRateData.xml')), 'MiDevilLVUpRateData')) {
  const group = +member(o, 'groupID');
  if (group < 1 || group > 12) continue;
  const rank = +member(o, 'subID');
  const conds = o.split('reunionConditions')[1] || '';
  const items = [...conds.matchAll(/itemID">(\d+)<[\s\S]*?amount">(\d+)</g)].filter((m) => m[1] !== '0').map((m) => `${m[1]}:${m[2]}`);
  growth.push([+member(o, 'ID'), group, rank, cdata(o, 'name').trim(), items.join(';'), rank === 1 ? descriptions.get(group - 1) || '' : '']);
}
growth.sort((a, b) => a[1] - b[1] || a[2] - b[2] || a[0] - b[0]);
writeFileSync(join(outDir, 'reunion-growth.tsv'), tsv([['#growth type id', 'rune (groupID 1-12)', 'rank (subID; 0 = a demon\'s own growth of that rune)', 'name', 'cost items (any one: id:amount;...)', 'rune description (client, on the rank-1 row)'], ...growth]));

// ---- level-xp.tsv ----
const constants = read(join(compHack, 'libcomp', 'libcomp', 'src', 'Constants.h.in'));
const xpBlock = constants.slice(constants.indexOf('LEVEL_XP_REQUIREMENTS[]'));
const xp = [...xpBlock.slice(0, xpBlock.indexOf('};')).matchAll(/(\d+)ULL/g)].map((m) => m[1]);
if (xp.length !== 99) throw new Error(`LEVEL_XP_REQUIREMENTS: ${xp.length} values, expected 99`);
writeFileSync(join(outDir, 'level-xp.tsv'), tsv([['#from level', 'XP to the next level (comp_hack LEVEL_XP_REQUIREMENTS)'], ...xp.map((v, i) => [i, v])]));

// ---- correct-table.tsv ----
const shared = read(join(compHack, 'libcomp', 'libcomp', 'schema', 'binarydata', 'shared.xml'));
const enumStart = shared.indexOf('CorrectTbl');
const enumBlock = shared.slice(enumStart, shared.indexOf('</member>', enumStart));
const ids = [...enumBlock.matchAll(/<value num="(\d+)">([A-Z_0-9]+)</g)].map((m) => [+m[1], m[2]]);
if (ids.length < 100) throw new Error(`CorrectTbl: only ${ids.length} values found`);
writeFileSync(join(outDir, 'correct-table.tsv'), tsv([['#CorrectTbl id', 'name (comp_hack shared.xml)'], ...ids]));

// ---- mitama-pg.tsv ----
const PG = { 47489: 1, 47490: 2, 47491: 3, 47492: 4 };   // Summon PG: Ara / Nigi / Kusi / Saki Mitama -> Mitama type
const barters = [];
for (const o of objects(read(join(decoded, 'NPCBarterData.xml')), 'MiNPCBarterData')) {
  const r = o.indexOf('"resultItems"'), t = o.indexOf('"tradeItems"');
  if (r < 0 || t < 0) continue;
  const [res, trade] = r < t ? [o.slice(r, t), o.slice(t)] : [o.slice(r), o.slice(t, r)];
  const items = (s) => [...s.matchAll(/"type">(\w+)<[\s\S]*?"subtype">(-?\d+)<[\s\S]*?"amount">(-?\d+)</g)].filter((m) => m[1] === 'ITEM').map((m) => [+m[2], +m[3]]);
  const got = items(res);
  if (got.length !== 1 || !PG[got[0][0]]) continue;
  barters.push([+member(o, 'ID'), got[0][0], PG[got[0][0]], got[0][1], items(trade).map(([id, n]) => `${id}:${n}`).join(';')]);
}
barters.sort((a, b) => a[2] - b[2] || a[0] - b[0]);
writeFileSync(join(outDir, 'mitama-pg.tsv'), tsv([['#barter id', 'PG item', 'Mitama type (1 Ara, 2 Nigi, 3 Kusi, 4 Saki)', 'PGs given', 'gems asked (id:amount;...)'], ...barters]));

console.log(`reunion-growth.tsv ${growth.length} rows, level-xp.tsv ${xp.length}, correct-table.tsv ${ids.length}, mitama-pg.tsv ${barters.length}`);
