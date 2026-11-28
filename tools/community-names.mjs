// What Community Builds need to name a published build that the page's other files don't have (app\ui\community-names.json):
//   - the expertises, from reference\expertise-imagine.tsv (the IMAGINE client's own names): a build holds their ids;
//   - which crystal each tarot / soul fusion comes from, from reference\enchants-game.tsv (the client's enchant table):
//     a build holds the fusion's enchant id, and the crystal is an item the Item Database names.
//
//   node tools/community-names.mjs
import fs from 'node:fs';
import path from 'node:path';

const root = path.resolve(path.dirname(new URL(import.meta.url).pathname.replace(/^\/([A-Za-z]:)/, '$1')), '..');
const rows = (file) => fs.readFileSync(path.join(root, 'reference', file), 'utf8').split(/\r?\n/).filter((l) => l && !l.startsWith('#')).map((l) => l.split('\t'));

const expertises = rows('expertise-imagine.tsv').map(([id, name, , maxClass, isChain]) => [Number(id), name, Number(maxClass) || 0, isChain === 'true'])
  .filter(([id, name]) => Number.isInteger(id) && name);
const crystals = {};
for (const [id, , item] of rows('enchants-game.tsv')) {
  const e = Number(id), i = Number(item);
  if (Number.isInteger(e) && e > 0 && Number.isInteger(i) && i > 0 && !(e in crystals)) crystals[e] = i;
}
const out = {
  note: 'For Community Builds. expertises: [id, name (IMAGINE client), top class, is a chain]. crystals: enchant id -> the crystal item it comes from. Made by tools\\community-names.mjs from reference\\expertise-imagine.tsv and reference\\enchants-game.tsv.',
  expertises, crystals,
};
fs.writeFileSync(path.join(root, 'app', 'ui', 'community-names.json'), JSON.stringify(out) + '\n');
console.log(`${expertises.length} expertises, ${Object.keys(crystals).length} crystals -> app\\ui\\community-names.json`);
