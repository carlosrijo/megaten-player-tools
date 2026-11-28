// The website's demo character: one character's saved page data, as the app's "load" answers it
// (crates\imagine-tracker\src\main_form.rs, load_data), for the character pages to show on the website.
//
//   node tools/build-demo.mjs <character data folder> <out folder> [name] [comp demons json]
//
// Writes <out>/demo.json, the character's picture, the digitalize demon's picture and the skill icons. The last
// argument is the demon picker's list (imagine-reader comp-demons --out): it fills the partner card and the demon list.
// Embedded demon pictures (the Digitalize unlocks', the demon list's) become files of their own, so demo.json stays
// small. Nothing about this PC goes in: no paths, no spreadsheet, no game state. tools\publish-web-assets.ps1 puts it
// online.
import fs from 'node:fs';
import path from 'node:path';

const [dir, out, nameArg, compFile] = process.argv.slice(2);
if (!dir || !out) { console.error('usage: node tools/build-demo.mjs <character data folder> <out folder> [name]'); process.exit(1); }
const read = (p, dflt) => { try { return JSON.parse(fs.readFileSync(path.join(dir, p), 'utf8').replace(/^﻿/, '')); } catch { return dflt; } };
const dash = (n, dflt) => read(path.join('dash', n + '.json'), dflt);

const snapshots = dash('snapshots', []);
if (!snapshots.length) { console.error(`No snapshots in ${dir}: take one first.`); process.exit(1); }
fs.rmSync(out, { recursive: true, force: true });
fs.mkdirSync(path.join(out, 'unlock-icons'), { recursive: true });

// embedded demon pictures -> demo/unlock-icons/<type>.png (one per demon type, shared by both lists)
const unpack = (d) => {
  const m = /^data:image\/png;base64,(.+)$/.exec(d.icon || '');
  if (!m || !(d.type > 0)) return;
  fs.writeFileSync(path.join(out, 'unlock-icons', `${d.type}.png`), Buffer.from(m[1], 'base64'));
  d.icon = `demo/unlock-icons/${d.type}.png`;
};
const unlocks = dash('digi-unlocks', []);
for (const race of unlocks) for (const d of race.demons || []) unpack(d);
const comp = compFile ? JSON.parse(fs.readFileSync(compFile, 'utf8').replace(/^﻿/, '')) : [];
comp.forEach(unpack);

// as Store does it: alignment and the switch skills' source from the profile
const profile = read(path.join('cache', 'profile.json'), null);
const saved = profile && /^Updated (\d{4}-\d{2}-\d{2} \d{2}:\d{2})/.exec(String(profile.note || ''));
const copy = (from, to) => { const p = path.join(dir, from); if (!fs.existsSync(p)) return false; fs.cpSync(p, path.join(out, to), { recursive: true }); return true; };
const meta = {
  character: nameArg || path.basename(path.resolve(dir)),
  alignment: (profile && profile.alignment ? String(profile.alignment) : 'NEUTRAL').toUpperCase(),
  alignmentKnown: !!profile,
  togglesSource: !profile ? null : profile.toggles_source || (profile.toggles && saved ? 'saved, ' + saved[1] : null),
  gameRunning: false,
  picture: copy('picture.png', 'picture.png') ? 'demo/picture.png' : null,
  autoPicture: false,
  demonPicture: copy(path.join('dash', 'demon.png'), 'demon.png') ? 'demo/demon.png' : null,
  skillIcons: copy(path.join('dash', 'skill-icons'), 'skill-icons') ? 'demo/skill-icons/' : null,
  demo: true,
};
const demo = {
  snapshots, gear: dash('gear', []), expertise: dash('expertise', []), bonuses: dash('bonuses', []),
  ranks: read('expertise-ranks.json', null), digitalize: dash('digitalize', null), unlocks,
  summoned: dash('summoned', null), icons: dash('icons', {}), meta, comp,
};
const text = JSON.stringify(demo);
if (/[A-Za-z]:\\\\Users\\\\|\\\\AppData\\\\/i.test(text)) { console.error('A path of this PC is in the page data: not publishing it.'); process.exit(1); }
fs.writeFileSync(path.join(out, 'demo.json'), text);
console.log(`Demo of ${meta.character}: ${snapshots.length} snapshots, ${Math.round(text.length / 1024)} KB.`);
