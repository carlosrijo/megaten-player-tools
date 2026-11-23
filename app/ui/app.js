'use strict';
// NewMoon Build Tracker: the page inside the desktop window.
// Data comes from the host (MainForm.cs) through host.call(cmd, args); in a plain browser a stand-in reads the repo's
// data\dash files so the page can be tested without the app (see README, "Working on the app's page").
(() => {
  const SVG = 'http://www.w3.org/2000/svg';
  const Q = new URLSearchParams(location.search);
  const $ = (sel, root) => (root || document).querySelector(sel);

  // ---------- small helpers ----------
  const ICONS = {
    moon: '<path d="M20 14.5A8.5 8.5 0 1 1 9.5 4a7 7 0 0 0 10.5 10.5z"/>',
    grid: '<rect x="3.5" y="3.5" width="7" height="7" rx="1.5"/><rect x="13.5" y="3.5" width="7" height="7" rx="1.5"/><rect x="3.5" y="13.5" width="7" height="7" rx="1.5"/><rect x="13.5" y="13.5" width="7" height="7" rx="1.5"/>',
    list: '<path d="M9 6h11M9 12h11M9 18h11"/><path d="M4.5 6h.01M4.5 12h.01M4.5 18h.01"/>',
    shield: '<path d="M12 3l8 3v6c0 4.5-3.2 8-8 9-4.8-1-8-4.5-8-9V6l8-3z"/>',
    bolt: '<path d="M13 3L5 13.5h6L10 21l8-10.5h-6L13 3z"/>',
    star: '<path d="M12 3.5l2.6 5.4 5.9.8-4.3 4.1 1 5.9L12 16.9l-5.2 2.8 1-5.9-4.3-4.1 5.9-.8L12 3.5z"/>',
    chart: '<path d="M4 4v16h16"/><path d="M8 15l3.5-4.5 3 2.5L19 7"/>',
    sliders: '<path d="M4 7h9M17 7h3M4 17h3M11 17h9"/><circle cx="15" cy="7" r="2"/><circle cx="9" cy="17" r="2"/>',
    camera: '<path d="M4 8h3l1.5-2h7L17 8h3a1 1 0 0 1 1 1v9a1 1 0 0 1-1 1H4a1 1 0 0 1-1-1V9a1 1 0 0 1 1-1z"/><circle cx="12" cy="13.2" r="3.4"/>',
    chevron: '<path d="M9 6l6 6-6 6"/>',
    search: '<circle cx="11" cy="11" r="6.5"/><path d="M16 16l4.5 4.5"/>',
    alert: '<path d="M12 4l9 16H3l9-16z"/><path d="M12 10v4.5M12 17.5v.01"/>',
    check: '<circle cx="12" cy="12" r="9"/><path d="M8 12.5l2.8 2.8L16.2 9.5"/>',
    info: '<circle cx="12" cy="12" r="9"/><path d="M12 11v5.5M12 7.8v.01"/>',
    x: '<path d="M6 6l12 12M18 6L6 18"/>',
    book: '<path d="M5 4.5h10.5a3 3 0 0 1 3 3V20H8a3 3 0 0 1-3-3V4.5z"/><path d="M5 17a3 3 0 0 1 3-3h10.5"/>',
  };
  function icon(name) {
    const s = document.createElementNS(SVG, 'svg');
    s.setAttribute('class', 'icon'); s.setAttribute('viewBox', '0 0 24 24'); s.setAttribute('aria-hidden', 'true');
    s.innerHTML = ICONS[name] || '';
    return s;
  }
  function h(tag, attrs, ...kids) {
    const el = document.createElement(tag);
    if (attrs) for (const [k, v] of Object.entries(attrs)) {
      if (v === null || v === undefined || v === false) continue;
      if (k === 'class') el.className = v;
      else if (k === 'text') el.textContent = v;
      else if (k === 'style') el.style.cssText = v;
      else if (k.startsWith('on')) el.addEventListener(k.slice(2), v);
      else el.setAttribute(k, v === true ? '' : String(v));
    }
    for (const kid of kids.flat(3)) if (kid !== null && kid !== undefined && kid !== false) el.append(kid);
    return el;
  }
  function s(tag, attrs, ...kids) {
    const el = document.createElementNS(SVG, tag);
    if (attrs) for (const [k, v] of Object.entries(attrs)) if (v !== null && v !== undefined) el.setAttribute(k, String(v));
    for (const kid of kids.flat()) if (kid) el.append(kid);
    return el;
  }
  const store = {
    get(k) { try { return localStorage.getItem('nm-' + k); } catch (e) { return null; } },
    set(k, v) { try { localStorage.setItem('nm-' + k, v); } catch (e) { /* preferences only */ } },
  };
  const num = (v) => (v === null || v === undefined || v === '' ? null : Number(v));
  const n0 = (v) => Number(v) || 0;
  const round2 = (v) => Math.round(v * 100) / 100;
  const isTrue = (v) => v === true || v === 'true';
  const nf = new Intl.NumberFormat('en-US', { maximumFractionDigits: 1 });
  /** 459, +50%, −91%. `signed` forces a plus on plain numbers too (+47). */
  function fmt(v, unit, signed) {
    const n = num(v);
    if (n === null || Number.isNaN(n)) return '—';
    const sign = n < 0 ? '−' : (n > 0 && (unit === '%' || signed) ? '+' : '');
    return sign + nf.format(Math.abs(n)) + (unit || '');
  }
  const date = (v) => new Date(String(v));
  function when(v) {
    const d = date(v);
    return Number.isNaN(d.getTime()) ? String(v || '—') : d.toLocaleString(undefined, { month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit', hour12: false });
  }
  const clock = (v) => date(v).toLocaleTimeString(undefined, { hour: '2-digit', minute: '2-digit', hour12: false });
  const sameDay = (a, b) => date(a).toDateString() === date(b).toDateString();
  function ago(v) {
    const mins = Math.round((Date.now() - date(v).getTime()) / 60000);
    if (!Number.isFinite(mins) || mins < 0) return '';
    if (mins < 1) return 'just now';
    if (mins < 60) return mins + ' min ago';
    if (mins < 48 * 60) return Math.round(mins / 60) + ' h ago';
    return Math.round(mins / 1440) + ' days ago';
  }

  // ---------- host bridge ----------
  const host = (() => {
    const wv = window.chrome && window.chrome.webview;
    if (wv) {
      let seq = 0;
      const pending = new Map();
      wv.addEventListener('message', (ev) => {
        const m = ev.data;
        if (!m || !pending.has(m.id)) return;
        const p = pending.get(m.id); pending.delete(m.id);
        if (m.ok) p.resolve(m.result); else p.reject(Object.assign(new Error(m.error || 'Something went wrong.'), { kind: m.kind }));
      });
      return { native: true, call: (cmd, args) => new Promise((resolve, reject) => { const id = ++seq; pending.set(id, { resolve, reject }); wv.postMessage({ id, cmd, args: args || {} }); }) };
    }
    // Stand-in for a plain browser: ?empty=1 (first run), ?snap=fail, ?clients=0|1|2 (how many characters are in game)
    const get = (p, dflt) => fetch(p).then((r) => (r.ok ? r.json() : dflt)).catch(() => dflt);
    const settings = { character: 'Rubito', csv: 'C:\\Users\\…\\smt-stats\\rubito-stats.csv', theme: store.get('theme') || 'system', dataDir: '…\\newmoon-reader\\data', csvExists: true };
    const wait = (ms) => new Promise((r) => setTimeout(r, ms));
    const fake = { active: 'Rubito', scanned: false, clients: [], characters: [{ name: 'Rubito', hasData: true, inGame: false, folder: '', level: 0 }] };
    const fakeClients = [{ pid: 7224, folder: 'client2', character: 'Rubito', level: 95 }, { pid: 30184, folder: 'client1', character: 'Rokujuu', level: 97 }];
    const fakeState = () => {
      fake.characters.forEach((c) => { const g = fake.clients.find((x) => x.character === c.name); c.inGame = !!g; c.folder = g ? g.folder : ''; c.level = g ? g.level : 0; c.picture = fakePic[c.name] || null; });
      return JSON.parse(JSON.stringify(fake));
    };
    // pictures: a drawn stand-in for the game window, cropped here in the browser
    const fakePic = {}, fakeSrc = {}, fakeAuto = {};
    const fakeShot = () => {
      const c = document.createElement('canvas'); c.width = 1280; c.height = 800;
      const g = c.getContext('2d'), sky = g.createLinearGradient(0, 0, 0, 800);
      sky.addColorStop(0, '#2b3350'); sky.addColorStop(1, '#8a8fa6'); g.fillStyle = sky; g.fillRect(0, 0, 1280, 800);
      g.fillStyle = '#0c2d3a'; g.fillRect(20, 80, 230, 360); g.fillStyle = '#e8c9a0'; g.beginPath(); g.arc(640, 300, 46, 0, 7); g.fill();
      g.fillStyle = '#b3264d'; g.fillRect(585, 346, 110, 190); g.fillStyle = '#fff'; g.font = '20px sans-serif'; g.fillText('stand-in game window', 520, 760);
      return { url: c.toDataURL('image/png'), width: 1280, height: 800 };
    };
    return {
      native: false,
      async call(cmd, args) {
        if (cmd === 'state') return fakeState();
        if (cmd === 'clients') {
          await wait(900);
          fake.clients = Q.get('clients') === '1' ? fakeClients.slice(1) : fakeClients.slice(0, Number(Q.get('clients') || 0));
          fake.scanned = true;
          return fakeState();
        }
        if (cmd === 'selectCharacter') {
          if (!fake.characters.some((c) => c.name === args.name)) fake.characters.push({ name: args.name, hasData: false });
          fake.active = settings.character = args.name;
          settings.csv = `C:\\Users\\…\\smt-stats\\${args.name.toLowerCase()}-stats.csv`;
          return fakeState();
        }
        if (cmd === 'load') {
          const empty = Q.get('empty') === '1' || fake.active !== 'Rubito';
          const [snapshots, gear, expertise, bonuses, ranks, digitalize, icons, unlocks] = empty ? [[], [], [], [], null, null, {}, []] : await Promise.all([
            get('../../data/dash/snapshots.json', []), get('../../data/dash/gear.json', []), get('../../data/dash/expertise.json', []),
            get('../../data/dash/bonuses.json', []), get('../../data/expertise-ranks.json', null), get(Q.get('digi') || '../../data/dash/digitalize.json', null), get('../../data/dash/icons.json', {}), get('../../data/dash/digi-unlocks.json', [])]);
          return { snapshots, gear, expertise, bonuses, ranks, digitalize, icons, unlocks, meta: { character: fake.active, alignment: (fake.align && fake.align[fake.active]) || 'NEUTRAL', alignmentKnown: fake.active === 'Rubito' || !!(fake.align && fake.align[fake.active]), csv: settings.csv, dataDir: settings.dataDir, gameRunning: fake.clients.length > 0, picture: fakePic[fake.active] || null, autoPicture: fakeAuto[fake.active] !== false, demonPicture: empty ? null : '../../data/dash/demon.png', skillIcons: empty ? null : '../../data/dash/skill-icons/' } };
        }
        if (cmd === 'snapshot') {
          await wait(1400);
          if (Q.get('snap') === 'fail') throw Object.assign(new Error('ImagineClient is not running.'), { kind: 'user' });
          return { source: Q.get('snap') === 'live' ? 'window' : 'cache', statsNote: 'Saved 2026-11-22 21:06', missing: '', level: 95, ranks: 31 };
        }
        if (cmd === 'status') return { gameRunning: fake.clients.length > 0, busy: false, clientsChanged: false };
        if (cmd === 'pictureSource') return fakeSrc[fake.active] || null;
        if (cmd === 'pictureCapture') {
          await wait(500);
          if (!fake.clients.some((c) => c.character === fake.active)) throw Object.assign(new Error(`No running client has '${fake.active}' logged in.`), { kind: 'user' });
          return (fakeSrc[fake.active] = { ...fakeShot(), view: Q.get('info') === '0' ? null : { x: 550, y: 240, w: 180, h: 382 } });
        }
        if (cmd === 'pictureFile') return (fakeSrc[fake.active] = fakeShot());
        if (cmd === 'pictureAuto') { fakeAuto[fake.active] = !!args.on; return fakeState(); }
        if (cmd === 'pictureCrop') {
          const src = fakeSrc[fake.active], img = new Image();
          await new Promise((r) => { img.onload = r; img.src = src.url; });
          const k = Math.min(1, 480 / args.w, 360 / args.h), c = document.createElement('canvas');
          c.width = Math.round(args.w * k); c.height = Math.round(args.h * k);
          c.getContext('2d').drawImage(img, args.x, args.y, args.w, args.h, 0, 0, c.width, c.height);
          fakePic[fake.active] = c.toDataURL('image/png'); fakeAuto[fake.active] = false;
          return fakeState();
        }
        if (cmd === 'pictureRemove') { delete fakePic[fake.active]; delete fakeSrc[fake.active]; return fakeState(); }
        if (cmd === 'compDemons') {
          await wait(args && args.cached ? 0 : 1200);   // seen before: at once; looking at the game: a moment
          const cur = (S.data && S.data.digitalize && S.data.digitalize.demon) || 'Specter';   // the saved demon, as the app would mark it
          return [['Specter', 90, 'Foul', 'Kusi', 0, false, true], ['Shikigami', 90, 'Brute', 'Ara', 8, true, true], ['Alilat', 93, 'Entity', 'Nigi', 8, false, false],
            ['Seraph', 66, 'Divine', '', 2, true, false], ['Pixie', 2, 'Fairy', '', 0, false, false]].map(([name, level, race, mitama, forceStack, equipment, inGame]) =>
            ({ key: name, name, level, race, mitama, forceStack, equipment, inGame: !(args && args.cached) && inGame, current: name === cur, icon: null, loaded: name !== 'Pixie', seenAt: name === 'Seraph' ? null : '2026-11-23T15:40:23',
              // stand-ins for the three states of the unlock tag
              unlock: name === 'Alilat' ? { listed: false, level: 0, raceLevel: 1, race, reached: false } : { listed: true, level: name === 'Seraph' ? 8 : 5, raceLevel: name === 'Seraph' ? 5 : 10, race, reached: name !== 'Seraph' } }));
        }
        // The browser preview has no game and no files to change: say so instead of pretending (the desktop app switches)
        if (cmd === 'useDemon') { await wait(300); throw Object.assign(new Error('This is the browser preview: it shows stand-in demons and can’t change your data. Pick the demon in the desktop app (NewMoonTracker.exe).'), { kind: 'user' }); }
        if (cmd === 'setAlignment') {
          await wait(900);
          fake.align = fake.align || {}; fake.align[fake.active] = args.alignment;
          return { changed: true, rows: fake.active === 'Rubito' ? 5 : 0, changedRows: fake.active === 'Rubito' ? 3 : 0, backup: 'rubito-stats.backup-before-alignment-20261122-233212.csv' };
        }
        if (cmd === 'getSettings' || cmd === 'pickCsv') return { ...settings };
        if (cmd === 'saveSettings') { Object.assign(settings, args); return { ...settings }; }
        return true;
      },
    };
  })();

  // ---------- what the page knows ----------
  const STATS = [['str', 'STR'], ['mag', 'MAG'], ['vit', 'VIT'], ['int', 'INT'], ['spd', 'SPD'], ['luck', 'LUCK'],
    ['clsr', 'CLS-R'], ['lngr', 'LNG-R'], ['spell', 'Spell'], ['support', 'Support'], ['pdef', 'P.Def'], ['mdef', 'M.Def']];
  const COMBAT = [
    ['crit', 'Critical', ''], ['final_crit', 'Final critical correction', '%'], ['lb_chance', 'Limit break chance', '%'], ['lb_power', 'Limit break power', '%'],
    ['lb_cap', 'Limit break damage cap', ''], ['pursuit_chance', 'Pursuit chance', '%'], ['pursuit_power', 'Pursuit power', '%'],
    ['ta_chance', 'Technical attack chance', '%'], ['ta_power', 'Technical attack power', '%'],
    ['dmg_close', 'Close-range damage dealt', '%'], ['dmg_long', 'Long-range damage dealt', '%'], ['dmg_spell', 'Spell damage dealt', '%'],
    ['taken_close', 'Close-range damage taken', '%'], ['taken_long', 'Long-range damage taken', '%'], ['taken_spell', 'Spell damage taken', '%'],
    ['boost_spin', 'Spin skill boost', '%'], ['boost_attack', 'Attack skill boost', '%'], ['boost_rush', 'Rush skill boost', '%'], ['boost_rapid', 'Rapid skill boost', '%'], ['boost_shot', 'Shot skill boost', '%'],
    ['cooldown', 'Cooldown', '%'], ['move_speed', 'Movement speed', '%'],
  ];
  // The Overview's stat sheet: one line per key stat, in two columns of named groups.
  const SHEET = [
    [['Offence', [['crit', 'Critical'], ['final_crit', 'Final correction'], ['lb_chance', 'Limit break chance'], ['lb_power', 'Limit break power'], ['lb_cap', 'Limit break cap'],
      ['ta_chance', 'Technical chance'], ['ta_power', 'Technical power'], ['pursuit_chance', 'Pursuit chance'], ['pursuit_power', 'Pursuit power']]]],
    [['Damage dealt', [['dmg_close', 'Close-range'], ['dmg_long', 'Long-range'], ['dmg_spell', 'Spell']]],
      ['Damage taken', [['taken_close', 'Close-range'], ['taken_long', 'Long-range'], ['taken_spell', 'Spell']]],
      ['Speed', [['cooldown', 'Cooldown'], ['move_speed', 'Movement']]]],
  ];
  /** The Bonuses row behind a key stat (same name, same unit; the cap is called "Limit break cap" there). */
  const BONUS_NAME = { lb_cap: 'Limit break cap' };
  const bonusRowOf = (k, label, unit) => (S.data.bonuses || []).find((r) => r.group === 'Combat' && r.bonus === (BONUS_NAME[k] || label) && (r.unit || '') === (unit || ''));
  const TOP_CHANGES = 5;   // the Overview lists this many changes until "Show all"
  const PAGES = [['overview', 'Overview', 'grid'], ['bonuses', 'Bonuses', 'list'], ['gear', 'Gear', 'shield'], ['expertise', 'Expertise', 'star'], ['digitalize', 'Digitalize', 'bolt'], ['history', 'History', 'chart']];
  // Tools: pages about the game itself, not about the character (they work before the first snapshot too).
  const TOOLS = [['dcm', 'DCM Locator', 'search'], ['quests', 'Quest Guide', 'book'], ['geardb', 'Gear Database', 'shield']];
  const isTool = (page) => TOOLS.some((t) => t[0] === page);
  const BONUS_GROUPS = [
    ['Combat', 'Combat', 'You · combat'], ['Stats', 'Stats', 'You · stats'], ['Affinity', 'Affinity', 'You · affinity'], ['Other', 'Other', 'You · other'],
    ['Partner', 'Partner', 'Your partner'], ['Only in some fights', 'Situational', 'Only in some fights · not added to the totals'],
  ];
  const KIND = { item: 'Item', mod: 'Mod', tarot: 'Tarot', soul: 'Soul fusion', appearance: 'Appearance', set: 'Set bonus', 'fusion set': 'Fusion set', expertise: 'Expertise', clan: 'Clan bonuses', dlevel: 'Digitalize level', quest: 'Quest bonus', compendium: 'Compendium', demon: 'Digitalize demon' };
  const KIND_ORDER = ['item', 'mod', 'tarot', 'soul', 'appearance', 'set', 'fusion set', 'expertise', 'clan', 'dlevel', 'quest', 'compendium', 'demon'];
  const lowIsGood = (name) => /taken|cooldown|cast time|incantation/i.test(name);

  // Stats that give nothing past a ceiling (from the user, 2026-11-23). The ceiling applies to the whole value
  // (gear + expertise, in the shown mode). To add one: its key from COMBAT above and its ceiling.
  const CAPS = { final_crit: 100, lb_chance: 100, ta_chance: 100, pursuit_chance: 100, pursuit_power: 100 };
  const capOfBonus = (row) => {   // the same list for a row of the bonus list (your own combat bonuses only)
    if (!row || row.group !== 'Combat') return null;
    const hit = COMBAT.find(([, label]) => label === row.bonus);
    return hit && CAPS[hit[0]] !== undefined ? CAPS[hit[0]] : null;
  };
  /** "At the 100% cap" / "10% over the cap" as a small tag, or nothing while below it. */
  function capTag(total, cap, unit, style) {
    if (cap === null || cap === undefined || total < cap) return null;
    const over = round2(total - cap);
    return h('span', { class: 'tag ' + (over > 0 ? 'tag-over' : 'tag-max'), style: style || null,
      'data-tip': over > 0 ? `Caps at ${fmt(cap, unit)}: the ${fmt(over, unit).replace(/^\+/, '')} above it adds nothing` : `Caps at ${fmt(cap, unit)}: more adds nothing`,
      text: over > 0 ? `${fmt(over, unit).replace(/^\+/, '')} over the cap` : 'At the cap' });
  }

  const S = {
    data: null, meta: {}, v: null,
    settings: { character: 'Rubito', csv: '', theme: 'system', dataDir: '' },
    mode: Q.get('mode') || store.get('mode') || 'normal',
    page: Q.get('page') || store.get('page') || 'overview',
    busy: false, scanning: false, lastScan: 0,
    state: { active: null, scanned: false, clients: [], characters: [] },   // who is in game, whose data is shown
    compare: null,
    over: { open: null, demon: store.get('demon-open') === '1', allChanges: false, switching: null, aff: store.get('aff') || null },   // (aff: the affinity series brought forward)   // Overview: the opened stat, demon details shown, every change listed, the demon being switched to
    demons: null,   // the COMP demons as last listed (the picker shows these at once, then looks at the game again)
    bonus: { q: '', group: 'all', open: new Set() },
    gear: { view: 'pieces', sel: null, cols: {}, colq: '', sort: null, short: Q.has('short') ? Q.get('short') === '1' : store.get('short') === '1' },
    dcm: { q: '', kind: 'all', open: new Set(), data: null, notes: {}, state: 'idle' },
    quest: { q: '', type: 'available', open: new Set(), data: null, state: 'idle' },
    gdb: { q: '', slot: 'all', known: true, open: new Set(), shown: 150, data: null, state: 'idle' },
    exp: { q: '', closed: new Set() },
    dg: { q: '', closed: new Set() },
    extra: { q: '', closed: new Set() },   // Bonuses: the quest bonus and compendium groups that are closed   // Digitalize: the search and the closed races
    hist: { key: store.get('hist') || 'lngr' },
  };
  if (Q.get('page') === 'compare') { S.page = 'gear'; S.gear.view = 'table'; }   // --page compare, for captures
  if (!PAGES.concat(TOOLS).some((p) => p[0] === S.page)) S.page = 'overview';
  if (S.mode !== 'digi') S.mode = 'normal';
  const digi = () => S.mode === 'digi';

  /** Everything worked out from the loaded data, once per load. */
  function derive() {
    const d = S.data;
    const snaps = d.snapshots.slice().sort((a, b) => String(a.taken_at).localeCompare(String(b.taken_at)));
    const latest = snaps[snaps.length - 1] || null;
    const by = {};
    if (latest) COMBAT.forEach(([k, label, unit]) => {
      const gear = n0(latest[k]);
      const gearDigi = latest[k + '_digi'] === null || latest[k + '_digi'] === undefined ? gear : n0(latest[k + '_digi']);
      // expertise: summed from the expertise list; a stat that list has no column for (movement speed, the skill
      // boosts) takes the expertise share of its Bonuses row, so both pages show the same total
      const exp = d.expertise.some((r) => r[k] !== undefined) ? round2(d.expertise.reduce((a, r) => a + n0(r[k]), 0))
        : n0((bonusRowOf(k, label, unit) || {}).expertise);
      // what the digitalize demon adds (only while digitalized): the saved demon, else the snapshot's own record
      const dgc = d.digitalize && d.digitalize.combat;
      const demon = dgc && dgc[k] !== undefined ? n0(dgc[k]) : n0(latest[k + '_demon']);
      by[k] = { stat: k, label, unit, gear, gearDigi, exp, demon };
    });
    // Which bonuses each item / set contributes to: from the per-source parts of the all-bonuses list.
    const setKey = (text) => { const m = /\((?:\d+\/\d+, )?(fusion set|set) (\d+)\)\s*$/.exec(String(text || '')); return m ? `${m[1]}:${m[2]}` : null; };
    // Sets made of the same pieces (the game gives some gear two set bonuses, e.g. sets 2021 and 2022) are one row:
    // the later ones are folded into the first, and their bonuses are filed under its key.
    const alias = new Map(), byPieces = new Map(), gearRows = [];
    d.gear.forEach((g) => {
      const key = g.kind === 'set' ? setKey(g.item) : 'item:' + g.item;
      const id = g.kind === 'set' ? (/set (\d+)\)\s*$/.exec(String(g.item || '')) || [])[1] : null;
      const same = g.kind === 'set' && g.shown_as ? byPieces.get(g.slot + '|' + g.shown_as) : null;
      if (same) { alias.set(key, same.key); if (id) same.setIds.push(id); return; }
      const row = { ...g, key, setIds: id ? [id] : [] };
      if (g.kind === 'set' && g.shown_as) byPieces.set(g.slot + '|' + g.shown_as, row);
      gearRows.push(row);
    });
    const partKey = (part) => {
      const key = (part.kind === 'set' || part.kind === 'fusion set') ? setKey(part.source) : (part.kind === 'expertise' ? null : 'item:' + part.source);
      return alias.get(key) || key;
    };
    const contrib = new Map();
    d.bonuses.forEach((row) => (Array.isArray(row.parts) ? row.parts : []).forEach((part) => {
      const key = partKey(part);
      if (!key) return;
      if (!contrib.has(key)) contrib.set(key, []);
      contrib.get(key).push({ row, part });
    }));
    const gear = gearRows.map((g) => ({ ...g, parts: contrib.get(g.key) || [] }));
    const ranks = new Map();
    ((d.ranks && d.ranks.ranks) || []).forEach((r) => ranks.set(r.name, r));
    // Compare columns: every bonus at least one worn piece or complete set adds to, with what each piece adds
    // (normal = always-on lines, digi = those plus the digitalized-only ones).
    const worn = new Set(gear.map((g) => g.key));
    const cols = d.bonuses.map((row) => {
      const per = new Map();
      (Array.isArray(row.parts) ? row.parts : []).forEach((part) => {
        const key = partKey(part);
        if (!key || !worn.has(key)) return;
        const e = per.get(key) || { normal: 0, digi: 0 }, v = n0(part.value);
        if (!isTrue(part.digi)) e.normal = round2(e.normal + v);
        e.digi = round2(e.digi + v);
        per.set(key, e);
      });
      return per.size ? { id: row.row, row, per } : null;
    }).filter(Boolean);
    // The worn piece or set a bonus part comes from (for its icon on the Bonuses page); undefined for other sources.
    const gearByKey = new Map(gear.map((g) => [g.key, g]));
    const gearOf = (part) => gearByKey.get(partKey(part));
    S.v = { snaps, latest, by, gear, ranks, cols, gearOf };
    if (!gear.some((g) => g.row === S.gear.sel)) S.gear.sel = gear.length ? gear[0].row : null;
  }
  /** One combat stat in the current mode. */
  function stat(k) {
    const r = S.v.by[k];
    if (!r) return null;
    const gear = digi() ? r.gearDigi : r.gear;
    const demonNow = digi() ? r.demon : 0;
    return { ...r, gearNow: gear, demonNow, total: round2(gear + r.exp + demonNow), extra: round2(r.gearDigi - r.gear) };
  }

  // ---------- tooltip ----------
  const tipEl = $('#tip');
  /** Puts the tip at `left`, with its top at `below`, or (when that would run off the window) its bottom at `above`. */
  function placeTip(text, left, below, above) {
    // a plain one-line hint, or a ready-made element (the game-style tooltips of icons)
    const rich = text instanceof Node;
    if (!rich) tipEl.textContent = text; else if (tipEl.firstChild !== text || tipEl.childNodes.length !== 1) tipEl.replaceChildren(text);
    tipEl.classList.toggle('tip-rich', rich); tipEl.hidden = false;
    // The tip is a popover so it sits in the top layer, above the demon dropdown and the dialogs (which are there too;
    // a plain z-index can't get above them). The top layer stacks in the order things were opened: it is opened here,
    // after whatever is open, and closed in hideTip.
    try { if (!tipEl.matches(':popover-open')) tipEl.showPopover(); } catch (e) { /* no popover support: it shows as before */ }
    const w = tipEl.offsetWidth, hgt = tipEl.offsetHeight;
    tipEl.style.left = Math.max(8, Math.min(left, window.innerWidth - w - 8)) + 'px';
    tipEl.style.top = Math.max(8, below + hgt > window.innerHeight ? Math.min(above - hgt, window.innerHeight - hgt - 8) : below) + 'px';
  }
  /** Gives an element a game-style tooltip: `build` makes its content the first time the pointer is over it. */
  const withTip = (el, build) => { el.dataset.rich = ''; el._rich = build; return el; };
  const tipHead = (iconEl, title, sub) => h('div', { class: 'tt-head' }, iconEl, h('div', null, h('div', { class: 'tt-title', text: title }), sub ? h('div', { class: 'tt-sub', text: sub }) : null));
  /** Tooltip text as lines: "[Section]" lines become headings; Japanese flavour lines are left out. */
  const tipLines = (text) => {
    const out = []; let blank = true;
    String(text || '').replace(/\r/g, '').split('\n').forEach((line) => {
      if (/[ぁ-ゖァ-ヺ一-鿿]/.test(line)) return;   // kana or kanji (not the "・" bullet the English text uses)
      const t = line.replace(/　/g, ' ').replace(/\s+$/, '');
      if (!t.trim()) { blank = true; return; }
      const sec = /^\[(.+)\]$/.exec(t.trim());
      out.push(sec ? h('div', { class: 'tt-sec', text: sec[1] }) : h('div', { class: 'tt-line' + (blank && out.length && !sec ? ' tt-gap' : ''), text: t.replace(/^ {2}(?! )/, '') }));
      blank = false;
    });
    return out;
  };
  const showTip = (text, x, y) => placeTip(text, x + 14, y + 18, y - 10);   // beside the pointer
  let tipFor = null;   // the element a keyboard-focus tip describes
  const hideTip = () => {
    try { if (tipEl.matches(':popover-open')) tipEl.hidePopover(); } catch (e) { /* as above */ }
    tipEl.hidden = true;
    if (tipFor) { tipFor.removeAttribute('aria-describedby'); tipFor = null; }
  };
  document.addEventListener('pointermove', (ev) => {
    const rich = ev.target instanceof Element ? ev.target.closest('[data-rich]') : null;
    if (rich && rich._rich) { showTip(rich._richNode || (rich._richNode = rich._rich()), ev.clientX, ev.clientY); return; }
    const t = ev.target instanceof Element ? ev.target.closest('[data-tip]') : null;
    if (t) showTip(t.dataset.tip, ev.clientX, ev.clientY); else if (!ev.target.closest || !ev.target.closest('.chart')) hideTip();
  });
  document.addEventListener('pointerleave', hideTip);
  document.addEventListener('keydown', hideTip, true);
  // The pointer can stay still while the page moves under it (mouse wheel) or changes (a click): don't leave a tip behind.
  document.addEventListener('scroll', hideTip, true);
  document.addEventListener('pointerdown', hideTip, true);
  // Keyboard: the same tip under whatever Tab lands on. A button with no tip of its own shows the one inside it.
  document.addEventListener('focusin', (ev) => {
    const el = ev.target;
    if (!(el instanceof Element) || !el.matches(':focus-visible')) return;
    const t = el.closest('[data-tip]') || (el.matches('button') ? el.querySelector('[data-tip]') : null);
    if (!t) return;
    const r = el.getBoundingClientRect();
    placeTip(t.dataset.tip, r.left, r.bottom + 6, r.top - 6);
    tipFor = el; el.setAttribute('aria-describedby', 'tip');
  });
  document.addEventListener('focusout', hideTip);

  // ---------- toasts ----------
  function toast(kind, title, text, ms) {
    const el = h('div', { class: 'toast toast-' + kind },
      icon(kind === 'ok' ? 'check' : kind === 'warn' ? 'alert' : kind === 'err' ? 'alert' : 'info'),
      h('div', null, h('div', { class: 'toast-title', text: title }), text ? h('div', { class: 'toast-text', text }) : null),
      h('button', { type: 'button', class: 'btn btn-ghost btn-square', 'aria-label': 'Dismiss', onclick: () => el.remove() }, icon('x')));
    homeToasts();
    toastsEl.append(el);
    if (ms !== 0) setTimeout(() => el.remove(), ms || 7000);
  }
  // An open dialog covers the rest of the page and switches it off, toasts included. So while one is open the toasts
  // live inside it (still pinned to the window's corner), and go back to the page when it closes.
  const toastsEl = $('#toasts');
  function homeToasts() {
    const home = [...document.querySelectorAll('dialog[open]')].pop() || document.body;
    if (toastsEl.parentElement === home) return;
    toastsEl.querySelectorAll('.toast').forEach((t) => { t.style.animation = 'none'; });   // moving would replay the entrance
    home.append(toastsEl);
  }
  const dialogWatch = new MutationObserver(homeToasts);
  document.querySelectorAll('dialog').forEach((d) => dialogWatch.observe(d, { attributes: true, attributeFilter: ['open'] }));

  // ---------- theme ----------
  const media = window.matchMedia('(prefers-color-scheme: dark)');
  // "system" follows Windows' light/dark app setting, also while the app is open. The saved choice lives with the
  // window (settings.json); a ?theme= override (used for --capture) is for that run only and is never stored.
  let themeNow = 'system';
  function paintTheme() {
    const dark = themeNow === 'dark' || (themeNow !== 'light' && media.matches);
    document.documentElement.dataset.theme = dark ? 'dark' : 'light';
    host.call('theme', { dark }).catch(() => {});
    document.querySelectorAll('[data-theme-choice]').forEach((b) => b.setAttribute('aria-pressed', String(b.dataset.themeChoice === S.settings.theme)));
    if (S.data) render();   // charts read colours from the page
  }
  function applyTheme(choice, thisRunOnly) {
    themeNow = choice === 'light' || choice === 'dark' ? choice : 'system';
    if (!thisRunOnly) { S.settings.theme = themeNow; store.set('theme', themeNow); }
    paintTheme();
  }
  media.addEventListener('change', () => { if (themeNow === 'system') paintTheme(); });
  // The change event is only delivered while the page is being drawn, so look again when the window comes back.
  const recheckTheme = () => { if (themeNow === 'system' && (document.documentElement.dataset.theme === 'dark') !== media.matches) paintTheme(); };
  window.addEventListener('focus', recheckTheme);
  document.addEventListener('visibilitychange', recheckTheme);

  // ---------- chrome: header, nav, sidebar foot ----------
  document.querySelectorAll('[data-icon]').forEach((el) => el.append(icon(el.dataset.icon)));
  const nav = $('#nav');
  PAGES.forEach(([id, label, ic], i) => nav.append(h('button', { type: 'button', class: 'nav-item', 'data-page': id, 'data-tip': `${label} (Ctrl+${i + 1})`, onclick: () => go(id) },
    icon(ic), h('span', { class: 'nav-label', text: label }))));
  nav.append(h('div', { class: 'nav-head', text: 'Tools (beta)' }));
  TOOLS.forEach(([id, label, ic]) => nav.append(h('button', { type: 'button', class: 'nav-item', 'data-page': id, 'data-tip': label, onclick: () => go(id) },
    icon(ic), h('span', { class: 'nav-label', text: label }))));
  function go(page) {
    if (S.page === page) return;
    S.page = page; store.set('page', page);
    render(); $('#main').scrollTop = 0;
  }
  // (Normal / Digitalized is picked in the demon dropdown on the Overview: "No demon" or a demon. See setMode.)

  function renderChrome() {
    const L = S.v && S.v.latest;
    const who = S.meta.character || S.state.active;
    $('#who-name').textContent = who || 'No character yet';
    const pic = $('#who-pic'), picUrl = S.meta.picture || null;
    pic.hidden = !picUrl;
    pic.className = 'who-pic ' + alignClass(S.meta.alignment);   // framed in the alignment's colour, as on the Overview
    if (picUrl && pic.getAttribute('src') !== picUrl) pic.src = picUrl;
    // Alignment is read from the game at each snapshot; before the first one the reader assumes Neutral, and the header says so.
    const al = S.meta.alignment ? S.meta.alignment.charAt(0) + S.meta.alignment.slice(1).toLowerCase() + (S.meta.alignmentKnown === false ? ' (assumed)' : '') : '';
    $('#who-sub').textContent = L ? [`Level ${fmt(L.level)}`, al].filter(Boolean).join(' · ') : (who ? 'No snapshot yet' : 'Log in to the game');
    $('#page-title').textContent = (PAGES.concat(TOOLS).find((p) => p[0] === S.page) || [])[1] || '';   // the page's name, in the middle of the top bar
    document.querySelectorAll('.nav-item').forEach((b) => { if (b.dataset.page === S.page) b.setAttribute('aria-current', 'page'); else b.removeAttribute('aria-current'); });
    const foot = $('#side-foot');
    foot.replaceChildren();
    if (L) {
      const live = isTrue(L.stats_live);
      foot.append(h('span', { text: 'Last snapshot' }), h('b', { text: when(L.taken_at) }), h('span', { text: ago(L.taken_at) }),
        h('span', { class: live ? '' : 'warn', style: 'margin-top:6px', text: live ? 'All read live' : `Stats from ${sameDay(L.stats_from, L.taken_at) ? clock(L.stats_from) : when(L.stats_from)}` }));
    }
    // Is the shown character in game right now? (from the last look at the running clients)
    const g = $('#game'), st = S.state;
    const mine = who ? st.clients.find((c) => c.character === who) : null;
    const others = st.clients.filter((c) => c.character && c.character !== who);
    g.hidden = !st.scanned && !S.scanning;
    g.classList.toggle('on', !!mine);
    $('#game-text').textContent = !st.scanned ? 'Looking for the game…'
      : mine ? `${who} is in game`
      : !st.clients.length ? 'Game not running'
      : who ? `${who} is not in game` : 'Nobody logged in yet';
    g.dataset.tip = !st.scanned ? 'Checking which characters are logged in' : mine ? `Logged in on ${mine.folder || 'a game client'}` : others.length ? `In game now: ${others.map((c) => c.character).join(', ')}. Click the name on the left to switch.` : 'Start the game and log in';
    const snap = $('#snap');
    snap.disabled = S.busy;
    $('#snap-label').textContent = S.busy ? 'Reading the game…' : 'Take snapshot';
    const ic = snap.querySelector('.btn-icon');
    ic.replaceChildren(S.busy ? h('span', { class: 'spin' }) : icon('camera'));
  }

  // ---------- shared page pieces ----------
  const pageHead = (sub, ...right) => h('div', { class: 'page-head' },
    h('div', null, sub ? h('p', { class: 'page-sub', text: sub }) : null), right);   // the page's name is in the top bar (renderChrome)
  const legend = (...items) => h('div', { class: 'legend' }, items.map(([cls, text]) => h('span', null, h('i', { class: 'swatch ' + cls }), text)));
  function searchBox(value, placeholder, oninput) {
    const input = h('input', { class: 'input', type: 'search', placeholder, value, 'aria-label': placeholder, 'data-search': true, spellcheck: 'false', oninput: () => oninput(input.value) });
    input.addEventListener('keydown', (ev) => { if (ev.key === 'Escape' && input.value) { input.value = ''; oninput(''); ev.stopPropagation(); } });
    return h('div', { class: 'search' }, icon('search'), input);
  }
  function firstRun() {
    const name = S.meta.character || S.state.active;
    if (!name) return h('div', { class: 'page' }, h('div', { class: 'empty' },
      h('h2', { class: 'empty-title', text: 'No character yet' }),
      h('p', { class: 'page-sub', text: 'Start NewMoon and log in. The app reads your character’s name from the game by itself; if two game clients are open it asks which one to look at.' }),
      h('button', { type: 'button', class: 'btn btn-primary', onclick: () => openChooser() }, 'Look for my character')));
    return h('div', { class: 'page' }, h('div', { class: 'empty' },
      h('h2', { class: 'empty-title', text: `No snapshot of ${name} yet` }),
      h('p', { class: 'page-sub', text: 'A snapshot reads your stats, gear and expertise from the running game and works out every bonus. It only reads; nothing in the game is changed.' }),
      h('ol', { class: 'steps' },
        h('li', null, h('span', { text: `Start NewMoon and log in as ${name}.` })),
        h('li', null, h('span', { text: 'Press Take snapshot (or F5).' }))),
      h('button', { type: 'button', class: 'btn btn-primary', onclick: takeSnapshot }, icon('camera'), 'Take snapshot')));
  }

  // ---------- Overview ----------
  /** Alignment (LAW / NEUTRAL / CHAOS) as a colour class and as a word. */
  const alignClass = (a) => (/^(LAW|NEUTRAL|CHAOS)$/i.test(String(a || '')) ? 'align-' + String(a).toLowerCase() : '');
  const alignName = (a) => (a ? String(a).charAt(0) + String(a).slice(1).toLowerCase() : '');

  /** The demon the Digitalized view adds: who, how much of its stats, which assists. */
  function demonCard() {
    const DG = S.data.digitalize;
    // Not digitalized (or no demon saved yet): the same one-line band with "No demon", so the page doesn't jump when
    // you go between that and a demon. Its name is the same dropdown.
    if (!digi() || !DG) return h('section', { class: 'card demon-card closed' },
      h('div', { class: 'demon-pic none', 'aria-hidden': 'true' }, h('span', { text: '–' })),
      h('div', { class: 'demon-main' }, h('div', { class: 'demon-id' },
        h('h3', { class: 'demon-name' }, S.over.switching
          ? h('span', { class: 'demon-switching', role: 'status' }, h('span', { class: 'spin ink' }), `Switching to ${S.over.switching}…`)
          : h('button', { type: 'button', class: 'name-btn', popovertarget: 'demons', 'aria-haspopup': 'menu', 'data-tip': 'Pick a demon to see your numbers while digitalized with it',
            onclick: (ev) => { demonsFor = ev.currentTarget; } }, 'No demon', h('span', { class: 'chev down' }, icon('chevron')))),
        h('span', { class: 'tag tag-off', text: 'Not digitalized' }))));
    const why = isTrue(DG.digitalized_when_taken) ? 'you were digitalized with it when it was saved' : 'it is the demon you last digitalized with or picked';
    // its icon from the game (32x32, shown at double size, pixel-crisp); its initial when there is none
    // framed in the colour of its alignment (Law / Neutral / Chaos)
    // (no "Digitalize demon" label: the user found it redundant; what it explained is the picture's tip now)
    const pic = h('div', { class: 'demon-pic ' + alignClass(DG.alignment), 'aria-hidden': 'true', 'data-tip': `Your digitalize demon. Its passives, features and Mitama sets count when the boost for them is learned. Chosen because ${why}.` }, h('span', { text: String(DG.demon || '?').charAt(0) }));
    if (S.meta.demonPicture) {
      const img = h('img', { src: S.meta.demonPicture, alt: '' });
      img.onerror = () => img.remove();
      pic.append(img);
    }
    const chips = [`Lv ${fmt(DG.level)}`, DG.race, DG.mitama ? `${DG.mitama} Mitama` : null].filter(Boolean)
      .map((t) => h('span', { class: 'tag', text: t }));
    if (DG.alignment) chips.splice(2, 0, h('span', { class: 'tag tag-align ' + alignClass(DG.alignment), text: alignName(DG.alignment) }));
    const idChips = chips.length;   // who it is; the chips after these explain what counts, and show with the details
    chips.push(h('span', { class: 'tag tag-demon', tabindex: 0, 'data-tip': `You get ${fmt(DG.rate)}% of its stats (at most ${fmt(DG.max_rate)}%).`, text: `${fmt(DG.rate)}% stats` }));
    if (isTrue(DG.mitama_boost)) chips.push(h('span', { class: 'tag', tabindex: 0, 'data-tip': 'Mitama Demon Enhancement: the boosted Mitama set bonuses apply.', text: 'Sets boosted' }));
    // the stats it adds: tiny bars like the player's, four columns of three, HP and MP first.
    // Each bar is scaled within its group, so STR..LUCK compare with each other, HP with MP, CLS-R..Support together.
    const ORDER = ['Max HP', 'Max MP', 'STR', 'MAG', 'VIT', 'INT', 'SPD', 'LUCK', 'CLS-R', 'LNG-R', 'Spell', 'Support'];
    const rank = (s) => { const i = ORDER.indexOf(s.stat); return i < 0 ? 99 : i; };
    const stats = (DG.stats || []).slice().sort((a, b) => rank(a) - rank(b));
    const group = (s) => (/^Max /.test(s.stat) ? 'hpmp' : /^(CLS-R|LNG-R|Spell|Support)$/.test(s.stat) ? 'atk' : 'base');
    const gmax = {};
    stats.forEach((s) => { gmax[group(s)] = Math.max(gmax[group(s)] || 1, n0(s.value)); });
    const bars = h('div', { class: 'dg-bars' }, stats.map((s) =>
      h('div', { class: 'srow dg-row', 'data-tip': `${s.stat} +${fmt(s.value)} while digitalized` },
        h('span', { class: 'srow-label', text: s.stat.replace(/^Max /, '').replace(/^Support$/, 'Supp') }),
        h('div', { class: 'strack' }, h('i', { class: 'demon', style: `width:${Math.max(2, (n0(s.value) / gmax[group(s)]) * 100)}%` })),
        h('span', { class: 'srow-value', text: '+' + fmt(s.value) }))));
    // learned assists, one line per level (the full wording on hover)
    const short = (t) => t.replace(/^Duration /, 'Time ').replace(/ min$/, 'm').replace(/^Passive skills$/, 'Passives').replace(/^Active skills$/, 'Actives')
      .replace(/^Mitama sets$/, 'Mitama sets').replace(/^Resistances$/, 'Resistances');
    const assists = h('div', { class: 'demon-assists' },
      h('span', { class: 'stat-label', text: 'Digitalize boosts' }),
      isTrue(DG.assists_known)
        // every level, L1-L10 (five per column); level 1 offers no assist, and a level with no pick shows a dash
        ? (() => {
          const byLevel = new Map();
          (DG.assists || []).forEach((a) => { const m = /^L(\d+) (.*)$/.exec(a); if (m) byLevel.set(+m[1], m[2]); });
          const top = Math.max(10, ...byLevel.keys());
          return h('div', { class: 'dg-assist-list', style: `--rows:${Math.ceil(top / 2)}` }, Array.from({ length: top }, (_, i) => {
            const lv = i + 1, t = byLevel.get(lv);
            return h('div', { class: 'dg-assist' + (t ? '' : ' none'), 'data-tip': t ? `Level ${lv} assist: ${t}` : lv === 1 ? 'Level 1 has no assist' : `No assist picked at level ${lv} (or not reached yet)` },
              h('b', { text: 'L' + lv }), h('span', { text: t ? short(t) : '—' }));
          }));
        })()
        : h('span', { class: 'note', text: 'Couldn’t be read, so only its stats count.' }));
    // what else it brings: your Max HP/MP while digitalized, its Demon Force stack and equipment (each counted only with
    // the race's assist for it), and its Demon Force values (summoned only)
    const extra = [];
    // (your Max HP / MP while digitalized are with your own stats, beside your picture)
    const FS = DG.force_stack;
    if (FS && FS.count) {
      const lines = (FS.effects || []).concat((FS.other || []).map((t) => t + ' (not for you while digitalized)'));
      extra.push(h('span', { class: 'tag tag-demon' + (isTrue(FS.counted) ? '' : ' tag-off'), tabindex: 0,
        'data-tip': (isTrue(FS.counted) ? 'Counted (Demon Force assist learned): ' : 'Not counted: needs this race’s Demon Force assist. ') + (lines.join(' · ') || 'no effects'),
        text: `Force stack ×${FS.count}` }));
    }
    (DG.equipment || []).forEach((q) => extra.push(h('span', { class: 'tag tag-demon' + (isTrue(q.counted) ? '' : ' tag-off'), tabindex: 0,
      'data-tip': `Demon equipment in slot ${q.slot}${q.replaces ? `, in place of the feature ${q.replaces}` : ''}. ` +
        (isTrue(q.counted) ? 'Counted (Features assist learned)' : 'Not counted: needs this race’s Features assist') + ((q.effects || []).length ? ': ' + q.effects.join(' · ') : ''),
      text: q.replaces ? `${q.name} (replaces ${q.replaces})` : q.name })));
    if ((DG.force || []).length) extra.push(h('span', { class: 'tag tag-off', tabindex: 0, 'data-tip': `Demon Force: ${DG.force.join(', ')}. Only for the summoned demon, not while digitalized.`, text: 'Demon Force (summoned only)' }));
    // One line by default: who it is. "Details" opens the rest (what counts, the stats it adds, its boosts per level):
    // icon | name, chips and bars | boosts, all in one band. The choice is remembered.
    // The name is the picker: click it for the dropdown of COMP demons (popovertarget opens and closes it; the click
    // only says which button it hangs from).
    const open = S.over.demon;
    // its own skills: closed, the icons of its slotted skills on the line; open, the skills and features under the rest
    const SK = demonSkills(DG);
    const actions =h('span', { class: 'demon-actions' },
      h('button', { type: 'button', class: 'btn btn-sm btn-ghost', 'aria-expanded': String(open), 'data-tip': open ? 'Back to one line' : 'The stats it adds, what counts, and its boosts per level',
        onclick: () => { S.over.demon = !open; store.set('demon-open', open ? '0' : '1'); render(); } }, h('span', { class: 'chev' }, icon('chevron')), open ? 'Less' : 'Details'));
    const name = S.over.switching
      ? h('span', { class: 'demon-switching', role: 'status' }, h('span', { class: 'spin ink' }), `Switching to ${S.over.switching}…`)
      : h('button', { type: 'button', class: 'name-btn', popovertarget: 'demons', 'aria-haspopup': 'menu', 'data-tip': 'Pick another demon, or no demon',
        onclick: (ev) => { demonsFor = ev.currentTarget; } }, DG.demon, h('span', { class: 'chev down' }, icon('chevron')));
    const id = h('div', { class: 'demon-id' },
      h('h3', { class: 'demon-name' }, name), open ? chips : chips.slice(0, idChips), open || !SK ? null : SK.icons, actions);
    if (!open) return h('section', { class: 'card demon-card closed' }, pic, h('div', { class: 'demon-main' }, id));
    return h('section', { class: 'card demon-card' }, pic,
      h('div', { class: 'demon-main' }, id, extra.length ? h('div', { class: 'demon-extra' }, extra) : null, bars),
      assists, SK ? SK.parts : null);
  }

  /**
   * Your affinity boosts and resistances in the shown mode, one entry per element.
   * The Bonuses list has two rows for some of them ("Fire Boost" from gear, in %, and "Fire boost" from expertise, as a
   * plain number): both are the server's BOOST_<element> value (RES_<element> for resistances), so they are added here.
   */
  // the game's names and order (CorrectTbl affinity order; the reader names them from the client's text)
  const AFF_ORDER = ['Slash', 'Thrust', 'Blunt', 'Handgun', 'Penetrate', 'Spread', 'Fire', 'Ice', 'Electric', 'Almighty', 'Force', 'Expel', 'Death', 'Curative', 'Support', 'Mystic', 'Nerve', 'Mind', 'Words of Power', 'Special', 'Suicide'];
  function affinity() {
    const by = { boost: new Map(), resistance: new Map(), cap: new Map() };
    (S.data.bonuses || []).forEach((r) => {
      if ((r.group !== 'Affinity' && r.group !== 'Other') || !isTrue(r.counted)) return;   // yours, and always on
      // each element's boost stops at 100% unless something raises that element's cap ("Ice affinity cap")
      // rows are "<element> affinity", "<element> affinity cap" and "<element> resist" (Bonuses.Canonical)
      const m = /^(.+?) (affinity cap|affinity|resist)$/i.exec(String(r.bonus || ''));
      // an element's affinity and cap are in the Affinity group ("Weapon based Affinity" is not an element: Other)
      if (!m || (m[2].toLowerCase() !== 'resist' && r.group !== 'Affinity')) return;
      const k = m[2].toLowerCase(), name = m[1].charAt(0).toUpperCase() + m[1].slice(1), map = by[k === 'affinity cap' ? 'cap' : k === 'resist' ? 'resistance' : 'boost'];
      const e = map.get(name) || { name, gear: 0, exp: 0, demon: 0 };
      e.gear = round2(e.gear + n0(digi() && r.gear_digi !== null && r.gear_digi !== undefined ? r.gear_digi : r.gear));
      e.exp = round2(e.exp + n0(r.expertise));
      if (digi()) e.demon = round2(e.demon + n0(r.demon));
      map.set(name, e);
    });
    const rank = (n) => { const i = AFF_ORDER.indexOf(n); return i < 0 ? AFF_ORDER.length : i; };
    // the series share their spokes: every element with a boost, a resistance or a raised cap, in the game's order
    const names = [...new Set([...by.boost.values(), ...by.resistance.values(), ...by.cap.values()].filter((e) => round2(e.gear + e.exp + e.demon)).map((e) => e.name))]
      .sort((a, b) => rank(a) - rank(b) || a.localeCompare(b));
    const list = (map) => names.map((name) => { const e = map.get(name) || { name, gear: 0, exp: 0, demon: 0 }; return { ...e, total: round2(e.gear + e.exp + e.demon) }; });
    // the cap on each spoke: 100 plus what raises it (the raise keeps its gear / expertise / demon split)
    const cap = list(by.cap).map((e) => ({ ...e, raise: e.total, total: round2(100 + e.total) }));
    return { boost: list(by.boost), resistance: list(by.resistance), cap };
  }

  /**
   * Affinity, beside the stat sheet: one radar, one spoke per element, three things on the same percentage scale:
   * the boost to what you deal, the cap that boost stops at, and how much of the element you resist (below 0 is a
   * weakness). The buttons under it bring one of the three forward: the others fade and its numbers appear on the spokes.
   */
  const AFF_SERIES = [['boost', 'Boost'], ['cap', 'Cap'], ['res', 'Resistance']];
  function affinityCard() {
    const DG = S.data.digitalize, aff = affinity(), n = aff.boost.length;
    const card = h('section', { class: 'card' });
    const wrap = sect('Affinity', digi() && DG ? `While digitalized with ${DG.demon}` : 'Not digitalized', null, card);
    if (n < 3) {   // a radar needs three spokes
      card.append(h('p', { class: 'note', text: n ? aff.boost.concat(aff.resistance).filter((e) => e.total).map((e) => `${e.name} ${fmt(e.total, '%')}`).join(' · ') : 'No affinity boosts or resistances yet.' }));
      return wrap;
    }
    const focus = AFF_SERIES.some(([k]) => k === S.over.aff) ? S.over.aff : null;
    const val = { boost: aff.boost.map((e) => e.total), cap: aff.cap.map((e) => e.total), res: aff.resistance.map((e) => e.total) };
    // the scale: from the deepest weakness (or 0) to the cap, or past it when something is over the cap; steps of 50
    const lo = Math.min(0, Math.floor(Math.min(...val.res, ...val.boost) / 50) * 50), hi = Math.max(100, Math.ceil(Math.max(...val.cap, ...val.boost, ...val.res) / 50) * 50);
    const W = 346, H = 318, cx = W / 2, cy = H / 2, R = 94;   // sized for the card's column, with room for a two-line label on every spoke
    const svg = s('svg', { viewBox: `0 0 ${W} ${H}`, width: W, height: H, role: 'img',
      'aria-label': 'Affinity by element. ' + aff.boost.map((e, i) => `${e.name}: boost ${fmt(e.total, '%')}, cap ${fmt(val.cap[i])}%, resistance ${fmt(val.res[i], '%')}`).join('; ') });
    const ang = (i) => -Math.PI / 2 + (i * 2 * Math.PI) / n;
    const at = (i, v, r0) => { const r = r0 !== undefined ? r0 : R * Math.max(0, Math.min(1, (v - lo) / (hi - lo))); return [cx + r * Math.cos(ang(i)), cy + r * Math.sin(ang(i))]; };
    const path = (pts) => pts.map(([x, y], i) => `${i ? 'L' : 'M'}${x.toFixed(1)},${y.toFixed(1)}`).join('') + 'Z';
    const all = aff.boost.map((_, i) => i);
    // grid: the spokes, a ring every 50 with its number between the first two spokes, the 0 ring a little stronger
    all.forEach((i) => { const [x, y] = at(i, hi); svg.append(s('line', { class: 'gridline', x1: cx, y1: cy, x2: x, y2: y })); });
    for (let v = lo + 50; v <= hi; v += 50) {
      svg.append(s('path', { class: v === 0 && lo < 0 ? 'zeroline' : 'gridline', d: path(all.map((i) => at(i, v))) }));
      const [x, y] = at(0.5, v);
      svg.append(Object.assign(s('text', { class: 'ring-label', x: x + 3, y: y + 3 }), { textContent: fmt(v) + (v === hi ? '%' : '') }));
    }
    // the three series; the one brought forward is drawn last, the others faded
    const draw = { boost: 'circle', res: 'square', cap: null };
    AFF_SERIES.map(([k]) => k).sort((a, b) => (a === focus) - (b === focus)).forEach((k) => {
      const g = s('g', { class: `aff-s aff-${k}` + (focus && focus !== k ? ' dim' : '') });
      g.append(s('path', { class: 'aff-area', d: path(all.map((i) => at(i, val[k][i]))) }), s('path', { class: 'aff-line', d: path(all.map((i) => at(i, val[k][i]))) }));
      if (draw[k]) all.forEach((i) => {
        if (!val[k][i]) return;
        const [x, y] = at(i, val[k][i]), bad = k === 'res' && val[k][i] < 0 ? ' bad' : '';
        g.append(draw[k] === 'circle' ? s('circle', { class: 'aff-dot' + bad, cx: x, cy: y, r: 4 })
          : s('rect', { class: 'aff-dot' + bad, x: x - 3.5, y: y - 3.5, width: 7, height: 7, rx: 1 }));
      });
      svg.append(g);
    });
    // Labels go outside the rim, each centred on its spoke: the element, and under it the number of the series brought
    // forward. Near the top and bottom neighbours sit side by side, so a label that would run into one already placed
    // moves further out along its spoke. Each spoke's tooltip has all three numbers and where they come from.
    const split = (e) => { const parts = [['gear', e.gear], ['expertise', e.exp], ...(digi() && DG ? [[DG.demon, e.demon]] : [])].filter(([, v]) => v); return parts.length > 1 ? ` (${parts.map(([name, v]) => `${name} ${fmt(v, '%')}`).join(' + ')})` : parts.length ? ` (${parts[0][0]})` : ''; };
    const placed = [], raised = aff.cap.filter((c) => c.raise);
    all.forEach((i) => {
      const e = aff.boost[i], rs = aff.resistance[i], cp = aff.cap[i], capText = fmt(cp.total) + '%', c = Math.cos(ang(i)), sn = Math.sin(ang(i));
      // the number of the series brought forward: nothing rather than "0%", and the cap without a plus sign
      const value = focus === 'cap' ? capText : focus && val[focus][i] ? fmt(val[focus][i], '%') : '', quiet = !e.total && !rs.total && !cp.raise;
      const anchor = c > 0.3 ? 'start' : c < -0.3 ? 'end' : 'middle';
      const w = Math.max(e.name.length, value.length) * 6.4 + 4, hgt = value ? 25 : 13;   // room the text needs, near enough
      let r = R + 9 + 13 * Math.abs(sn), box, lx, ly;
      for (let tries = 0; tries < 5; tries++, r += 8) {
        [lx, ly] = at(i, 0, r);
        const x0 = anchor === 'start' ? lx : anchor === 'end' ? lx - w : lx - w / 2;
        box = [x0, ly - hgt / 2, x0 + w, ly + hgt / 2];
        if (!placed.some((b) => box[0] < b[2] && b[0] < box[2] && box[1] < b[3] && b[1] < box[3])) break;
      }
      placed.push(box);
      const [rx, ry] = at(i, hi), top = ly - hgt / 2 + 10;
      svg.append(s('g', { 'data-tip': `${e.name}: boost ${fmt(e.total, '%')}${split(e)} · cap ${capText}${cp.raise ? ` (100% + ${fmt(cp.raise)}%)` : ''} · resistance ${fmt(rs.total, '%')}${split(rs)}` },
        s('line', { x1: cx, y1: cy, x2: rx, y2: ry, stroke: 'transparent', 'stroke-width': 16 }),
        Object.assign(s('text', { class: 'aff-name' + (quiet ? ' none' : ''), x: lx, y: top, 'text-anchor': anchor }), { textContent: e.name }),
        value ? Object.assign(s('text', { class: 'aff-value' + (focus === 'res' && val.res[i] < 0 ? ' bad' : ''), x: lx, y: top + 12, 'text-anchor': anchor }), { textContent: value }) : null));
    });
    // the legend is the control: All, or one of the three
    const pick = (k, label, mark) => h('button', { type: 'button', 'aria-pressed': String(focus === k), onclick: () => { S.over.aff = k; store.set('aff', k || ''); render(); } },
      mark ? h('i', { class: 'aff-key aff-key-' + k }) : null, label);
    card.append(h('div', { class: 'chart radar' }, svg),
      h('div', { class: 'seg aff-pick', role: 'group', 'aria-label': 'Bring forward' }, pick(null, 'All'), AFF_SERIES.map(([k, label]) => pick(k, label, true))));
    // a line about the series brought forward; nothing under "All" (the user asked for that hint to go)
    if (focus) card.append(h('p', { class: 'note', text: focus === 'cap' ? 'Boosts stop at 100%' + (raised.length ? `, except where something of yours raises the cap: ${raised.map((c) => `${c.name} ${fmt(c.total)}%`).join(', ')}.` : '. Nothing of yours raises it for any element.')
      : focus === 'res' ? 'How much of each element you resist. Inside the 0 ring is a weakness.'
      : 'What you add to the damage you deal with each element.' }));
    return wrap;
  }


  function pageOverview() {
    const L = S.v.latest;
    const DG = S.data.digitalize;
    const page = h('div', { class: 'page' });
    // the character's picture (or a place to add one) sits beside the title; click it to change
    const portrait = h('button', { type: 'button', class: 'portrait' + (S.meta.picture ? '' : ' no-pic') + ' ' + alignClass(S.meta.alignment), 'aria-label': S.meta.picture ? 'Change picture' : 'Add a picture',
      'data-tip': S.meta.picture ? 'Change this character’s picture' : 'Add a picture of this character from the game', onclick: () => openPicture() },
      S.meta.picture ? h('img', { src: S.meta.picture, alt: `Picture of ${S.meta.character}` }) : h('span', null, icon('camera'), h('span', { text: 'Add picture' })));
    // who this is: name, level, alignment ("assumed" until it has been set in Settings)
    const align = S.meta.alignment ? S.meta.alignment.charAt(0) + S.meta.alignment.slice(1).toLowerCase() + (S.meta.alignmentKnown === false ? ' (assumed)' : '') : '';
    // Your stats under the name, compact like the demon's: Max HP, Max MP and the 12, each a thin bar of base + bonus
    // and, in the Digitalized view, what the digitalize demon adds. Bars are scaled within their group (HP with MP,
    // STR..LUCK, CLS-R..Support, the two defences).
    const add = {};   // what the demon adds, by stat name
    if (digi() && DG) (DG.stats || []).forEach((x) => { add[x.stat] = n0(x.value); });
    const HM = digi() && DG ? DG.hp_mp : null;   // Max HP / MP while digitalized: from the HP/MP formula on the final stats, not a plain sum
    const vital = (label, key, normal, whenDigi) => {
      const total = HM && whenDigi !== null && whenDigi !== undefined ? n0(whenDigi) : n0(normal) + n0(add[key]);
      return { label, group: 'hpmp', base: n0(normal), bonus: 0, demon: Math.max(0, total - n0(normal)), total, whole: true };
    };
    const mine = [vital('HP', 'Max HP', L.max_hp, HM && HM.hp), vital('MP', 'Max MP', L.max_mp, HM && HM.mp),
      ...STATS.map(([k, label], i) => ({ label, group: i < 6 ? 'base' : i < 10 ? 'atk' : 'def', base: Math.max(0, n0(L[k + '_base'])), bonus: Math.max(0, n0(L[k + '_bonus'])),
        demon: n0(add[label]), total: n0(L[k]) + n0(add[label]), noDemon: i >= 10 }))];
    const gmax = {};
    mine.forEach((e) => { gmax[e.group] = Math.max(gmax[e.group] || 1, e.total); });
    const seg = (cls, v, e) => (v > 0 ? h('i', { class: cls, style: `width:${(v / gmax[e.group]) * 100}%` }) : null);
    const statRow = (e) => h('div', { class: 'srow dg-row', tabindex: 0,
      'data-tip': (e.whole ? `Max ${e.label} ${fmt(e.total)}` + (e.demon ? ` = ${fmt(e.base)} normally + ${DG.demon} ${fmt(e.demon)}` : '')
        : `${e.label} ${fmt(e.total)} = base ${fmt(e.base)} + bonus ${fmt(e.bonus)}` + (e.demon ? ` + ${DG.demon} ${fmt(e.demon)}` : ''))
        + (digi() && DG && e.noDemon ? '. What digitalizing does to this one isn’t worked out, so it is the normal value.' : '') },
      h('span', { class: 'srow-label', text: e.label === 'Support' ? 'Supp' : e.label }),
      h('div', { class: 'strack' }, seg('base', e.base, e), seg('bonus', e.bonus, e), seg('demon', e.demon, e)),
      h('span', { class: 'srow-value', text: fmt(e.total) }));
    page.append(h('div', { class: 'page-head with-portrait' }, portrait, h('div', { class: 'page-head-text' },
      h('div', { class: 'who-line' },
        h('p', { class: 'page-who' }, h('b', { class: 'who-big', text: S.meta.character || '' }), [`Level ${fmt(L.level)}`, align].filter(Boolean).map((t) => ` · ${t}`).join('')),
        legend(['sw-base', 'Base'], ['sw-bonus', 'Bonus'], ...(digi() && DG ? [['sw-demon', DG.demon]] : []))),
      h('div', { class: 'dg-bars who-stats' }, mine.map(statRow)))));
    page.append(demonCard());   // always there: a demon, or "No demon"

    const missing = String(L.gear_missing || '').trim();
    if (missing) page.append(h('div', { class: 'notice notice-warn' }, icon('alert'), h('div', null,
      h('b', { text: `No saved bonuses for ${missing}. ` }), 'They are not in the totals yet. Open the Information window in game once, then take a snapshot.')));
    else if (!isTrue(L.stats_live)) page.append(h('div', { class: 'notice notice-warn' }, icon('alert'), h('div', null,
      h('b', { text: `The 12 stats and HP/MP are from ${when(L.stats_from)}. ` }), 'Gear and expertise are current. The stats are kept from then because you were digitalized (the game then shows the demon’s share too). Take a snapshot when not digitalized to refresh them.')));

    // Key stats: one line each (name, how close to its cap when it has one, total). Where the total comes from is a
    // click away, under the line; every single source is on the Bonuses page.
    const sheetRow = ([k, label]) => {
      const r = stat(k);
      if (!r) return null;
      const f = (v) => fmt(v, r.unit, true);
      const cap = CAPS[k] === undefined ? null : CAPS[k];
      const over = cap !== null && r.total > cap ? round2(r.total - cap) : 0;
      const plain = (v) => fmt(v, r.unit).replace(/^\+/, '');
      const srcs = [['sw-gear', 'Gear', r.gearNow], ['sw-exp', 'Expertise', r.exp], ...(digi() && DG ? [['sw-demon', DG.demon, r.demonNow]] : [])].filter(([, , v]) => v);
      const open = S.over.open === k;
      // a bar only where there is a cap, so every bar on the sheet means the same thing: how close to the cap
      // (full and in the warning colour when past it; by how much is in the tooltip and under the opened line)
      const meter = cap === null ? h('span') : h('span', { class: 'cap-bar' + (over ? ' over' : '') }, h('i', { style: `width:${Math.max(0, Math.min(100, (r.total / cap) * 100))}%` }));
      const row = h('button', { type: 'button', class: 'sheet-row', 'aria-expanded': String(open),
        'data-tip': (srcs.length ? srcs.map(([, name, v]) => `${name} ${f(v)}`).join(' · ') : 'Nothing adds to this yet')
          + (cap === null ? '' : `. Caps at ${fmt(cap, r.unit)}` + (over ? `: ${plain(over)} over, which adds nothing.` : '.')),
        onclick: () => { S.over.open = open ? null : k; render(); } },
        h('span', { class: 'chev' }, icon('chevron')), h('span', { class: 'sheet-label', text: label }), meter,
        h('span', { class: 'sheet-value' + (r.total < 0 && !lowIsGood(r.label) ? ' bad' : ''), text: fmt(r.total, r.unit) }));
      if (!open) return row;
      const bonusRow = bonusRowOf(k, r.label, r.unit);
      const line = (cls, name, value, note) => h('div', { class: 'sheet-src' }, h('i', { class: 'swatch ' + cls }),
        h('span', null, name, note ? h('small', { text: note }) : null), h('b', { text: value }));
      return [row, h('div', { class: 'sheet-detail' },
        srcs.length ? srcs.map(([cls, name, v]) => line(cls, name, f(v),
          cls === 'sw-gear' && r.extra ? (digi() ? `${f(r.extra)} of it only while digitalized` : `${f(r.extra)} more while digitalized`) : null))
          : h('p', { class: 'note', text: 'Nothing adds to this yet.' }),
        cap === null ? null : h('p', { class: 'note', text: `Caps at ${fmt(cap, r.unit)}: ` + (over ? `the ${plain(over)} above it adds nothing, so those points could go elsewhere.`
          : r.total >= cap ? 'reached, more adds nothing.' : `${plain(round2(cap - r.total))} to go.`) }),
        bonusRow ? h('button', { type: 'button', class: 'btn btn-sm', onclick: () => { S.bonus.q = bonusRow.bonus; S.bonus.group = 'all'; S.bonus.open.add(bonusRow.row); go('bonuses'); } },
          'Every source in Bonuses') : null)];
    };
    const sheet = h('section', { class: 'card sheet' }, h('div', { class: 'sheet-cols' }, SHEET.map((col) => h('div', { class: 'sheet-col' },
      col.map(([title, stats]) => h('div', { class: 'sheet-group' }, h('h3', { class: 'list-head', text: title }), stats.map(sheetRow)))))));

    // the sheet with what changed under it; affinity (two radars, one over the other) beside them
    page.append(h('div', { class: 'over-grid' }, h('div', { class: 'over-main' }, sheet, changesCard()), h('div', { class: 'over-side' }, affinityCard(), chainCard())));

    page.append(h('p', { class: 'note', text: 'Not included anywhere: buffs, clan and title bonuses, and your partner demon\u2019s own stats. The game only works those out on the server. Pick your digitalize demon above to add what it gives.' }));
    return page;
  }

  /**
   * Expertise on Overview: points spent against the cap (as the game's Expertise window shows them: points / 100, "pt"),
   * then each chain expertise the character has unlocked with its class-rank and the icons of the skills its reached
   * ranks give. No stats here: what they add is on the Expertise and Bonuses pages. A chain opens the Expertise page on it.
   */
  function chainCard() {
    const R = S.data.ranks || {}, P = R.points || null;
    const pt = (v) => (n0(v) / 100).toFixed(2) + 'pt';
    const card = h('section', { class: 'card' });
    const wrap = sect('Expertise', P ? (P.cap ? `${pt(P.total)} / ${pt(P.cap)}` : `${pt(P.total)}, no cap`) : null, null, card);
    if (P && P.cap) {
      const free = n0(P.cap) - n0(P.total);
      card.append(h('div', { class: 'exp-points', 'data-tip': `Cap ${pt(P.cap)} = 17000.00pt + 1000.00pt per 10 levels (level ${P.level})`
          + (P.level === 99 ? ' + 1000.00pt at 99' : '') + ` + ${P.extension} expansions × 1000.00pt` },
        h('span', { class: 'exp-bar' }, h('i', { style: `width:${Math.max(0, Math.min(100, (n0(P.total) / n0(P.cap)) * 100))}%` })),
        h('span', { class: 'exp-free', text: free > 0 ? `${pt(free)} free` : 'At the cap' })));
    } else if (!P) card.append(h('p', { class: 'note', text: 'Take a snapshot to read your expertise points and cap.' }));
    const chains = ((R.ranks) || []).filter((r) => r.chain);
    const more = h('button', { type: 'button', class: 'btn btn-sm chain-more', onclick: () => { S.exp.q = ''; go('expertise'); } }, 'View more');
    if (!chains.length) { card.append(h('p', { class: 'note', text: 'No chain expertise unlocked yet.' }), more); return wrap; }
    // One line of icons per kind. The skills come from the ranks file ("skills": every skill of the reached ranks,
    // actives too); a passive's row in expertise.json says whether a toggle is switched off (shown dimmed).
    // Older data without "skills" shows the passives from expertise.json only.
    const iconLine = (label, list) => (list.length ? h('span', { class: 'chain-kind' }, h('span', { class: 'chain-kind-label', text: label }),
      h('span', { class: 'chain-icons' }, list.map((s) => {
        const ic = skillIcon(s);
        ic.dataset.tip = `${s.name} (${s.rank})` + (s.off ? ', toggle switched off' : '');
        if (s.off) ic.classList.add('off');
        return ic;
      }))) : null);
    card.append(h('h4', { class: 'list-head', text: 'Chain expertise' }), h('div', { class: 'chain-list' }, chains.map((c) => {
      const rows = S.data.expertise.filter((r) => r.expertise === c.name && !isExtra(r));
      const offIds = new Set(rows.filter((r) => r.toggle && !r.toggle_on).map((r) => r.skill_id));
      const all = Array.isArray(c.skills) ? c.skills.map((s) => ({ ...s, off: offIds.has(s.id) }))
        : rows.map((r) => ({ id: r.skill_id, name: r.skill || '—', icon: r.icon, rank: r.rank || '', active: false, off: r.toggle && !r.toggle_on }));
      const passives = all.filter((s) => !s.active), actives = all.filter((s) => s.active);
      return h('button', { type: 'button', class: 'chain', 'data-tip': `Open ${c.name} on the Expertise page`,
        onclick: () => { S.exp.q = c.name; go('expertise'); } },
        h('span', { class: 'chain-head' }, h('span', { class: 'chain-name', text: c.name }), h('span', { class: 'chain-rank', text: `${c.class}-${c.rank}` })),
        all.length ? [iconLine('Passive', passives), iconLine('Active', actives)] : h('span', { class: 'chain-none', text: 'No skills from its ranks yet' }));
    })), more);
    return wrap;
  }

  function changesCard() {
    const { snaps, latest: L } = S.v;
    const earlier = snaps.slice(0, -1);
    const card = h('section', { class: 'card' });
    if (!earlier.length) {
      card.append(h('p', { class: 'note', text: 'Take another snapshot after you change gear, and the differences will be listed here.' }));
      return sect('What changed', null, null, card);
    }
    const base = earlier.find((x) => x.taken_at === S.compare) || earlier[earlier.length - 1];
    const sel = h('select', { class: 'select', 'aria-label': 'Compare with snapshot', onchange: () => { S.compare = sel.value; render(); } },
      earlier.slice().reverse().map((x) => h('option', { value: x.taken_at, selected: x.taken_at === base.taken_at, text: when(x.taken_at) })));
    const wrap = sect('What changed', h('label', { class: 'card-sub', style: 'display:inline-flex;align-items:center;gap:8px' }, 'since', sel), null, card);

    const sfx = digi() ? '_digi' : '';
    const groups = [
      ['Character', [['level', 'Level', ''], ['max_hp', 'Max HP', ''], ['max_mp', 'Max MP', '']]],
      ['Stats', STATS.map(([k, label]) => [k, label, ''])],
      [digi() ? 'Gear bonuses, digitalized' : 'Gear bonuses', COMBAT.map(([k, label, unit]) => [k + sfx, label, unit])],
    ];
    const list = h('div', { class: 'chg-list' });
    // every change, with how big it is next to the number it changed from (so 50 on 365 and 51 on 524 can be compared)
    const changed = groups.map(([title, keys]) => [title, keys.map(([k, label, unit]) => {
      const a = num(base[k]), b = num(L[k]);
      if (a === null || b === null || a === b) return null;
      const d = round2(b - a), good = lowIsGood(label) ? d < 0 : d > 0;
      return { size: Math.abs(d) / Math.max(Math.abs(a), Math.abs(b), 1), el: h('div', { class: 'chg' }, h('span', { text: label }),
        h('span', { class: 'chg-vals' }, fmt(a, unit) + ' → ', h('b', { text: fmt(b, unit) })),
        h('span', { class: 'delta ' + (good ? 'good' : 'bad'), text: (d > 0 ? '▲ ' : '▼ ') + fmt(d, unit, true) })) };
    }).filter(Boolean)]);
    const count = changed.reduce((n, [, rows]) => n + rows.length, 0);
    if (!count) { card.append(h('p', { class: 'note', text: `Nothing changed since ${when(base.taken_at)}: same stats, same gear bonuses.` })); return wrap; }
    // the biggest few first; the whole list, grouped, on request
    const all = S.over.allChanges || count <= TOP_CHANGES;
    if (all) changed.forEach(([title, rows]) => { if (rows.length) list.append(h('div', { class: 'chg-group', text: title }), ...rows.map((r) => r.el)); });
    else list.append(...changed.flatMap(([, rows]) => rows).sort((x, y) => y.size - x.size).slice(0, TOP_CHANGES).map((r) => r.el));
    card.append(list);
    // the bonus lines are the snapshots' gear columns, so they can differ from the sheet's totals
    if (changed[2][1].length) card.append(h('p', { class: 'note', text: 'Bonus lines compare gear only; expertise and the digitalize demon are not in them.' }));
    if (count > TOP_CHANGES) card.append(h('button', { type: 'button', class: 'btn btn-sm chg-more', 'aria-expanded': String(all),
      onclick: () => { S.over.allChanges = !S.over.allChanges; render(); } }, all ? `Show the ${TOP_CHANGES} biggest` : `Show all ${count} changes`));
    return wrap;
  }

  // ---------- Bonuses ----------
  function partsList(row, onlySource) {
    const parts = (Array.isArray(row.parts) ? row.parts : []).filter((p) => !onlySource || String(p.source).toLowerCase().includes(onlySource))
      .sort((a, b) => Math.abs(n0(b.value)) - Math.abs(n0(a.value)));
    return h('ul', { class: 'parts' }, parts.map((p) => {
      const notes = [];
      if (p.note) notes.push(String(p.note).replace(/^(When|If) /, (m) => m.toLowerCase()));
      // a tarot or soul fusion line also names the crystal fused into the piece (the piece alone doesn't say which)
      const piece = S.v.gearOf(p), crystal = piece && (p.kind === 'soul' ? piece.soul : p.kind === 'tarot' ? piece.tarot : null);
      // "demon": the demon's picture; "skill:<name>": the skill icon file of that name; else a picture in icons.json (compendium, clan, quest)
      const pic = String(p.icon || '');
      const other = { icon: pic === 'demon' ? S.meta.demonPicture : pic.startsWith('skill:') ? (S.meta.skillIcons ? S.meta.skillIcons + pic.slice(6) + '.png' : null) : S.data.icons[pic] };
      return h('li', null,
        // the worn piece's or set's icon; a skill's own icon (expertise, demon skills); the demon's picture for the rest it gives
        // hovering it shows a game-style tooltip
        piece ? withTip(gearIcon(piece), gearTip(piece)) : withTip(gearIcon(other), sourceTip(p, other)),
        h('span', { class: 'kind', text: KIND[p.kind] || p.kind || '' }),
        h('span', null, p.source, crystal ? h('span', { class: 'part-from', text: crystal }) : null, isTrue(p.digi) ? h('span', { class: 'tag ' + (digi() ? 'tag-digi' : 'tag-off'), style: 'margin-left:8px', text: 'Digi only' }) : null,
          notes.length ? h('small', { text: notes.join(' · ') }) : null),
        h('span', { class: 'pv', text: p.value === null || p.value === undefined ? '' : fmt(p.value, row.unit, true) }));
    }));
  }

  function pageBonuses() {
    // Rows that only the digitalize demon gives appear in the Digitalized view only.
    const demonOnly = (r) => (r.parts || []).length > 0 && r.parts.every((p) => p.kind === 'demon');
    const all = S.data.bonuses.filter((r) => digi() || !demonOnly(r));
    const DG = S.data.digitalize, ncol = digi() ? 5 : 4;
    const page = h('div', { class: 'page' });
    page.append(pageHead(digi() && DG ? `Everything your gear, expertise and ${DG.demon} (digitalized) give. Click a row to see where each part comes from.`
      : 'Everything your gear and expertise give. Click a row to see where each part comes from.'));
    const counts = {}; all.forEach((r) => { counts[r.group] = (counts[r.group] || 0) + 1; });
    const chips = h('div', { class: 'chips', role: 'group', 'aria-label': 'Filter by kind' });
    const body = h('tbody');
    const drawChips = () => chips.replaceChildren(
      ...[['all', 'All', all.length], ...BONUS_GROUPS.filter(([id]) => counts[id]).map(([id, short]) => [id, short, counts[id]])].map(([id, label, n]) =>
        h('button', { type: 'button', class: 'chip-btn', 'aria-pressed': String(S.bonus.group === id), onclick: () => { S.bonus.group = id; drawChips(); draw(); } },
          label, h('span', { class: 'count', text: n }))));
    const draw = () => {
      body.replaceChildren();
      const q = S.bonus.q.trim().toLowerCase();
      let shown = 0;
      BONUS_GROUPS.forEach(([gid, , heading]) => {
        if (S.bonus.group !== 'all' && S.bonus.group !== gid) return;
        const rows = all.filter((r) => r.group === gid).map((r) => {
          if (!q) return { r, viaSource: false };
          if (String(r.bonus).toLowerCase().includes(q)) return { r, viaSource: false };
          const hit = (r.parts || []).some((p) => String(p.source).toLowerCase().includes(q));
          return hit ? { r, viaSource: true } : null;
        }).filter(Boolean);
        if (!rows.length) return;
        body.append(h('tr', { class: 'group-row' }, h('th', { colspan: ncol, scope: 'colgroup', text: heading })));
        rows.forEach(({ r, viaSource }) => {
          shown++;
          const counted = isTrue(r.counted);
          const open = S.bonus.open.has(r.row) || viaSource;
          const gear = digi() ? r.gear_digi : r.gear, total = digi() ? r.total_digi : r.total;
          const extra = round2(n0(r.gear_digi) - n0(r.gear));
          const nParts = (r.parts || []).length;
          const toggle = () => { if (S.bonus.open.has(r.row)) S.bonus.open.delete(r.row); else S.bonus.open.add(r.row); draw(); };
          const cell = (v, cls) => h('td', { class: 'num ' + (n0(v) ? cls || '' : 'zero'), text: num(v) === null ? '—' : n0(v) === 0 ? '0' : fmt(v, r.unit, true) });
          const tr = h('tr', { class: 'row-click' + (open ? ' open' : ''), onclick: (ev) => { if (!ev.target.closest('button')) toggle(); } },
            h('td', null, h('button', { type: 'button', class: 'row-btn', 'aria-expanded': String(open), onclick: toggle },
              h('span', { class: 'chev' }, icon('chevron')),
              h('span', null, r.bonus, h('span', { class: 'src-count', text: nParts === 1 ? '1 source' : `${nParts} sources` }),
                counted && extra ? h('span', { class: 'tag ' + (digi() ? 'tag-digi' : 'tag-off'), style: 'margin-left:8px', text: digi() ? `incl. ${fmt(extra, r.unit, true)} digi` : `${fmt(extra, r.unit, true)} digitalized` }) : null,
                counted ? capTag(n0(total), capOfBonus(r), r.unit, 'margin-left:8px') : null))),
            cell(gear), cell(r.expertise), digi() ? cell(r.demon) : null,
            counted ? cell(total, 'emph')
              : h('td', { class: 'num zero', 'data-tip': 'Depends on the fight, so it is not added up', text: '—' }));
          body.append(tr);
          // Found through an item's name: list just that item's lines, unless the row was opened by hand.
          if (open) body.append(h('tr', { class: 'detail' }, h('td', { colspan: ncol }, partsList(r, viaSource && !S.bonus.open.has(r.row) ? q : null))));
        });
      });
      if (!shown) body.append(h('tr', null, h('td', { colspan: ncol, class: 'empty-row', text: q ? `Nothing matches “${S.bonus.q.trim()}”.` : 'No bonuses recorded yet.' })));
    };
    page.append(h('div', { class: 'toolbar' },
      searchBox(S.bonus.q, 'Search a bonus or an item', (v) => { S.bonus.q = v; draw(); }), chips, h('span', { class: 'toolbar-spacer' }),
      h('button', { type: 'button', class: 'btn btn-sm', onclick: () => { all.forEach((r) => S.bonus.open.add(r.row)); draw(); } }, 'Expand all'),
      h('button', { type: 'button', class: 'btn btn-sm', onclick: () => { S.bonus.open.clear(); draw(); } }, 'Collapse all')));
    page.append(h('div', { class: 'tbl-wrap' }, h('table', { class: 'tbl' },
      h('thead', null, h('tr', null, h('th', { scope: 'col', text: 'Bonus' }), h('th', { scope: 'col', class: 'num', text: 'Gear' }),
        h('th', { scope: 'col', class: 'num', text: 'Expertise' }), digi() ? h('th', { scope: 'col', class: 'num', text: DG ? DG.demon : 'Demon' }) : null,
        h('th', { scope: 'col', class: 'num', text: digi() ? 'Total, digitalized' : 'Total' }))), body)));
    page.append(h('p', { class: 'note', text: 'Gear includes tarot, soul fusions, mods, complete sets and appearance bonuses. For bonuses that only apply in some fights, the largest single value is shown.' }));
    drawChips(); draw();
    // Quest bonus and Demonic Compendium: what each step gives (their values are already in the rows above)
    const extras = S.data.expertise.filter(isExtra);
    if (extras.length) {
      // one section each, the title outside the box like every other section ("Quest bonus (32 quests)" -> title + count)
      const names = []; extras.forEach((r) => { if (!names.includes(r.expertise)) names.push(r.expertise); });
      names.forEach((name) => {
        const mine = extras.filter((r) => r.expertise === name), m = /^(.*?)\s*\((.*)\)\s*$/.exec(name);
        const counted = mine.some((r) => isTrue(r.counted));
        page.append(sect(m ? m[1] : name, [m ? m[2] : null, mine.length === 1 ? '1 bonus' : `${mine.length} bonuses`].filter(Boolean).join(' · '),
          counted ? null : h('span', { class: 'tag tag-off', text: 'Not counted' }),
          h('section', { class: 'card step-list' }, mine.map((r) => h('div', { class: 'passive' + (isTrue(r.counted) ? '' : ' off') },
            h('span', { class: 'skill' }, h('span', { text: r.skill || '—' })),
            h('span', { class: 'fx' }, String(r.effects || '').split('; ').filter(Boolean).map((t) => h('span', { text: t }))))))));
      });
      page.append(h('p', { class: 'note', text: 'What each step gives. The counted ones are already in the Expertise column above.' }));
    }
    return page;
  }

  // ---------- Gear ----------
  const setTitle = (g) => String(g.item || '').replace(/\s*\((\d+\/\d+), (fusion )?set \d+\)\s*$/, '');
  const setIds = (g) => g.setIds || [];
  // The short name in lists: "Set 1430", or "2 sets" when several sets share the same pieces.
  const setLabel = (g) => (setIds(g).length > 1 ? `${setIds(g).length} sets` : `${g.slot === 'Fusion set' ? 'Fusion' : 'Set'} ${setIds(g)[0] || ''}`);
  // The hover tooltip of a worn piece: the game's own tooltip text (kept from the snapshot), under its icon and name.
  // A set has no tooltip in the game: it lists its pieces and what the set adds.
  // The order of a piece's tooltip sections (the user's, not the game's); the set bonus always goes before them.
  const TIP_ORDER = ['appearance bonus', 'basic features', 'characteristics', 'tarot fusion', 'soul fusion', 'modification slot'];
  const gearTip = (g) => () => {
    const box = h('div', { class: 'tt' });
    if (g.kind === 'set') {
      const worn = /\((\d+)\/(\d+),/.exec(String(g.item || ''));
      box.append(tipHead(gearIcon(g), setTitle(g), [g.slot, worn ? `${worn[1]} of ${worn[2]} worn` : '', setIds(g).map((id) => `set ${id}`).join(', ')].filter(Boolean).join(' · ')));
      if (g.shown_as) box.append(h('div', { class: 'tt-sec', text: g.slot === 'Fusion set' ? 'Crystals' : 'Set pieces' }), ...String(g.shown_as).split(', ').map((p) => h('div', { class: 'tt-line', text: p })));
      if (g.parts.length) box.append(h('div', { class: 'tt-sec', text: 'Activation effects' }),
        ...g.parts.map(({ row, part }) => h('div', { class: 'tt-line', text: `${row.bonus} ${part.value === null || part.value === undefined ? '' : fmt(part.value, row.unit, true)}${isTrue(part.digi) ? ' (digitalized)' : ''}` })));
      return box;
    }
    // The first line is the piece's in-game name; one the game shows untranslated gives way to the English item name.
    const raw = String(g.tip || '').replace(/\r/g, ''), nl = raw.indexOf('\n'), name = (nl < 0 ? raw : raw.slice(0, nl)).trim();
    const lines = raw ? [null, ...tipLines(nl < 0 ? '' : raw.slice(nl + 1))] : [];
    box.append(tipHead(gearIcon(g), name && !/[ぁ-ゖァ-ヺ一-鿿]/.test(name) ? name : g.item, g.slot));
    // The stats block first, then the sections in the order the user asked for (not the game's); any other section
    // (set bonuses...) keeps its place after those.
    const blocks = [[]];
    lines.slice(1).forEach((el) => { if (el.classList.contains('tt-sec')) blocks.push([el]); else blocks[blocks.length - 1].push(el); });
    const rank = (b) => { const i = TIP_ORDER.indexOf(b[0].textContent.toLowerCase()); return i < 0 ? TIP_ORDER.length : i; };
    // The game prints a set's pieces and effects on some pieces only (Spiral ones, not the Resolute ones): that text
    // is left out, and every piece of a complete set gets the same set section, from the set's own row.
    const [stats, ...all] = blocks;
    const secs = all.filter((b) => !/^(set bonus \d+|set pieces|activation effects)$/i.test(b[0].textContent.trim()));
    const noSetLine = (el) => !/^Set Bonus \d+$/i.test(el.textContent.trim());   // the heading line of the game's set text
    // What the piece is (type, affinity, sex, level, durability) stays on top; its own stats become a "Basic Features"
    // section; the set bonus is a section too; then all sections go in TIP_ORDER.
    const isInfo = (el) => /^\s*(Type|Affinity|Sex|Level|Durability)\s*:/.test(el.textContent);
    const own = stats.filter(noSetLine), basic = own.filter((el) => !isInfo(el));
    box.append(...own.filter(isInfo));
    // Fusion great-success bonuses, the way the game's Features tab writes them: "+127 (82 + 45) MAX"
    (g.fuse || []).forEach((f) => {
      const el = basic.find((b) => b.textContent.trim().startsWith(f.stat + ':')), m = el && /([+-]?\d+)\s*$/.exec(el.textContent);
      if (!m) return;
      el.append(h('span', { class: 'tt-fuse', text: ` (${Number(m[1]) - f.bonus} + ${f.bonus})` }));
      if (isTrue(f.max)) el.append(h('span', { class: 'tt-max', text: 'MAX' }));
    });
    if (basic.length) { basic[0].classList.remove('tt-gap'); secs.unshift([h('div', { class: 'tt-sec', text: 'Basic Features' }), ...basic]); }
    const sets = S.v.gear.filter((s) => s.kind === 'set' && (s.piece_ids || []).includes(g.item_id)).flatMap((s) => [
      h('div', { class: 'tt-sec', text: `Set bonus · ${setIds(s).map((id) => `set ${id}`).join(', ')}` }),
      h('div', { class: 'tt-line', text: `Pieces: ${s.shown_as || ''}` }),
      ...s.parts.map(({ row, part }) => h('div', { class: 'tt-line', text: `* ${row.bonus} ${part.value === null || part.value === undefined ? '' : fmt(part.value, row.unit, true)}${isTrue(part.digi) ? ' (digitalized)' : ''}` }))]);
    box.append(...sets, ...secs.map((b, i) => ({ b, i })).sort((x, y) => rank(x.b) - rank(y.b) || x.i - y.i).flatMap((x) => x.b.filter(noSetLine)));
    if (!lines.length) box.append(h('div', { class: 'tt-line', text: 'No tooltip was read for this piece.' }));
    return box;
  };
  // The hover tooltip of any other bonus source: its icon and name, and the game's description when it is a skill.
  const sourceTip = (p, iconOf) => () => h('div', { class: 'tt' },
    tipHead(gearIcon(iconOf), String(p.source).replace(/ \((Other learned skills|Quest bonus[^)]*\)|Demonic Compendium[^)]*\))\)?$/, ''), KIND[p.kind] || p.kind || ''),
    ...tipLines(p.tip), p.note ? h('div', { class: 'tt-line tt-gap', text: p.note }) : null);
  // A piece's icon from the game (32x32, pixel-crisp); an empty box of the same size for sets and pieces without one.
  // A set shows its pieces' icons as a cluster of small round "orbs" (up to four).
  const gearIcon = (g) => {
    const one = g.icon || ((g.icons || []).length === 1 ? g.icons[0] : null);   // a lone icon is shown whole, not as an orb
    if (one) return h('img', { class: 'slot-icon', src: one, alt: '' });
    const orbs = (g.icons || []).slice(0, 4);
    return h('span', { class: 'slot-icon' + (orbs.length ? ` orbs orbs-${orbs.length}` : '') }, orbs.map((src) => h('img', { src, alt: '' })));
  };
  function gearDetail(g) {
    const card = h('section', { class: 'card detail' });
    if (!g) return card;
    const isSet = g.kind === 'set';
    const m = /\((\d+)\/(\d+), (?:fusion )?set (\d+)\)\s*$/.exec(String(g.item || ''));
    card.append(h('div', { class: 'detail-head' }, g.icon || (g.icons || []).length ? gearIcon(g) : null,
      h('div', null, h('span', { class: 'stat-label', text: g.slot }), h('h2', { class: 'detail-title' }, isSet ? setTitle(g) : g.item,
        !isSet && g.item_id ? h('span', { class: 'detail-id', 'data-tip': 'Item id', text: `#${g.item_id}` }) : null))));
    const meta = h('div', { class: 'detail-meta' });
    if (isSet && m) meta.append(h('span', { class: 'tag', text: `${m[1]} of ${m[2]} worn` }), ...(setIds(g).length ? setIds(g) : [m[3]]).map((id) => h('span', { class: 'tag', text: `Set ${id}` })));
    // Tarot and soul fusion: hovering the label shows what the crystal gives, as the game's own tooltip of the piece
    // says it (its "[Tarot Fusion]" / "[Soul Fusion]" part: the crystal's name, then its lines up to the next blank line).
    const fusionTag = (label, crystal, section) => {
      const tag = h('span', { class: 'tag', tabindex: 0, text: `${label}: ${crystal}` });
      const all = String(g.tip || '').replace(/\r/g, '').split('\n'), at = all.findIndex((l) => l.trim() === `[${section}]`);
      if (at < 0) return tag;
      let end = at + 1;
      while (end < all.length && all[end].trim() && !/^\[.+\]$/.test(all[end].trim())) end++;
      const lines = all.slice(at + 1, end).map((l) => l.trim()).filter((l) => l && l !== String(crystal).trim());
      if (!lines.length) return tag;
      return withTip(tag, () => h('div', null, tipHead(null, crystal, label), ...tipLines(lines.join('\n'))));
    };
    if (!isSet && g.tarot) meta.append(fusionTag('Tarot', g.tarot, 'Tarot Fusion'));
    if (!isSet && g.soul) meta.append(fusionTag('Soul fusion', g.soul, 'Soul Fusion'));
    // fusion great-success bonuses on the piece's own stats ("MAX" = at the top of the game's table); the user calls it Limit
    if (!isSet && (g.fuse || []).length) meta.append(h('span', { class: 'tag', 'data-tip': 'Limit bonus on its basic stats',
      text: 'Limit: ' + g.fuse.map((f) => `${f.stat} +${f.bonus}${isTrue(f.max) ? ' MAX' : ''}`).join(', ') }));
    if (meta.children.length) card.append(meta);
    if (isSet && g.shown_as) card.append(h('p', { class: 'note', text: `Pieces: ${g.shown_as}` }));
    if (!isSet && g.shown_as && g.shown_as !== g.item) card.append(h('p', { class: 'note', text: `In game: ${g.shown_as}` }));

    if (!g.parts.length) { card.append(h('p', { class: 'note', text: 'No bonuses read for this piece.' })); return card; }
    KIND_ORDER.forEach((kind) => {
      // Several lines of one kind for the same bonus (three mod slots adding LNG-R, say) are shown as one, added up.
      const merged = new Map();
      g.parts.filter((c) => c.part.kind === kind).forEach(({ row, part }) => {
        const id = `${row.row}|${isTrue(part.digi)}|${part.note || ''}`, known = merged.get(id);
        if (known && known.part.value !== null && part.value !== null) { known.part = { ...known.part, value: round2(n0(known.part.value) + n0(part.value)) }; known.n++; }
        else merged.set(id + (known ? '|' + merged.size : ''), { row, part, n: 1 });
      });
      const list = [...merged.values()].sort((a, b) => (isTrue(b.row.counted) - isTrue(a.row.counted)) || Math.abs(n0(b.part.value)) - Math.abs(n0(a.part.value)));
      if (!list.length) return;
      const grp = h('div', { class: 'kv-group' });
      if (!isSet) grp.append(h('div', { class: 'kv-head', text: KIND[kind] || kind }));
      list.forEach(({ row, part, n }) => {
        const counted = isTrue(row.counted), digiOnly = isTrue(part.digi);
        const off = !counted || (digiOnly && !digi());
        grp.append(h('div', { class: 'kv' + (off ? ' off' : '') },
          h('div', { class: 'kv-label' }, h('span', { text: (row.group === 'Partner' ? 'Partner: ' : '') + row.bonus }),
            n > 1 ? h('span', { class: 'src-count', style: 'margin-left:0', text: `${n} lines added up` }) : null,
            digiOnly ? h('span', { class: 'tag ' + (digi() ? 'tag-digi' : 'tag-off'), text: 'Digi only' }) : null,
            !counted ? h('span', { class: 'tag tag-off', 'data-tip': 'Depends on the fight, so it is not added to the totals', text: 'Situational' }) : null),
          h('span', { class: 'kv-value', text: part.value === null || part.value === undefined ? '' : fmt(part.value, row.unit, true) })));
      });
      card.append(grp);
    });
    return card;
  }

  // Compare: one row per piece, one column per bonus. Any bonus the gear gives can be a column; the panel on the
  // right picks which ones are shown. The choice is remembered; bonuses never seen before follow the defaults.
  const COL_DEFAULT_GROUPS = new Set(['Combat', 'Stats']);
  try { S.gear.cols = JSON.parse(store.get('cols') || '{}') || {}; } catch (e) { S.gear.cols = {}; }
  const colShown = (c) => (Object.prototype.hasOwnProperty.call(S.gear.cols, c.id) ? !!S.gear.cols[c.id] : COL_DEFAULT_GROUPS.has(c.row.group));
  const colLabel = (c) => (c.row.group === 'Partner' ? 'Partner: ' : '') + c.row.bonus;

  // Short names ("Use Short Names"): wording rules applied in order to any bonus name, so new bonuses get one too.
  const SHORT_RULES = [
    [/final critical correction/gi, 'FCC'],
    [/limit break (?:damage )?cap/gi, 'LBCap'], [/limit break chance/gi, 'LBC'], [/limit break power/gi, 'LBP'],
    [/technical attack chance/gi, 'TAC'], [/technical attack power/gi, 'TAP'],
    [/pursuit chance/gi, 'PC'], [/pursuit power/gi, 'PP'],
    [/close-range|CLS-R/gi, 'CLS'], [/long-range|LNG-R/gi, 'LNG'],
    [/critical/gi, 'Crit'],
    [/damage dealt/gi, 'Dmg'], [/damage taken/gi, 'Taken'], [/\bdamage\b/gi, 'Dmg'],
    [/ skill boost/gi, ''], [/movement speed/gi, 'Move'], [/cooldown/gi, 'CD'], [/cast time/gi, 'Cast'],
    [/regeneration/gi, 'Regen'], [/incantation/gi, 'Cast'], [/\baffinity\b/gi, 'Aff'], [/\bresist(?:ance)?\b/gi, 'Res'], [/\bdefense\b/gi, 'Def'],
    [/\bsupport\b/gi, 'SUPP'], [/\bspell\b/gi, 'SPELL'],
    [/P\.Def/g, 'PDEF'], [/M\.Def/g, 'MDEF'], [/Max (HP|MP)/g, 'Max$1'],
    [/E?XP earned/gi, 'XP'], [/COMP Stocks/gi, 'COMP'],
    [/[- ]based skills? skill stack/gi, ' Stack'], [/[- ]based skill effects?/gi, ' Skill'], [/ based /gi, ' '],
    [/\(only with certain partners\)/gi, '(some partners)'], [/\((?:against|vs) (\w+) enemies\)/gi, '(vs $1)'],
    [/\(when Partner's Friendship is at ("[^"]+") or higher\)/gi, '($1+)'], [/\(when Partner's Friendship is at ("[^"]+")\)/gi, '($1)'],
  ];
  const shortName = (name) => SHORT_RULES.reduce((s, [rx, to]) => s.replace(rx, to), String(name || '')).replace(/\s+/g, ' ').trim();
  const colShort = (c) => (c.row.group === 'Partner' ? 'Ptr ' : '') + shortName(c.row.bonus);
  const colHead = (c) => (S.gear.short ? colShort(c) : colLabel(c));
  const colMatches = (c, q) => colLabel(c).toLowerCase().includes(q) || colShort(c).toLowerCase().includes(q);

  function gearCompare() {
    const cols = S.v.cols, rows = S.v.gear;
    const val = (c, g) => { const e = c.per.get(g.key); return e ? (digi() ? e.digi : e.normal) : 0; };
    const tableHost = h('div', { class: 'tbl-wrap' });
    const listHost = h('div', { class: 'col-list' });
    const countEl = h('span', { class: 'card-sub' });
    const saveCols = () => store.set('cols', JSON.stringify(S.gear.cols));
    const setShown = (list, on) => { list.forEach((c) => { S.gear.cols[c.id] = on; }); saveCols(); drawTable(); drawPanel(); };

    function drawTable() {
      const shown = cols.filter(colShown);
      countEl.textContent = `${shown.length} of ${cols.length} shown`;
      if (!shown.length) { tableHost.replaceChildren(h('div', { class: 'empty-row', text: 'No columns are shown. Tick some in the list on the right.' })); return; }
      const sort = S.gear.sort && shown.find((c) => c.id === S.gear.sort.id) ? S.gear.sort : null;
      const list = rows.slice();
      if (sort) {
        const c = shown.find((x) => x.id === sort.id);
        // pieces that add nothing to this bonus always go last
        list.sort((a, b) => { const va = val(c, a), vb = val(c, b); return (!va !== !vb) ? (va ? -1 : 1) : sort.dir * (vb - va); });
      }
      const head = h('tr', null, h('th', { scope: 'col', class: 'pin', text: 'Piece' }), shown.map((c) => {
        const dir = sort && sort.id === c.id ? sort.dir : 0, counted = isTrue(c.row.counted);
        return h('th', { scope: 'col', class: 'num', 'aria-sort': dir === 1 ? 'descending' : dir === -1 ? 'ascending' : null },
          h('button', { type: 'button', class: 'th-btn', 'data-tip': `${colLabel(c)}${counted ? '' : ' (only in some fights)'}. Click to sort.`,
            onclick: () => { S.gear.sort = dir === 0 ? { id: c.id, dir: 1 } : dir === 1 ? { id: c.id, dir: -1 } : null; drawTable(); } },
            h('span', null, colHead(c), counted ? null : h('span', { class: 'sit', text: ' *' })), dir ? h('span', { class: 'sort-mark', text: dir === 1 ? '▼' : '▲' }) : null));
      }));
      const body = h('tbody', null, list.map((g) => h('tr', null,
        h('td', { class: 'pin' }, h('div', { class: 'pin-row', 'data-tip': g.kind === 'set' ? `${setTitle(g)} (${g.shown_as || ''})` : (g.tarot ? `${g.item} · Tarot: ${g.tarot}` : g.item) },
          h('span', { class: 'slot-name', text: g.kind === 'set' ? setLabel(g) : g.slot }),
          h('span', { class: 'slot-item', text: g.kind === 'set' ? setTitle(g).replace(/^Set with /, '') : g.item }))),
        shown.map((c) => { const v = val(c, g); return h('td', { class: 'num' + (sort && sort.id === c.id ? ' emph' : ''), text: v ? fmt(v, c.row.unit, true) : '' }); }))));
      const foot = h('tfoot', null, h('tr', null, h('td', { class: 'pin', text: 'All gear' }), shown.map((c) => {
        if (!isTrue(c.row.counted)) return h('td', { class: 'num zero', 'data-tip': 'Depends on the fight, so it is not added up', text: '—' });
        const t = round2(rows.reduce((a, g) => a + val(c, g), 0));
        const cap = capOfBonus(c.row), whole = round2(t + n0(c.row.expertise));   // the cap counts expertise too
        if (cap !== null && whole >= cap) return h('td', { class: 'num capped', 'data-tip': whole > cap
          ? `With expertise this is ${fmt(whole, c.row.unit)}: ${fmt(round2(whole - cap), c.row.unit).replace(/^\+/, '')} over the ${fmt(cap, c.row.unit).replace(/^\+/, '')} cap, which adds nothing`
          : `With expertise this is exactly at the ${fmt(cap, c.row.unit).replace(/^\+/, '')} cap`, text: (t ? fmt(t, c.row.unit, true) : '0') + (whole > cap ? ' ▲' : ' ●') });
        return h('td', { class: 'num', text: t ? fmt(t, c.row.unit, true) : '0' });
      })));
      const scroll = tableHost.firstElementChild, left = scroll ? scroll.scrollLeft : 0, top = scroll ? scroll.scrollTop : 0;
      const box = h('div', { class: 'cmp-scroll' }, h('table', { class: 'tbl' }, h('thead', null, head), body, foot));
      tableHost.replaceChildren(box);
      box.scrollLeft = left; box.scrollTop = top;
    }

    function drawPanel() {
      const q = S.gear.colq.trim().toLowerCase();
      listHost.replaceChildren();
      BONUS_GROUPS.forEach(([gid, short]) => {
        const mine = cols.filter((c) => c.row.group === gid), hit = q ? mine.filter((c) => colMatches(c, q)) : mine;
        if (!hit.length) return;
        const on = hit.filter(colShown).length;
        const all = h('input', { type: 'checkbox', 'aria-label': `All ${short} columns`, onchange: () => setShown(hit, all.checked) });
        all.checked = on === hit.length; all.indeterminate = on > 0 && on < hit.length;
        listHost.append(h('label', { class: 'col-item col-group' }, all, h('span', { text: short }), h('span', { class: 'src-count', text: `${on}/${hit.length}` })));
        hit.forEach((c) => {
          const box = h('input', { type: 'checkbox', onchange: () => setShown([c], box.checked) });
          box.checked = colShown(c);
          // With short names on, the list leads with the heading as the table shows it, and keeps the full name beside it.
          const short = shortName(c.row.bonus);
          listHost.append(h('label', { class: 'col-item' }, box, S.gear.short && short !== c.row.bonus
            ? h('span', null, short, h('span', { class: 'col-full', text: c.row.bonus }))
            : h('span', { text: c.row.bonus })));
        });
      });
      if (!listHost.children.length) listHost.append(h('p', { class: 'note', text: `No column matches “${S.gear.colq.trim()}”.` }));
    }
    const visible = () => { const q = S.gear.colq.trim().toLowerCase(); return q ? cols.filter((c) => colMatches(c, q)) : cols; };
    const wrap = h('div', { class: 'cmp' + (S.gear.short ? ' short' : '') });
    const shortBox = h('input', { type: 'checkbox', class: 'switch', role: 'switch', onchange: () => {
      S.gear.short = shortBox.checked; store.set('short', S.gear.short ? '1' : '0');
      wrap.classList.toggle('short', S.gear.short); drawTable(); drawPanel();
    } });
    shortBox.checked = S.gear.short;
    const find = h('input', { class: 'input', type: 'search', placeholder: 'Find a column', 'aria-label': 'Find a column', value: S.gear.colq, spellcheck: 'false', 'data-search': true,
      oninput: () => { S.gear.colq = find.value; drawPanel(); } });
    const panel = h('aside', { class: 'card cmp-panel', 'aria-label': 'Columns' },
      h('div', { class: 'card-head' }, h('h3', { class: 'card-title', text: 'Columns' }), countEl),
      h('label', { class: 'switch-row', 'data-tip': 'Column headings like Crit, FCC, LBC, TAP. Hover a heading for its full name.' }, h('span', { text: 'Use Short Names' }), shortBox),
      h('div', { class: 'search', style: 'max-width:none;flex:none' }, icon('search'), find),
      h('div', { class: 'field-row' },
        h('button', { type: 'button', class: 'btn btn-sm', 'data-tip': 'Show every column in the list below', onclick: () => setShown(visible(), true) }, 'Show all'),
        h('button', { type: 'button', class: 'btn btn-sm', 'data-tip': 'Hide every column in the list below', onclick: () => setShown(visible(), false) }, 'Hide all'),
        h('button', { type: 'button', class: 'btn btn-sm', 'data-tip': 'Back to the starting choice: combat and stats', onclick: () => { S.gear.cols = {}; saveCols(); drawTable(); drawPanel(); } }, 'Reset')),
      listHost);
    drawTable(); drawPanel();
    wrap.append(h('div', { class: 'cmp-main' }, tableHost,
      h('p', { class: 'note', style: 'margin-top:10px', text: `${digi() ? 'Digitalized: includes the lines that only apply while digitalized.' : 'Normal: lines that only apply while digitalized are left out.'} Each cell is the piece itself plus its tarot, soul fusion, mods and appearance bonus. Columns marked * only apply in some fights.` })), panel);
    return wrap;
  }

  function pageGear() {
    const page = h('div', { class: 'page' + (S.gear.view === 'table' ? ' wide' : '') });
    const view = h('div', { class: 'seg', role: 'group', 'aria-label': 'View' },
      [['pieces', 'By piece'], ['table', 'Compare']].map(([id, label]) => h('button', { type: 'button', 'aria-pressed': String(S.gear.view === id), onclick: () => { S.gear.view = id; render(); } }, label)));
    page.append(pageHead(S.gear.view === 'pieces'
      ? 'Pick a piece to see everything it adds: its own bonuses, tarot, soul fusion, mods and appearance bonus.'
      : 'Every piece side by side. Choose the columns on the right; click a column heading to sort by it.', view));
    if (S.gear.view === 'table') { page.append(gearCompare()); return page; }

    const items = S.v.gear.filter((g) => g.kind !== 'set'), sets = S.v.gear.filter((g) => g.kind === 'set');
    const detailHost = h('div');
    const list = h('div', { class: 'slot-list', role: 'listbox', 'aria-label': 'Equipped gear' });
    const select = (g) => {
      S.gear.sel = g.row;
      list.querySelectorAll('.slot').forEach((b) => b.setAttribute('aria-selected', String(b.dataset.row === g.row)));
      detailHost.replaceChildren(gearDetail(g));
    };
    const slotBtn = (g) => h('button', { type: 'button', class: 'slot', role: 'option', 'data-row': g.row, 'aria-selected': String(g.row === S.gear.sel), onclick: () => select(g) },
      withTip(gearIcon(g), gearTip(g)),
      h('span', { class: 'slot-name', text: g.kind === 'set' ? setLabel(g) : g.slot }),
      h('span', { class: 'slot-item', 'data-tip': g.kind === 'set' ? setTitle(g) : null, text: g.kind === 'set' ? setTitle(g).replace(/^Set with /, '') : g.item }),
      h('span', { class: 'slot-n', 'data-tip': `${g.parts.length} bonuses`, text: g.parts.length || '' }));
    list.append(...items.map(slotBtn));
    if (sets.length) list.append(h('div', { class: 'list-head', text: 'Complete sets' }), ...sets.map(slotBtn));
    list.addEventListener('keydown', (ev) => {
      if (ev.key !== 'ArrowDown' && ev.key !== 'ArrowUp') return;
      const btns = [...list.querySelectorAll('.slot')], i = btns.indexOf(document.activeElement);
      const next = btns[Math.max(0, Math.min(btns.length - 1, i + (ev.key === 'ArrowDown' ? 1 : -1)))];
      if (next) { next.focus(); next.click(); ev.preventDefault(); }
    });
    detailHost.append(gearDetail(S.v.gear.find((g) => g.row === S.gear.sel)));
    page.append(h('div', { class: 'gear-layout' }, list, detailHost));
    return page;
  }

  // ---------- Expertise and Digitalize: two pages over the same list of passives ----------
  // The expertise data holds both: the "Digitalize: <race>" groups are the Digitalization Level passives (Digitalize
  // page), every other group is expertise, learned skills, quest and compendium bonuses (Expertise page).
  const isDlevel = (r) => /^Digitalize: /.test(String(r.expertise || ''));
  /** What Digitalization Level passives add to one Bonuses row (they sit inside its expertise share). */
  const shareOf = (r, kinds) => round2((r.parts || []).filter((p) => kinds.includes(p.kind)).reduce((a, p) => a + n0(p.value), 0));
  const dlevelShare = (r) => shareOf(r, ['dlevel']);
  /** Quest bonus and Demonic Compendium rows: they ride along in the expertise data, and are shown on the Bonuses page. */
  const isExtra = (r) => /^(Quest bonus|Demonic Compendium)/.test(String(r.expertise || ''));
  /** The "adds in total" card: one tile per bonus. */
  /**
   * A section of a page: its heading outside the box (the title, a short line beside it, anything else at the right end),
   * then the box. The same on every page, so a card never carries its own title.
   */
  const sect = (title, sub, right, ...body) => h('div', { class: 'sect' },
    h('div', { class: 'toolbar' }, h('h2', { class: 'section-title', text: title }),
      sub ? (typeof sub === 'string' ? h('span', { class: 'card-sub', text: sub }) : sub) : null,
      right ? [h('span', { class: 'toolbar-spacer' }), right] : null),
    ...body);
  const totalsCard = (title, sub, tiles, none, note) => h('section', { class: 'card' },
    title ? h('div', { class: 'card-head' }, h('h3', { class: 'card-title', text: title }), h('span', { class: 'card-sub', text: sub })) : null,   // no title: the page gives the section its own heading
    tiles.length ? h('div', { class: 'tiles' }, tiles.map(([r, v]) => h('div', { class: 'tile' },
      h('span', { class: 'tile-label', text: (r.group === 'Partner' ? 'Partner: ' : '') + r.bonus }), h('span', { class: 'tile-value', text: fmt(v, r.unit, true) }))))
      : h('p', { class: 'note', text: none }),
    note ? h('p', { class: 'note', text: note }) : null);
  /** A skill's own icon from the game (32x32, shown smaller); an empty box of the same size when there is none, so names stay in line. */
  const skillIcon = (r) => {
    const box = h('span', { class: 'skill-ico', 'aria-hidden': 'true' });
    if (r.icon && S.meta.skillIcons) {
      const img = h('img', { src: S.meta.skillIcons + r.icon + '.png', alt: '', loading: 'lazy' });
      img.onerror = () => img.remove();
      box.append(img);
    }
    return box;
  };
  /** The searchable, collapsible list of passives, one group per expertise (or race). `st` keeps the search and the closed groups. */
  function passiveList(page, rows, st, o) {
    const names = []; rows.forEach((r) => { if (!names.includes(r.expertise)) names.push(r.expertise); });
    const listHost = h('div');
    const draw = () => {
      listHost.replaceChildren();
      const q = st.q.trim().toLowerCase();
      let shown = 0;
      names.forEach((name) => {
        const mine = rows.filter((r) => r.expertise === name);
        const hit = !q || name.toLowerCase().includes(q) ? mine : mine.filter((r) => `${r.skill} ${r.effects}`.toLowerCase().includes(q));
        if (!hit.length) return;
        shown++;
        const rk = S.v.ranks.get(name);
        const anyCounted = mine.some((r) => isTrue(r.counted));
        const det = h('details', { class: 'exp-group', open: q ? true : !st.closed.has(name) },
          h('summary', null, h('span', { class: 'chev' }, icon('chevron')), h('span', { class: 'exp-name', text: o.title ? o.title(name) : name }),
            h('span', { class: 'exp-meta', text: [rk ? `Class ${rk.class}, rank ${rk.rank}` : null,
              /^(Quest bonus|Demonic Compendium)/.test(name) ? (anyCounted ? (mine.length === 1 ? '1 bonus' : `${mine.length} bonuses`) : null)
                : mine.length === 1 ? '1 passive' : `${mine.length} passives`].filter(Boolean).join(' · ') }),
            o.tag ? o.tag(name) : null,
            anyCounted ? null : h('span', { class: 'tag tag-off', text: 'Not counted' }),
            // closed: its passives as a row of small icons (hidden by the stylesheet while the group is open)
            // (a passive the game has no icon for is left out of the row; it is in the list when the group is open)
            o.noIcons ? null : h('span', { class: 'exp-icons' }, mine.filter((r) => r.icon).map((r) => {
              const ic = skillIcon(r);
              if (!isTrue(r.counted)) ic.classList.add('off');
              ic.dataset.tip = [r.skill || '—', r.rank ? `(${r.rank})` : null].filter(Boolean).join(' ') + (r.effects ? ': ' + String(r.effects).split('; ').join(' · ') : '');
              return ic;
            }))),
          h('div', { class: 'exp-body' }, hit.map((r) => h('div', { class: 'passive' + (isTrue(r.counted) ? '' : ' off') },
            h('span', { class: 'rank', 'data-tip': o.rankTip, text: r.rank || '' }), h('span', { class: 'skill' }, o.noIcons ? null : skillIcon(r), h('span', { text: r.skill || '—' })),
            h('span', { class: 'fx' }, String(r.effects || '').split('; ').filter(Boolean).map((t) => h('span', { text: t })))))));
        det.addEventListener('toggle', () => { if (q) return; if (det.open) st.closed.delete(name); else st.closed.add(name); });
        listHost.append(det);
      });
      if (!shown) listHost.append(h('div', { class: 'tbl-wrap' }, h('div', { class: 'empty-row', text: q ? `Nothing matches “${st.q.trim()}”.` : o.empty })));
    };
    page.append(h('div', { class: 'sect' }, h('div', { class: 'toolbar' },
      h('h2', { class: 'section-title', text: o.heading }), h('span', { class: 'toolbar-spacer' }),
      o.search ? searchBox(st.q, o.search, (v) => { st.q = v; draw(); }) : null,
      h('button', { type: 'button', class: 'btn btn-sm', onclick: () => { st.closed.clear(); draw(); } }, 'Expand all'),
      h('button', { type: 'button', class: 'btn btn-sm', onclick: () => { names.forEach((n) => st.closed.add(n)); draw(); } }, 'Collapse all')), listHost));
    draw();
  }

  function pageExpertise() {
    const rows = S.data.expertise.filter((r) => !isDlevel(r) && !isExtra(r));
    const page = h('div', { class: 'page' });
    page.append(pageHead('Passive skills your expertise ranks unlock (this assumes you acquired every one), and your other learned passives. Digitalization Levels are on the Digitalize page; the quest bonus and Demonic Compendium are on Bonuses.'));
    // the expertise share of each bonus, without what the Digitalization Levels, quest bonus and compendium put into it
    const totals = S.data.bonuses.filter((r) => isTrue(r.counted)).map((r) => [r, round2(n0(r.expertise) - shareOf(r, ['dlevel', 'quest', 'compendium']))]).filter(([, v]) => v);
    page.append(sect('What expertise adds in total', `${rows.filter((r) => isTrue(r.counted)).length} passives counted`, null,
      totalsCard(null, null, totals, 'No expertise bonuses counted yet.')));
    passiveList(page, rows, S.exp, { heading: 'Every passive', search: 'Search an expertise, passive or effect', rankTip: 'The rank that unlocks this passive', empty: 'No expertise read yet. Take a snapshot.' });
    return page;
  }

  // ---------- Digitalize ----------
  /**
   * The digitalize demon's own skills (digitalize.json "skills"), for the demon card: `icons` is the one-line version
   * (the icons of the skills in its slots, no features; the game's description on hover), `parts` the opened one
   * (the skills, then the features: kind, icon and name, and what each gives). A skill you don't get while
   * digitalized is dimmed (the reason, "why" in the data, is not shown: the user didn't want the label). Null when the saved data has no skills yet.
   */
  function demonSkills(DG) {
    if (!DG || !Array.isArray(DG.skills)) return null;
    const row = (s) => {
      const name = h('span', { class: 'skill', tabindex: 0 }, skillIcon(s), h('span', { text: s.name }));
      if (s.tip) withTip(name, () => h('div', null, tipHead(skillIcon(s), s.name, s.kind === 'Equipment' ? 'Demon equipment' : s.kind), ...tipLines(s.tip)));
      const fx = (s.effects || []).length ? s.effects
        : s.kind === 'Active' && isTrue(s.counted) ? ['You can use it while digitalized'] : isTrue(s.counted) ? ['No stat bonus'] : [];
      return h('div', { class: 'passive' + (isTrue(s.counted) ? '' : ' off') },
        h('span', { class: 'rank', text: s.kind }), name,
        h('span', { class: 'fx' }, fx.map((t) => h('span', { text: t })),
          s.replaces ? h('span', { text: `In place of the feature ${s.replaces}` }) : null));
    };
    const isFeature = (s) => s.kind === 'Feature' || s.kind === 'Equipment';
    const slots = DG.skills.filter((s) => !isFeature(s)), feats = DG.skills.filter(isFeature);
    const count = (part) => `${part.filter((s) => isTrue(s.counted)).length} of ${part.length} apply while digitalized`;
    const icons = slots.length ? h('span', { class: 'dg-skill-icons' }, slots.map((s) => {
      const ic = skillIcon(s);
      ic.removeAttribute('aria-hidden'); ic.tabIndex = 0; ic.setAttribute('role', 'img'); ic.setAttribute('aria-label', s.name);
      if (!isTrue(s.counted)) ic.classList.add('off');
      return withTip(ic, () => h('div', null, tipHead(skillIcon(s), s.name, s.kind), ...tipLines(s.tip)));
    })) : null;
    const part = (label, rows, none) => [
      h('div', { class: 'dg-skill-head' }, h('h4', { class: 'section-title', text: label }), rows.length ? h('span', { class: 'card-sub', text: count(rows) }) : null),
      rows.length ? h('div', { class: 'dg-skill-list' }, rows.map(row)) : h('p', { class: 'note', text: none })];
    return { icons, parts: h('div', { class: 'demon-skills' }, part('Skills', slots, 'No skills in its slots.'), part('Features', feats, 'It has no features.')) };
  }

  /**
   * One COMP demon as a card, for the demon dropdown: who it is, the share of its stats you get, the skills in its
   * slots and the key bonuses it would add ("offer", worked out by the reader with your learned boosts).
   * `pick` runs on a click; `on` marks the demon in use.
   */
  /**
   * Where a COMP demon stands for digitalizing ("unlock", from the game's Digitalization Level table and your levels):
   * unlocked, locked until a level of its race, or not named by any level. Plus a mark when the saved digitalize demon
   * is no longer among the COMP demons read (seenAt empty: it was only kept because it is the saved one).
   */
  function demonUnlockTags(d) {
    const U = d.unlock, tags = [];
    if (U && isTrue(U.listed) && U.raceLevel !== null && U.raceLevel !== undefined) {
      tags.push(isTrue(U.reached)
        ? h('span', { class: 'tag', 'data-tip': `${U.race} Digitalization Level ${U.level} unlocks it; you are at level ${U.raceLevel}.`, text: `Unlocked · ${U.race} DLv ${U.level}` })
        : h('span', { class: 'tag tag-off', 'data-tip': n0(U.raceLevel) ? `Your ${U.race} Digitalization Level is ${U.raceLevel}.` : `You have no ${U.race} Digitalization Level yet.`, text: `Locked · needs ${U.race} DLv ${U.level}` }));
    } else if (U && !isTrue(U.listed)) {
      tags.push(h('span', { class: 'tag tag-off', 'data-tip': `No ${U.race} Digitalization Level names this demon in the game data, so it may not be usable for digitalizing. The data does not say more.`, text: 'Not on any Digitalization Level' }));
    }
    if (!d.seenAt && !d.inGame) tags.push(h('span', { class: 'tag tag-off', 'data-tip': 'It was not among the COMP demons last read from the game. It is still here because it is your saved digitalize demon.', text: 'Not in your COMP' }));
    return tags.length ? h('span', { class: 'dgt-tags' }, tags) : null;
  }

  function demonTile(d, on, pick) {
    const O = d.offer || null;
    const pic = h('span', { class: 'demon-pic ' + alignClass(O && O.alignment), 'aria-hidden': 'true' }, h('span', { text: String(d.name || '?').charAt(0) }));
    if (d.icon) pic.append(h('img', { src: d.icon, alt: '' }));
    const slots = O ? (O.skills || []).filter((s) => s.kind !== 'Feature' && s.kind !== 'Equipment') : [];
    const icons = slots.map((s) => {
      const ic = skillIcon(s);
      if (!isTrue(s.counted)) ic.classList.add('off');
      return withTip(ic, () => h('div', null, tipHead(skillIcon(s), s.name, s.kind), ...tipLines(s.tip)));
    });
    const adds = O ? COMBAT.filter(([k]) => n0(O.combat && O.combat[k])).map(([k, label, unit]) => h('span', { class: 'dgt-add' }, h('span', { text: label }), h('b', { text: fmt(O.combat[k], unit, true) }))) : [];
    const hp = O && (O.stats || []).find((s) => s.stat === 'Max HP'), mp = O && (O.stats || []).find((s) => s.stat === 'Max MP');
    return h('button', { type: 'button', class: 'dg-tile' + (on ? ' on' : ''), 'aria-current': on ? 'true' : null, onclick: pick },
      h('span', { class: 'dgt-head' }, pic,
        h('span', { class: 'dgt-id' }, h('span', { class: 'dgt-name', text: d.name }),
          h('span', { class: 'dgt-line', text: [`Lv ${d.level}`, d.race, O && O.alignment ? alignName(O.alignment) : null, d.mitama ? `${d.mitama} Mitama` : null].filter(Boolean).join(' · ') })),
        on ? h('span', { class: 'tag tag-demon', text: 'In use' }) : null),
      demonUnlockTags(d),
      O ? h('span', { class: 'dgt-line' }, `${fmt(O.rate)}% of its stats`, hp ? ` · HP ${fmt(hp.value, '', true)}` : '', mp ? ` · MP ${fmt(mp.value, '', true)}` : '')
        : d.forceStack || d.equipment ? h('span', { class: 'dgt-line', text: [d.forceStack ? `force stack ${d.forceStack}` : null, d.equipment ? 'equipment' : null].filter(Boolean).join(' · ') }) : null,
      // listed by the game without its details (stats, skills, growth, Mitama): there is nothing to work out yet
      d.loaded === false ? h('span', { class: 'dgt-line', text: 'Not read yet. Open its status window in game (from the COMP), then load the demons again to see what it gives.' }) : null,
      icons.length ? h('span', { class: 'dg-skill-icons' }, icons) : null,
      adds.length ? h('span', { class: 'dgt-adds' }, adds) : null);
  }

  /**
   * The demons your Digitalization Levels have unlocked (digi-unlocks.json, from the game's level table): one line per
   * race you have a level in, its demons in the order the levels unlock them. One a higher level unlocks is dimmed
   * and says which level. A demon you have in the COMP demons seen is marked.
   */
  function unlockedDemons() {
    const races = (S.data.unlocks || []).filter((r) => (r.demons || []).length);
    const got = races.reduce((n, r) => n + r.demons.filter((d) => isTrue(d.unlocked)).length, 0);
    const all = races.reduce((n, r) => n + r.demons.length, 0);
    const chip = (d) => {
      const open = isTrue(d.unlocked);
      const pic = h('span', { class: 'demon-pic ' + alignClass(d.alignment), 'aria-hidden': 'true' }, h('span', { text: String(d.name || '?').charAt(0) }));
      if (d.icon) pic.append(h('img', { src: d.icon, alt: '' }));
      return h('span', { class: 'dgu-demon' + (open ? '' : ' off'), tabindex: 0,
        'data-tip': `${d.name}: unlocked at Digitalization Level ${d.level}` + (open ? '' : ', not reached yet') + (isTrue(d.in_comp) ? '. You have one in your COMP.' : '') },
        pic, h('span', { class: 'dgu-text' }, h('span', { class: 'dgu-name', text: d.name }),
          h('span', { class: 'dgt-line', text: `DLv ${d.level}` + (isTrue(d.in_comp) ? ' · in COMP' : '') })));
    };
    // Closed (the default): only a grid of small pictures, every race together. "Details" opens the lines per race.
    const open = store.get('dgu-open') === '1';
    const toggle = h('button', { type: 'button', class: 'btn btn-sm btn-ghost', 'aria-expanded': String(open), 'data-tip': open ? 'Back to the pictures' : 'By race, with names and the level that unlocks each',
      onclick: () => { store.set('dgu-open', open ? '0' : '1'); render(); } }, h('span', { class: 'chev' }, icon('chevron')), open ? 'Less' : 'Details');
    const small = (r, d) => {
      const pic = h('span', { class: 'demon-pic ' + alignClass(d.alignment) + (isTrue(d.unlocked) ? '' : ' off'), tabindex: 0, role: 'img', 'aria-label': d.name,
        'data-tip': `${d.name} · ${r.race} · Digitalization Level ${d.level}` + (isTrue(d.unlocked) ? '' : ', not reached yet') + (isTrue(d.in_comp) ? ' · in your COMP' : '') },
        h('span', { text: String(d.name || '?').charAt(0) }));
      if (d.icon) pic.append(h('img', { src: d.icon, alt: '' }));
      return pic;
    };
    return sect('Demons you have unlocked', all ? `${got} of ${all} in your races` : null, races.length ? toggle : null,
      !races.length ? h('div', { class: 'tbl-wrap' }, h('div', { class: 'empty-row', text: 'No Digitalization Levels read yet. Take a snapshot.' }))
        : !open ? h('section', { class: 'card dgu-small' }, races.flatMap((r) => r.demons.map((d) => small(r, d))))
          : h('section', { class: 'card dgu' }, races.map((r) => h('div', { class: 'dgu-race' },
            h('div', { class: 'dgu-head' }, h('span', { class: 'dgu-race-name', text: r.race }), h('span', { class: 'dgt-line', text: `DLv ${r.level}` })),
            h('div', { class: 'dgu-list' }, r.demons.map(chip))))));
  }

  function pageDigitalize() {
    const rows = S.data.expertise.filter(isDlevel), DG = S.data.digitalize;
    const page = h('div', { class: 'page' });
    page.append(pageHead('The demons you digitalize with, and the passives you unlocked.'));
    page.append(demonCard());
    page.append(unlockedDemons());
    const totals = S.data.bonuses.filter((r) => isTrue(r.counted)).map((r) => [r, dlevelShare(r)]).filter(([, v]) => v);
    // a section heading like the page's other sections (the card then has no title of its own)
    page.append(sect('What Digitalization Levels add in total', `${rows.filter((r) => isTrue(r.counted)).length} passives counted`, null,
      totalsCard(null, null, totals, 'No Digitalization Level bonuses counted yet.',
        'These passives are always on, digitalized or not, so they count in both views. What the demon itself adds only counts in the Digitalized view.')));
    const race = (name) => name.replace(/^Digitalize: /, '');
    passiveList(page, rows, S.dg, { heading: 'Unlocked Digitalization passives', search: 'Search a race, passive or effect', rankTip: 'The Digitalization Level that teaches this passive',
      empty: 'No Digitalization Levels read yet. Take a snapshot.', title: race,
      tag: (name) => (DG && DG.race && race(name).toLowerCase() === String(DG.race).toLowerCase() ? h('span', { class: 'tag tag-demon', 'data-tip': `${DG.demon} is of this race`, text: DG.demon }) : null) });
    return page;
  }

  // ---------- History ----------
  function niceScale(min, max, count) {
    if (min === max) { const pad = Math.abs(min) * 0.1 || 1; min -= pad; max += pad; }
    const raw = (max - min) / count, mag = Math.pow(10, Math.floor(Math.log10(raw))), n = raw / mag;
    const step = (n < 1.5 ? 1 : n < 3 ? 2 : n < 7 ? 5 : 10) * mag;
    const lo = Math.floor(min / step) * step, hi = Math.ceil(max / step) * step, ticks = [];
    for (let v = lo; v <= hi + step / 2; v += step) ticks.push(Number(v.toFixed(8)));
    return { lo, hi, ticks };
  }
  function drawHistory(el, pts, label, unit) {
    el.replaceChildren();
    if (!pts.length) { el.append(h('p', { class: 'note', text: `No snapshot has a value for ${label} yet.` })); return; }
    const W = Math.max(320, el.clientWidth), H = 250, m = { t: 16, r: 64, b: 30, l: 52 };
    const t0 = pts[0].t, t1 = pts[pts.length - 1].t;
    const x = (t) => (t1 === t0 ? (m.l + W - m.r) / 2 : m.l + ((t - t0) / (t1 - t0)) * (W - m.l - m.r));
    const vs = pts.map((p) => p.v), sc = niceScale(Math.min(...vs), Math.max(...vs), 4);
    const y = (v) => H - m.b - ((v - sc.lo) / (sc.hi - sc.lo)) * (H - m.t - m.b);
    const svg = s('svg', { viewBox: `0 0 ${W} ${H}`, height: H, role: 'img', 'aria-label': `${label} across ${pts.length} snapshots` });
    sc.ticks.forEach((v) => svg.append(
      s('line', { class: v === sc.lo ? 'axisline' : 'gridline', x1: m.l, x2: W - m.r, y1: y(v), y2: y(v) }),
      Object.assign(s('text', { x: m.l - 8, y: y(v), dy: '0.32em', 'text-anchor': 'end' }), { textContent: fmt(v, unit) })));
    // x labels: newest always shown; older ones only where they fit
    const oneDay = pts.every((p) => sameDay(p.at, pts[0].at));
    const room = oneDay ? 64 : 104, kept = [pts.length - 1];
    if (pts.length > 1 && x(t1) - x(t0) >= room) kept.push(0);
    for (let i = pts.length - 2; i > 0; i--) if (kept.every((j) => Math.abs(x(pts[j].t) - x(pts[i].t)) >= room)) kept.push(i);
    kept.forEach((i) => {
      const anchor = pts.length === 1 ? 'middle' : i === pts.length - 1 ? 'end' : i === 0 ? 'start' : 'middle';
      svg.append(Object.assign(s('text', { x: x(pts[i].t), y: H - m.b + 18, 'text-anchor': anchor }), { textContent: oneDay ? clock(pts[i].at) : when(pts[i].at) }));
    });
    if (pts.length > 1) svg.append(s('path', { class: 'line', d: pts.map((p, i) => `${i ? 'L' : 'M'}${x(p.t).toFixed(1)},${y(p.v).toFixed(1)}`).join('') }));
    const cross = s('line', { class: 'cross', y1: m.t, y2: H - m.b, visibility: 'hidden' });
    svg.append(cross);
    const dots = pts.map((p) => s('circle', { class: 'dot', cx: x(p.t), cy: y(p.v), r: 5 }));
    svg.append(...dots);
    const last = pts[pts.length - 1];
    svg.append(Object.assign(s('text', { class: 'endlabel', x: x(last.t) + 10, y: y(last.v), dy: '0.32em' }), { textContent: fmt(last.v, unit) }));
    const hit = s('rect', { x: m.l - 10, y: m.t, width: W - m.l - m.r + 20, height: H - m.t - m.b, fill: 'transparent' });
    hit.addEventListener('pointermove', (ev) => {
      const box = svg.getBoundingClientRect(), mx = ((ev.clientX - box.left) / box.width) * W;
      let best = 0; pts.forEach((p, i) => { if (Math.abs(x(p.t) - mx) < Math.abs(x(pts[best].t) - mx)) best = i; });
      const p = pts[best];
      cross.setAttribute('x1', x(p.t)); cross.setAttribute('x2', x(p.t)); cross.setAttribute('visibility', 'visible');
      dots.forEach((d, i) => d.setAttribute('r', i === best ? 6.5 : 5));
      showTip(`${when(p.at)} · ${label} ${fmt(p.v, unit)}`, ev.clientX, ev.clientY);
    });
    hit.addEventListener('pointerleave', () => { cross.setAttribute('visibility', 'hidden'); dots.forEach((d) => d.setAttribute('r', 5)); hideTip(); });
    svg.append(hit);
    el.append(svg);
  }

  function pageHistory() {
    const { snaps } = S.v;
    const page = h('div', { class: 'page' });
    const options = [
      ['Stats', [...STATS.map(([k, label]) => [k, label, '', false]), ['max_hp', 'Max HP', '', false], ['max_mp', 'Max MP', '', false]]],
      ['Gear bonuses', COMBAT.map(([k, label, unit]) => [k, label, unit, true])],
    ];
    const flat = options.flatMap(([, list]) => list);
    let cur = flat.find(([k]) => k === S.hist.key) || flat.find(([k]) => k === 'lngr');
    const field = cur[3] && digi() ? cur[0] + '_digi' : cur[0];
    const label = cur[1] + (cur[3] ? (digi() ? ' (gear, digitalized)' : ' (gear)') : '');
    const sel = h('select', { class: 'select', id: 'hist-stat', onchange: () => { S.hist.key = sel.value; store.set('hist', sel.value); render(); } },
      options.map(([group, list]) => h('optgroup', { label: group }, list.map(([k, text]) => h('option', { value: k, selected: k === cur[0], text })))));
    page.append(pageHead('One point per snapshot, so gear swaps show up as steps.',
      host.native ? h('button', { type: 'button', class: 'btn', onclick: () => host.call('reveal', { what: 'csv' }) }, 'Show spreadsheet') : null));

    const pts = snaps.map((r) => ({ t: date(r.taken_at).getTime(), v: num(r[field]), at: r.taken_at })).filter((p) => p.v !== null && !Number.isNaN(p.t));
    const chart = h('div', { class: 'chart' });
    page.append(sect(label, null, h('label', { class: 'card-sub', for: 'hist-stat', style: 'display:inline-flex;align-items:center;gap:8px' }, 'Stat', sel),
      h('section', { class: 'card' }, chart)));
    requestAnimationFrame(() => drawHistory(chart, pts, cur[1], cur[2]));

    const desc = snaps.slice().reverse();
    page.append(h('div', { class: 'tbl-wrap' }, h('div', { class: 'tbl-scroll' }, h('table', { class: 'tbl' },
      h('thead', null, h('tr', null, h('th', { scope: 'col', text: 'Snapshot' }), h('th', { scope: 'col', text: 'Stats read' }), h('th', { scope: 'col', class: 'num', text: 'Level' }),
        h('th', { scope: 'col', class: 'num', text: 'Max HP' }), h('th', { scope: 'col', class: 'num', text: cur[1] }), h('th', { scope: 'col', class: 'num', text: 'Change' }))),
      h('tbody', null, desc.map((r, i) => {
        const prev = desc[i + 1], a = prev ? num(prev[field]) : null, b = num(r[field]);
        const d = a !== null && b !== null ? round2(b - a) : null;
        const good = d !== null && (lowIsGood(cur[1]) ? d < 0 : d > 0);
        return h('tr', null,
          h('td', null, when(r.taken_at), i === 0 ? h('span', { class: 'tag', style: 'margin-left:8px', text: 'Latest' }) : null),
          h('td', { class: 'dim', text: isTrue(r.stats_live) ? 'Live' : `Saved, from ${sameDay(r.stats_from, r.taken_at) ? clock(r.stats_from) : when(r.stats_from)}` }),
          h('td', { class: 'num', text: fmt(r.level) }), h('td', { class: 'num', text: fmt(r.max_hp) }),
          h('td', { class: 'num emph', text: fmt(b, cur[2]) }),
          h('td', { class: 'num' }, d ? h('span', { class: 'delta ' + (good ? 'good' : 'bad'), text: (d > 0 ? '▲ ' : '▼ ') + fmt(d, cur[2], true) }) : h('span', { class: 'zero', text: d === 0 ? 'same' : '—' })));
      }))))));
    page.append(h('p', { class: 'note', text: `${snaps.length} snapshot${snaps.length === 1 ? '' : 's'}. Gear bonus columns started with the second snapshot, so the first one has no value for them.` }));
    return page;
  }

  // ---------- render ----------
  // ---------- Tools: DCM Locator ----------
  // dcm-sources.json is made by tools\dcmexport from the game data; dcm-notes.json is written by hand (what players
  // found on NewMoon itself). Both sit beside this file.
  const DCM_KINDS = [['all', 'All'], ['shop', 'Shop'], ['drop', 'Enemy drop'], ['chest', 'Chest or loot'], ['event', 'Event or NPC'], ['present', 'Present only'], ['none', 'No known source']];
  const pct = (v) => `${nf.format(n0(v))}%`;
  function loadDcm() {
    if (S.dcm.state !== 'idle') return;
    S.dcm.state = 'loading';
    const get = (p) => fetch(p, { cache: 'no-store' }).then((r) => (r.ok ? r.json() : null)).catch(() => null);
    Promise.all([get('dcm-sources.json'), get('dcm-notes.json')]).then(([d, n]) => {
      S.dcm.data = d; S.dcm.notes = (n && n.notes) || {};
      S.dcm.state = d ? 'ready' : 'failed';
      if (d) d.items.forEach((it) => { it.lines = dcmLines(it); it.kinds = new Set(it.lines.map((l) => l.group)); it.hay = [it.demon, ...it.lines.map((l) => [l.text, l.sub || '', ...(l.tags || [])].join(' '))].join(' ').toLowerCase(); });
      if (S.page === 'dcm') render();
    });
  }
  /** One DCM's sources as display lines, best first: { group, kind, text, sub, value, tags }. */
  function dcmLines(it) {
    const lines = [], by = (k) => it.sources.filter((s) => s.kind === k);
    const zones = (list) => { const z = [...new Set(list.map((s) => s.zone).filter(Boolean))]; return z.length > 6 ? z.slice(0, 6).join(', ') + ` and ${z.length - 6} more` : z.join(', '); };
    const grouped = (list, key) => { const m = new Map(); list.forEach((s) => { const k = key(s); if (!m.has(k)) m.set(k, []); m.get(k).push(s); }); return [...m.values()]; };
    (S.dcm.notes[it.id] || []).forEach((t) => lines.push({ group: 'note', kind: 'NewMoon note', text: String(t), rank: 0 }));
    // A shop's moon mask: bit n = moon phase n (0 new, 8 full), the phases it is sold in.
    const moon = (mask) => 'Moon: ' + [...Array(16).keys()].filter((i) => mask & (1 << i)).map((i) => (i === 0 ? 'new' : i === 8 ? 'full' : i < 8 ? `${i}/8 waxing` : `${16 - i}/8 waning`)).join(', ');
    // "201010_halloween" (the server's folder of a seasonal event) -> "Halloween 2010 event"
    const season = (v) => { const m = /^(\d{4})\d{2}_(.+)$/.exec(v); return m ? `${m[2].charAt(0).toUpperCase() + m[2].slice(1).replace(/_/g, ' ')} ${m[1]} event` : v; };
    const tagsOf = (s) => (s.season ? [season(s.season)] : []);
    const NOPLACE = 'The data does not say where';
    grouped(by('shop'), (s) => s.shopId + '|' + s.price).forEach((g) => lines.push({ group: 'shop', kind: 'Shop', text: g[0].shop + (g[0].zone ? ' in ' + g[0].zone : ''), sub: [g[0].npc ? 'talk to ' + g[0].npc : '', g[0].tab ? 'tab: ' + g[0].tab : ''].filter(Boolean).join(' · '), value: `${fmt(g[0].price)} ${g[0].currency}`,
      tags: g[0].moon ? [moon(g[0].moon)] : [], rank: 1 }));
    grouped(by('drop'), (s) => s.enemy + '|' + s.rate + '|' + (s.season || '')).sort((a, b) => b[0].rate - a[0].rate).forEach((g) => lines.push({ group: 'drop', kind: 'Enemy drop', text: g[0].enemy, sub: zones(g) || NOPLACE, value: pct(g[0].rate),
      tags: tagsOf(g[0]), rank: 2 }));
    grouped(by('loot').concat(by('chest')), (s) => (s.ref || s.dropSetId) + '|' + s.rate).sort((a, b) => b[0].rate - a[0].rate).forEach((g) => lines.push({ group: 'chest', kind: 'Chest',
      text: zones(g) || g[0].desc || NOPLACE, value: pct(g[0].rate), tags: tagsOf(g[0]), rank: 3 }));
    grouped(by('plasma'), (s) => s.zone + '|' + s.rate).forEach((g) => lines.push({ group: 'chest', kind: 'Plasma', text: g[0].zone || NOPLACE, value: pct(g[0].rate), tags: tagsOf(g[0]), rank: 3 }));
    grouped(by('event'), (s) => s.zone + '|' + (s.season || '')).forEach((g) => { const who = [...new Set(g.map((s) => s.npc).filter(Boolean))];
      lines.push({ group: 'event', kind: 'Event or NPC', text: g[0].zone || NOPLACE, sub: who.length ? 'Talk to ' + who.join(', ') : '', tags: tagsOf(g[0]), rank: 4 }); });
    by('box').forEach((s) => lines.push({ group: 'chest', kind: 'Random box', text: s.desc || `Box ${s.dropSetId}`, value: pct(s.rate), rank: 5 }));
    by('destiny').concat(by('demonquest'), by('dropset')).forEach((s) => lines.push({ group: 'drop', kind: s.kind === 'destiny' ? 'Destiny box' : s.kind === 'demonquest' ? 'Demon quest' : 'Drop table',
      text: s.desc || `Drop set ${s.dropSetId}`, sub: s.kind === 'dropset' ? 'The data does not say which enemy uses this table' : '', value: pct(s.rate), rank: 6 }));
    by('present').forEach((s) => lines.push({ group: 'present', kind: 'Present', text: `From ${s.demon} as your partner`, sub: `${s.tier} present`, rank: 7 }));
    const shown = new Set();   // two chests of one place at one rate read the same: say it once
    return lines.sort((a, b) => a.rank - b.rank).filter((l) => { const k = [l.kind, l.text, l.sub, l.value, (l.tags || []).join()].join('|'); return !shown.has(k) && shown.add(k); });
  }
  /** The demon's own icon (made by tools\\dcmexport from the client); an empty box when there is none. */
  function dcmIcon(it) {
    const box = h('span', { class: 'dcm-icon', 'aria-hidden': 'true' });
    // No picture in the client: its own skull icon. Satan has none either, and (the user's joke) wears the player's face.
    const joke = !it.icon && it.demon === 'Satan' && S.meta.picture;
    const src = it.icon || joke || (S.dcm.data && S.dcm.data.noIcon);
    if (src) box.append(h('img', { src, alt: '', width: 28, height: 28, loading: 'lazy', onerror: (ev) => ev.target.remove() }));
    return box;
  }
  function pageDcm() {
    const D = S.dcm, page = h('div', { class: 'page page-dcm' });
    loadDcm();
    page.append(pageHead('Every DCM (Demon Compendium Memory) and where the game data says it comes from. Click a row for all its sources.'));
    if (D.state !== 'ready') {
      page.append(h('p', { class: 'note', text: D.state === 'failed' ? 'The DCM list could not be loaded (dcm-sources.json is missing beside the page).' : 'Loading the DCM list…' }));
      return page;
    }
    page.append(h('div', { class: 'notice' }, icon('info'), h('span', null, h('b', { text: 'Read from game data, not checked on NewMoon. ' }),
      'The DCM list, zone names and shop stock are the NewMoon client\'s. Drops, chests, presents and events come from the public server data, so NewMoon may have changed them. Lines marked “NewMoon note” are confirmed by players.')));
    const items = D.data.items;
    const inKind = (it, k) => k === 'all' ? true : k === 'none' ? !it.lines.length : k === 'present' ? it.lines.length > 0 && it.lines.every((l) => l.group === 'present') : it.kinds.has(k);
    const chips = h('div', { class: 'chips', role: 'group', 'aria-label': 'Filter by source' });
    const body = h('tbody');
    const drawChips = () => chips.replaceChildren(...DCM_KINDS.map(([id, label]) =>
      h('button', { type: 'button', class: 'chip-btn', 'aria-pressed': String(D.kind === id), onclick: () => { D.kind = id; drawChips(); draw(); } },
        label, h('span', { class: 'count', text: items.filter((it) => inKind(it, id)).length }))));
    const lineEl = (l) => h('li', null, h('span', { class: 'kind', text: l.kind }),
      h('span', null, l.text, (l.tags || []).map((t) => h('span', { class: 'tag', style: 'margin-left:8px', text: t })), l.sub ? h('small', { text: l.sub }) : null),
      h('span', { class: 'pv', text: l.value || '' }));
    const draw = () => {
      body.replaceChildren();
      const q = D.q.trim().toLowerCase();
      const rows = items.filter((it) => inKind(it, D.kind) && (!q || it.hay.includes(q))).sort((x, y) => !x.lines.length - !y.lines.length);
      rows.forEach((it) => {
        const open = D.open.has(it.id), best = it.lines[0];
        const toggle = () => { if (D.open.has(it.id)) D.open.delete(it.id); else D.open.add(it.id); draw(); };
        body.append(h('tr', { class: 'row-click' + (open ? ' open' : ''), onclick: (ev) => { if (!ev.target.closest('button')) toggle(); } },
          h('td', null, h('button', { type: 'button', class: 'row-btn', 'aria-expanded': String(open), onclick: toggle },
            h('span', { class: 'chev' }, icon('chevron')), dcmIcon(it), h('span', null, it.demon, h('span', { class: 'src-count', text: it.lines.length === 1 ? '1 source' : `${it.lines.length} sources` })))),
          best ? h('td', null, h('div', { class: 'dcm-best' }, h('span', { class: 'dcm-kind', text: best.kind }),
            h('span', null, best.text, best.sub ? h('span', { class: 'src-count', text: best.sub }) : null)))
            : h('td', { class: 'zero', text: 'No source in the game data' }),
          h('td', { class: 'num' + (best && best.value ? '' : ' zero'), text: best && best.value ? best.value : '—' })));
        if (open) body.append(h('tr', { class: 'detail-row' }, h('td', { colspan: 3 },
          it.lines.length ? h('ul', { class: 'parts dcm-parts' }, it.lines.map(lineEl))
            : h('p', { class: 'note', text: 'Nothing in the data gives this DCM. It is most likely a NewMoon addition whose source only the live server knows.' }),
          h('p', { class: 'note', style: 'margin-top:8px', text: `Item ${it.id} · ${it.name}` }))));
      });
      if (!rows.length) body.append(h('tr', null, h('td', { colspan: 3, class: 'empty-row', text: q ? `Nothing matches “${D.q.trim()}”.` : 'No DCM in this group.' })));
    };
    page.append(h('div', { class: 'toolbar' }, searchBox(D.q, 'Search a demon, enemy, zone or shop', (v) => { D.q = v; draw(); }), chips));
    page.append(h('div', { class: 'tbl-wrap' }, h('table', { class: 'tbl' },
      h('thead', null, h('tr', null, h('th', { scope: 'col', text: 'DCM' }), h('th', { scope: 'col', text: 'Best known source' }), h('th', { scope: 'col', class: 'num', text: 'Rate or price' }))), body)));
    page.append(h('p', { class: 'note', text: `${items.length} DCMs, data made on ${D.data.generatedAt}. Rates are the chance per kill or per chest. A present is what the demon may give you while it is your partner.` }));
    drawChips(); draw();
    return page;
  }

  // ---------- Tools: Quest Guide ----------
  // quest-guide.json is made by tools\questexport: the game's quest tables for the text and needs, the server's event
  // files for the steps (who to talk to, where, what to pick, what you get). It sits beside this file.
  const QUEST_TYPES = [['available', 'Available'], ['Normal', 'Normal'], ['Repeatable', 'Repeatable'], ['Event', 'Event'], ['I-Time', 'I-Time'], ['Episode', 'Episode'], ['Inactive', 'Inactive']];
  function loadQuests() {
    if (S.quest.state !== 'idle') return;
    S.quest.state = 'loading';
    fetch('quest-guide.json', { cache: 'no-store' }).then((r) => (r.ok ? r.json() : null)).catch(() => null).then((d) => {
      S.quest.data = d; S.quest.state = d ? 'ready' : 'failed';
      if (d) d.quests.forEach((q) => {
        const words = [q.title, q.client, q.zone];
        q.steps.forEach((s) => { words.push(s.objective || '', ...(s.goals || [])); (s.ways || []).forEach((w) => words.push(w.do, w.who || '', w.where, ...(w.get || []), ...(w.lose || []), ...(w.needs || []))); });
        q.hay = words.join(' ').toLowerCase();
      });
      if (S.page === 'quests') render();
    });
  }
  /**
   * The ways of doing a step, as groups: ways that ask the same and give the same differ only in who you talk to
   * (three NPCs who all take the report), so they become one instruction naming all of them.
   */
  function questWays(ways, q) {
    const groups = new Map();
    ways.forEach((w0) => {
      const w = Object.assign({}, w0, { needs: (w0.needs || []).filter((n) => !q.needs.includes(n)) });
      const key = JSON.stringify([w.kind === 'npc' && w.who ? 'talk' : w.do + w.where + (w.grid || ''), w.pick, w.needs, w.fights, w.get, w.lose, w.effects]);
      if (!groups.has(key)) groups.set(key, []);
      groups.get(key).push(w);
    });
    return [...groups.values()];
  }
  /** One instruction: what to do and where as a sentence, then what to choose, what you need, and what changes hands. */
  function questWay(ws, alt) {
    const w = ws[0];
    const at = (x) => (x.where ? (x.kind === 'zone' ? ` to ${x.where}` : ` in ${x.where}`) : '') + (x.grid ? ` at (${x.grid})` : '');
    let sentence;
    if (w.who) {
      const seen = new Set(), people = ws.filter((x) => { const k = x.who + at(x); return !seen.has(k) && seen.add(k); });
      sentence = ['Talk to ', people.map((x, i) => [i ? (i === people.length - 1 ? ' or ' : ', ') : '', h('b', { text: x.who }), at(x)]), '.'];
    } else sentence = [h('b', { text: w.do }), w.kind === 'event' ? '' : at(w), '.'];
    const row = (label, list, cls) => (list && list.length ? h('div', { class: 'q-line' + (cls ? ' ' + cls : '') }, h('span', { class: 'q-key', text: label }), h('span', null, list.join(', '))) : null);
    return h('div', { class: 'q-way' },
      h('div', { class: 'q-do' }, alt ? h('span', { class: 'q-or', text: 'or' }) : null, h('span', null, sentence)),
      w.dungeon ? h('div', { class: 'q-hint', text: w.dungeon }) : null,
      w.hint ? h('div', { class: 'q-hint', text: w.hint }) : null,
      w.asks ? h('div', { class: 'q-line' }, h('span', { class: 'q-key', text: 'It asks' }), h('span', { class: 'q-asks', text: w.asks })) : null,
      w.pick && w.pick.length ? h('div', { class: 'q-line' }, h('span', { class: 'q-key', text: 'Choose' }), h('span', null, w.pick.map((p, i) => [i ? h('span', { class: 'q-arrow', text: ' then ' }) : null, h('span', { class: 'q-pick', text: p })]))) : null,
      row('You must', w.needs), row('You fight', w.fights), row('You receive', w.get, 'q-get'), row('You hand over', w.lose), row('Afterwards', w.effects));
  }
  function questDetail(q) {
    const box = h('div', { class: 'q-detail' });
    if (q.needs.length) box.append(h('div', { class: 'q-needs' }, h('span', { class: 'q-key', text: 'Before you can start' }), h('span', null, q.needs.join(' · '))));
    if (!q.walkthrough) box.append(h('p', { class: 'note', text: 'The server\'s event files do not cover this quest, so only the game\'s own objective text is shown.' }));
    const ol = h('ol', { class: 'q-steps' });
    q.steps.forEach((s) => ol.append(h('li', null,
      /^Step /.test(s.label) ? null : h('div', { class: 'q-step-title', text: s.label }),   // the number is already in the circle
      s.objective ? h('p', { class: 'q-objective', text: s.objective }) : null,
      s.goals && s.goals.length ? h('ul', { class: 'q-goals' }, s.goals.map((g) => h('li', { text: g }))) : null,
      questWays(s.ways || [], q).map((g, i) => questWay(g, i > 0)))));
    if (q.steps.length) box.append(ol);
    else box.append(h('p', { class: 'note', text: 'The data has no steps or objective text for this quest.' }));
    if (q.unlocks && q.unlocks.length) box.append(h('div', { class: 'q-needs' }, h('span', { class: 'q-key', text: 'Opens' }), h('span', null, q.unlocks.join(' · '))));
    return box;
  }
  function pageQuests() {
    const D = S.quest, page = h('div', { class: 'page page-dcm' });
    loadQuests();
    page.append(pageHead('Every quest with what to do, step by step. Click a quest to open it.'));
    if (D.state !== 'ready') {
      page.append(h('p', { class: 'note', text: D.state === 'failed' ? 'The quest list could not be loaded (quest-guide.json is missing beside the page).' : 'Loading the quests…' }));
      return page;
    }
    page.append(h('div', { class: 'notice' }, icon('info'), h('span', null, h('b', { text: 'Read from game data, not checked on NewMoon. ' }),
      'Quest names, objectives and kill or collect counts are the game\'s own. The steps (who to talk to, what to pick, rewards) come from the public server\'s event files, so NewMoon may have changed them. Valuables are shown with the name the game data has, which is Japanese.')));
    const quests = D.data.quests;
    const inType = (q, t) => (t === 'available' ? q.type !== 'Inactive' : q.type === t);
    const chips = h('div', { class: 'chips', role: 'group', 'aria-label': 'Filter by kind of quest' });
    const body = h('tbody');
    const drawChips = () => chips.replaceChildren(...QUEST_TYPES.map(([id, label]) =>
      h('button', { type: 'button', class: 'chip-btn', 'aria-pressed': String(D.type === id), onclick: () => { D.type = id; drawChips(); draw(); } },
        label, h('span', { class: 'count', text: quests.filter((q) => inType(q, id)).length }))));
    const draw = () => {
      body.replaceChildren();
      const q = D.q.trim().toLowerCase();
      const rows = quests.filter((x) => inType(x, D.type) && (!q || x.hay.includes(q)));
      rows.forEach((x) => {
        const open = D.open.has(x.id);
        const toggle = () => { if (D.open.has(x.id)) D.open.delete(x.id); else D.open.add(x.id); draw(); };
        const where = [x.zone, x.grid ? `(${x.grid})` : ''].filter(Boolean).join(' ');
        body.append(h('tr', { class: 'row-click' + (open ? ' open' : ''), onclick: (ev) => { if (!ev.target.closest('button')) toggle(); } },
          h('td', null, h('button', { type: 'button', class: 'row-btn', 'aria-expanded': String(open), onclick: toggle },
            h('span', { class: 'chev' }, icon('chevron')), h('span', null, x.title, x.walkthrough ? null : h('span', { class: 'src-count', text: 'objective only' })))),
          h('td', null, x.client || h('span', { class: 'zero', text: '—' }), where ? h('span', { class: 'src-count', text: where }) : null),
          h('td', null, h('span', { class: 'tag', text: x.type }))));
        if (open) body.append(h('tr', { class: 'detail-row' }, h('td', { colspan: 3 }, questDetail(x))));
      });
      if (!rows.length) body.append(h('tr', null, h('td', { colspan: 3, class: 'empty-row', text: q ? `Nothing matches “${D.q.trim()}”.` : 'No quest in this group.' })));
    };
    page.append(h('div', { class: 'toolbar' }, searchBox(D.q, 'Search a quest, person, place or item', (v) => { D.q = v; draw(); }), chips));
    page.append(h('div', { class: 'tbl-wrap' }, h('table', { class: 'tbl' },
      h('thead', null, h('tr', null, h('th', { scope: 'col', text: 'Quest' }), h('th', { scope: 'col', text: 'Client and place' }), h('th', { scope: 'col', text: 'Kind' }))), body)));
    page.append(h('p', { class: 'note', text: `${quests.length} quests, ${quests.filter((x) => x.walkthrough).length} with steps from the event files. Data made on ${D.data.generatedAt}. Numbers in brackets are the map grid position.` }));
    drawChips(); draw();
    return page;
  }
  // ---------- Tools: Gear Database ----------
  // gear-db.json is made by "newmoon-reader geardb": what each piece gives is read from the NewMoon client's current
  // tables; where it comes from is what the community spreadsheets say. It sits beside this file.
  const GDB_SLOTS = ['Weapon', 'Head', 'Face', 'Neck', 'Top', 'Arms', 'Bottom', 'Feet', 'Ring', 'Earring', 'Extra', 'Back', 'Talisman', 'COMP', 'Bullets'];
  function loadGearDb() {
    if (S.gdb.state !== 'idle') return;
    S.gdb.state = 'loading';
    fetch('gear-db.json', { cache: 'no-store' }).then((r) => (r.ok ? r.json() : null)).catch(() => null).then((d) => {
      S.gdb.data = d; S.gdb.state = d ? 'ready' : 'failed';
      if (d) {
        d.items.forEach((it, i) => {
          it.key = it.id || 'sheet' + i;
          // the game's own effect text packs several lines into one, with a written-out line break between them
          const lines = (list) => (list || []).flatMap((x) => String(x).split(/\s*\\n\s*/)).map((x) => x.trim()).filter(Boolean);
          it.stats = lines(it.stats); it.effects = lines(it.effects); it.appearance = lines(it.appearance);
          (it.sets || []).forEach((s) => { s.effects = lines(s.effects); });
          it.all = [...(it.stats || []), ...(it.effects || []), ...(it.appearance || []), ...(it.sets || []).flatMap((s) => s.effects)];
          it.get = it.get || [];
          const getWords = (list) => (list || []).flatMap((g) => [g.k, g.t, g.s || '']);
          it.known = it.get.length > 0 || it.sources.length > 0;
          it.hay = [it.name, it.slot, ...it.all, ...(it.sets || []).flatMap((s) => s.with), ...getWords(it.get),
            ...it.sources.flatMap((s) => [s.tab, s.how, s.ticket || '', ...getWords(s.ticketGet)])].join(' ').toLowerCase();
        });
        d.items.sort((a, b) => a.name.localeCompare(b.name));
      }
      if (S.page === 'geardb') render();
    });
  }
  function gearDbDetail(it) {
    const box = h('div', { class: 'q-detail' });
    const block = (label, list) => (list && list.length ? h('div', { class: 'gdb-block' }, h('span', { class: 'q-key', text: label }), h('ul', { class: 'gdb-list' }, list.map((x) => h('li', { text: x })))) : null);
    const facts = [it.slot, it.gender && it.gender !== 'Any' ? it.gender + ' only' : '', it.alignment ? it.alignment + ' only' : '', it.weapon || '', it.affinity ? it.affinity + ' affinity' : '',
      it.mods ? `${it.mods} mod slot${it.mods === 1 ? '' : 's'}` : ''].filter(Boolean);
    box.append(h('div', { class: 'detail-meta' }, facts.map((x) => h('span', { class: 'tag', text: x }))));
    if (it.sheetOnly) box.append(h('p', { class: 'note', text: 'This row is from the spreadsheet only: its name matched no item in the game data, so the effects below are the sheet\'s own text.' }));
    [block('Stats', it.stats), block('Effects', it.effects), block('Appearance bonus', it.appearance)].filter(Boolean).forEach((b) => box.append(b));
    // s.any: the bonus needs one of the listed pieces; otherwise it needs all of them
    (it.sets || []).forEach((s) => { const many = s.with.length > 1;
      const b = block(!many ? 'Set with ' + s.with[0] : s.any ? 'Set with any one of' : 'Set with all of', s.effects); if (!b) return;
      if (many) b.lastChild.prepend(h('li', { class: 'gdb-with', text: s.with.join(', ') }));
      box.append(b); });
    if (!it.all.length && !it.sheetOnly) box.append(h('p', { class: 'note', text: 'This piece gives no stats or effects in the game data.' }));
    // Where to get it, in the game: what the game data says about the piece itself, then about the ticket it comes in.
    // g.d: how deep the line sits under the one before it ("Open X", then where X comes from)
    const getLine = (g, extra) => h('li', { style: (g.d || 0) + (extra || 0) ? `padding-left:${22 * ((g.d || 0) + (extra || 0))}px` : null },
      g.k ? h('span', { class: 'gdb-kind', text: g.k }) : null, g.k ? h('b', { text: g.t }) : h('span', { class: 'src-count', style: 'margin-left:0', text: g.t }),
      g.s ? h('span', { class: 'src-count', text: g.s }) : null, g.v ? h('span', { class: 'gdb-rate', text: g.v }) : null);
    const where = [];
    it.get.forEach((g) => where.push(getLine(g)));
    const tickets = new Map();
    it.sources.forEach((s) => { if (s.ticket && !tickets.has(s.ticket)) tickets.set(s.ticket, s.ticketGet || []); });
    tickets.forEach((lines, ticket) => {
      where.push(h('li', null, h('span', { class: 'gdb-kind', text: 'Comes in' }), h('b', { text: ticket }),
        lines.length ? null : h('span', { class: 'src-count', text: 'the game data does not say where this ticket comes from' })));
      lines.forEach((g) => where.push(getLine(g, 1)));
    });
    if (where.length) box.append(h('div', { class: 'gdb-block' }, h('span', { class: 'q-key', text: 'Where to get it' }), h('ul', { class: 'gdb-list gdb-get' }, where)));
    // What players wrote down about it (the place is the name of the sheet's page, usually the dungeon).
    const notes = [...new Set(it.sources.map((s) => [s.tab, s.how].filter(Boolean).join(' · ')))];
    if (notes.length) box.append(h('div', { class: 'gdb-block' }, h('span', { class: 'q-key', text: 'Community notes' }), h('ul', { class: 'gdb-list' }, notes.map((n) => h('li', { text: n })))));
    if (!where.length && !notes.length) box.append(h('p', { class: 'note', text: 'Neither the game data nor the community notes say where this piece comes from.' }));
    if (it.id) box.append(h('p', { class: 'note', text: `Item ${it.id}` }));
    return box;
  }
  function pageGearDb() {
    const D = S.gdb, page = h('div', { class: 'page page-dcm' });
    loadGearDb();
    page.append(pageHead('Every piece of gear: what it gives and where it comes from. Click a row for the details.'));
    if (D.state !== 'ready') {
      page.append(h('p', { class: 'note', text: D.state === 'failed' ? 'The gear list could not be loaded (gear-db.json is missing beside the page).' : 'Loading the gear list…' }));
      return page;
    }
    page.append(h('div', { class: 'notice' }, icon('info'), h('span', null, h('b', { text: 'Two sources. ' }),
      'Stats, effects and sets are read from the NewMoon client\'s current item tables. "Where to get it" is traced through the public server data (drops, chests, shops, events), which NewMoon may have changed. "Community notes" are what the community wrote down for NewMoon, and can be imprecise.')));
    const items = D.data.items;
    const pass = (it, slot) => (!D.known || it.known) && (slot === 'all' || it.slot === slot);
    const chips = h('div', { class: 'chips', role: 'group', 'aria-label': 'Filter by slot' });
    const body = h('tbody'), more = h('div', { class: 'gdb-more' });
    const drawChips = () => chips.replaceChildren(...[['all', 'All']].concat(GDB_SLOTS.map((s) => [s, s])).map(([id, label]) =>
      h('button', { type: 'button', class: 'chip-btn', 'aria-pressed': String(D.slot === id), onclick: () => { D.slot = id; D.shown = 150; drawChips(); draw(); } },
        label, h('span', { class: 'count', text: items.filter((it) => pass(it, id)).length }))));
    const draw = () => {
      body.replaceChildren();
      const words = D.q.trim().toLowerCase().split(/\s+/).filter(Boolean);
      const rows = items.filter((it) => pass(it, D.slot) && words.every((w) => it.hay.includes(w)));
      rows.slice(0, D.shown).forEach((it) => {
        const open = D.open.has(it.key);
        const toggle = () => { if (D.open.has(it.key)) D.open.delete(it.key); else D.open.add(it.key); draw(); };
        const gives = it.all.slice(0, 3).join(' · ') + (it.all.length > 3 ? ` · and ${it.all.length - 3} more` : '');
        // the game's own answer first, then the ticket, then the place the community notes file it under
        const ticket = (it.sources.find((s) => s.ticket) || {}).ticket;
        const src = it.get.length ? (it.get[0].k === 'Open' ? `Open ${it.get[0].t}` : `${it.get[0].k}: ${it.get[0].t}`) : ticket ? `Open ${ticket}`
          : it.sources.length ? [...new Set(it.sources.map((s) => s.tab))].slice(0, 2).join(', ') : '';
        body.append(h('tr', { class: 'row-click' + (open ? ' open' : ''), onclick: (ev) => { if (!ev.target.closest('button')) toggle(); } },
          h('td', null, h('button', { type: 'button', class: 'row-btn', 'aria-expanded': String(open), onclick: toggle },
            h('span', { class: 'chev' }, icon('chevron')), h('span', null, it.name, it.sheetOnly ? h('span', { class: 'src-count', text: 'sheet only' }) : null))),
          h('td', null, it.slot || h('span', { class: 'zero', text: '—' })),
          h('td', { class: gives ? 'gdb-gives' : 'zero', text: gives || '—' }),
          h('td', { class: src ? '' : 'zero', text: src || 'Not known' })));
        if (open) body.append(h('tr', { class: 'detail-row' }, h('td', { colspan: 4 }, gearDbDetail(it))));
      });
      if (!rows.length) body.append(h('tr', null, h('td', { colspan: 4, class: 'empty-row', text: words.length ? `Nothing matches “${D.q.trim()}”.` : 'No gear in this group.' })));
      more.replaceChildren(...[h('span', { class: 'note', text: `Showing ${Math.min(D.shown, rows.length)} of ${rows.length}.` }),
        rows.length > D.shown ? h('button', { type: 'button', class: 'btn btn-sm', onclick: () => { D.shown += 300; draw(); } }, 'Show more') : null].filter(Boolean));
    };
    const known = h('label', { class: 'gdb-check' }, h('input', { type: 'checkbox', checked: D.known ? true : null, onchange: (ev) => { D.known = ev.target.checked; D.shown = 150; drawChips(); draw(); } }), 'Only gear with a known source');
    page.append(h('div', { class: 'toolbar' }, searchBox(D.q, 'Search a name, effect or place (e.g. limit break celu)', (v) => { D.q = v; D.shown = 150; draw(); }), known));
    page.append(chips);
    page.append(h('div', { class: 'tbl-wrap' }, h('table', { class: 'tbl' },
      h('thead', null, h('tr', null, h('th', { scope: 'col', text: 'Gear' }), h('th', { scope: 'col', text: 'Slot' }), h('th', { scope: 'col', text: 'Gives' }), h('th', { scope: 'col', text: 'Where to get it' }))), body)));
    page.append(more);
    page.append(h('p', { class: 'note', text: `${items.filter((x) => x.id).length} pieces in the game data; ${items.filter((x) => x.id && x.known).length} with a known way to get them. Data made on ${D.data.generatedAt}. Community notes are from SnowSheet's gear reference.` }));
    drawChips(); draw();
    return page;
  }
  const RENDER = { overview: pageOverview, bonuses: pageBonuses, gear: pageGear, expertise: pageExpertise, digitalize: pageDigitalize, history: pageHistory, dcm: pageDcm, quests: pageQuests, geardb: pageGearDb };
  function render() {
    renderChrome();
    const main = $('#main'), top = main.scrollTop;
    hideTip();
    main.replaceChildren(isTool(S.page) || (S.v && S.v.latest) ? RENDER[S.page]() : firstRun());
    main.scrollTop = top;
  }
  let resizeTimer = null;
  window.addEventListener('resize', () => { clearTimeout(resizeTimer); resizeTimer = setTimeout(() => { if (S.data && S.page === 'history') render(); }, 150); });

  // ---------- actions ----------
  async function reload() {
    const d = await host.call('load');
    S.data = { snapshots: d.snapshots || [], gear: d.gear || [], expertise: d.expertise || [], bonuses: d.bonuses || [], ranks: d.ranks || null, digitalize: d.digitalize || null, icons: d.icons || {}, unlocks: d.unlocks || [] };
    S.meta = d.meta || {};
    if (!S.data.digitalize) S.mode = 'normal';   // no demon saved: there is nothing to be digitalized with (the choice itself is kept)
    derive();
    render();
  }

  // ---------- characters: who is in game, whose data is shown ----------
  const chooser = $('#chooser');
  /** Look at the running game clients (read-only): who is logged in where. */
  async function scanClients() {
    if (S.scanning) return;
    S.scanning = true; renderChrome(); drawChooser();
    try { S.state = await host.call('clients'); S.lastScan = Date.now(); }
    catch (e) { /* the indicator keeps its last answer */ }
    finally { S.scanning = false; renderChrome(); drawChooser(); }
  }
  async function selectCharacter(name) {
    if (name === S.state.active && S.meta.character === name) return;
    S.state = await host.call('selectCharacter', { name });
    try { Object.assign(S.settings, await host.call('getSettings')); } catch (e) { /* keep */ }
    // what was open or picked belongs to the other character
    S.compare = null; S.over.open = null; S.over.allChanges = false; S.demons = null; S.bonus.open.clear(); S.bonus.q = ''; S.exp.closed.clear(); S.exp.q = ''; S.dg.closed.clear(); S.dg.q = ''; S.gear.sel = null; S.gear.sort = null; S.alignNote = '';
    await reload();
    $('#main').scrollTop = 0;
  }
  function drawChooser() {
    if (!chooser.open) return;
    const st = S.state, inGame = st.clients.filter((c) => c.character), idle = st.clients.filter((c) => !c.character);
    const saved = st.characters.filter((c) => !inGame.some((g) => g.character === c.name));
    const asked = chooser.dataset.why === 'startup';
    $('#chooser-title').textContent = asked ? `${inGame.length} characters are in game` : 'Characters';
    $('#chooser-sub').textContent = asked ? 'Which one do you want to look at?' : 'Each character keeps its own snapshots and spreadsheet. The name is read from the game.';
    const row = (name, line, live) => h('button', { type: 'button', class: 'char', 'aria-current': name === st.active ? 'true' : null,
      onclick: async () => { chooser.close(); try { await selectCharacter(name); } catch (e) { toast('err', 'Couldn’t switch character', String(e && e.message || e), 0); } } },
      (st.characters.find((k) => k.name === name) || {}).picture
        ? h('img', { class: 'char-pic' + (live ? ' on' : ''), src: st.characters.find((k) => k.name === name).picture, alt: '' })
        : h('span', { class: 'char-dot' + (live ? ' on' : '') }),
      h('span', { class: 'char-text' }, h('span', { class: 'char-name', text: name }), h('span', { class: 'char-line', text: line })),
      name === st.active ? h('span', { class: 'tag', text: 'Showing' }) : null);
    const list = $('#chooser-list');
    list.replaceChildren();
    if (inGame.length) list.append(h('div', { class: 'list-head', text: 'In game now' }),
      ...inGame.map((c) => row(c.character, [`Level ${c.level}`, c.folder, st.characters.some((k) => k.name === c.character && k.hasData) ? null : 'no snapshot yet'].filter(Boolean).join(' · '), true)));
    if (saved.length) list.append(h('div', { class: 'list-head', text: inGame.length ? 'Not in game' : 'Saved characters' }),
      ...saved.map((c) => row(c.name, c.hasData ? 'Saved snapshots' : 'No snapshot yet', false)));
    if (!inGame.length && !saved.length) list.append(h('p', { class: 'note', text: st.scanned ? 'Nobody is logged in. Start NewMoon, log in, then look again.' : 'Looking…' }));
    $('#chooser-status').textContent = S.scanning ? 'Looking at the game clients…'
      : idle.length ? `${idle.map((c) => c.folder || `client ${c.pid}`).join(', ')}: nobody logged in yet`
      : st.scanned && !st.clients.length ? 'The game is not running.' : '';
    $('#chooser-rescan').disabled = S.scanning;
  }
  function openChooser(why) {
    chooser.dataset.why = why || '';
    if (!chooser.open) chooser.showModal();
    drawChooser();
    if (!why && (host.native || Q.has('clients'))) scanClients();   // opened by hand: logins may have changed since the last look
  }
  // ---------- the digitalize demon: any demon from the COMP ----------
  // A dropdown under the demon's name on the Overview (a popover: a click outside or Esc closes it; the button's own
  // popovertarget opens and closes it). It is also the Normal / Digitalized switch: "No demon" is the normal view,
  // a demon is the digitalized view with it. The list is the demons seen before, kept by the host
  // (cache\comp-demons.json) and here (S.demons), so opening never reads the game; "Load demons from the game" at the
  // end does that (a scan of the game's memory, a second or two) and adds what it finds.
  // Picking closes the dropdown straight away; switching to another demon (another scan, the spreadsheet, the page
  // data) runs behind the demon line, which says so until it is done.
  const demonsMenu = $('#demons');
  let demonsFor = null, demonsLook = 0;   // the button it hangs from; which look at the game is the latest
  function placeDemons() {
    if (!demonsFor || !demonsFor.isConnected || !demonsMenu.matches(':popover-open')) return;
    const r = demonsFor.getBoundingClientRect(), w = demonsMenu.offsetWidth, hgt = demonsMenu.offsetHeight;
    demonsMenu.style.left = Math.max(8, Math.min(r.left, window.innerWidth - w - 8)) + 'px';
    demonsMenu.style.top = (r.bottom + 6 + hgt > window.innerHeight - 8 ? Math.max(8, r.top - 6 - hgt) : r.bottom + 6) + 'px';
  }
  const setMode = (mode) => { S.mode = mode; store.set('mode', mode); };
  /** The list as known now: "No demon", then every demon seen. `status`: a line under the load button. */
  function drawDemons(status, loading) {
    const list = $('#demons-list'), demons = S.demons || [];
    const had = list.contains(document.activeElement);
    const none = h('button', { type: 'button', class: 'char', 'aria-current': digi() ? null : 'true',
      onclick: () => { demonsMenu.hidePopover(); if (digi()) { setMode('normal'); render(); } } },
      h('span', { class: 'dm-icon dm-none dm-off', text: '–' }),
      h('span', { class: 'char-text' }, h('span', { class: 'char-name', text: 'No demon' }), h('span', { class: 'char-line', text: 'Not digitalized' })));
    // each demon as a card: what digitalizing with it would give (demonTile)
    const row = (d) => { const on = digi() && d.current; return demonTile(d, on,
      () => (on ? demonsMenu.hidePopover() : d.current ? (demonsMenu.hidePopover(), setMode('digi'), render())
        : d.loaded === false ? toast('warn', `${d.name} can’t be used yet`, 'The game only loads a demon’s stats and skills when you look at it. Open its status window in game (from the COMP), then press Load demons from the game.', 12000)
          : useDemon(d))); };
    list.replaceChildren(none, ...demons.map(row));
    const load = $('#demons-load');
    load.disabled = !!loading;
    load.textContent = loading ? 'Reading the game…' : 'Load demons from the game';
    $('#demons-status').textContent = status || (!S.demons ? '' : !demons.length ? 'No demons seen yet. Open the COMP in game, then load them.' : '');
    placeDemons();
    if (had) { const b = list.querySelector('[aria-current="true"]') || list.querySelector('button'); if (b) b.focus(); }
  }
  /** The demons seen before (at once), or with `scan` a fresh look at the game added to them. */
  async function loadDemons(scan) {
    const look = ++demonsLook;
    drawDemons(null, scan);
    try {
      const demons = await host.call('compDemons', scan ? {} : { cached: true });
      if (look !== demonsLook) return;
      S.demons = demons;
      // The game keeps every COMP demon in memory only while the COMP (or Demon List) is open; otherwise just the summoned one.
      const live = demons.filter((d) => d.inGame).length;
      const unread = demons.filter((d) => d.inGame && d.loaded === false).length;   // listed by the game without their details
      drawDemons(!scan ? null : !host.native ? 'Browser preview: stand-in demons.'
        : !live ? 'The game isn’t running, or has no demons loaded. The list is as before.'
        : live === 1 ? 'Only the summoned demon was loaded. Open the COMP in game, then load again for all of them.'
        : `${live} demons in your COMP.` + (unread ? ` ${unread} not read yet: open a demon’s status window in game, then load again.` : ''));
    } catch (e) { if (look === demonsLook) drawDemons(String(e && e.message || e)); }
  }
  demonsMenu.addEventListener('toggle', (ev) => {
    if (ev.newState !== 'open') { demonsLook++; return; }
    if (S.demons) drawDemons(); else loadDemons(false);   // never the game: only what is cached
    placeDemons();
    const b = $('#demons-list [aria-current="true"]') || $('#demons-list button');
    if (b && demonsFor && demonsFor.matches(':focus-visible')) b.focus();   // opened from the keyboard: go into the list
  });
  demonsMenu.addEventListener('keydown', (ev) => {
    if (ev.key !== 'ArrowDown' && ev.key !== 'ArrowUp') return;
    const rows = [...demonsMenu.querySelectorAll('.char, .dg-tile, #demons-load')], i = rows.indexOf(document.activeElement);
    const next = rows[ev.key === 'ArrowDown' ? Math.min(rows.length - 1, i + 1) : Math.max(0, i - 1)];
    if (next) { next.focus(); ev.preventDefault(); }
  });
  window.addEventListener('resize', placeDemons);
  $('#main').addEventListener('scroll', () => { if (demonsMenu.matches(':popover-open')) demonsMenu.hidePopover(); });   // its button moves away
  $('#demons-load').addEventListener('click', () => loadDemons(true));
  async function useDemon(d) {
    if (S.over.switching) return;
    S.over.switching = d.name;
    demonsMenu.hidePopover();
    render();   // the demon line says "Switching to …"
    try {
      const r = await host.call('useDemon', { key: d.key });
      if (S.demons) S.demons = S.demons.map((x) => ({ ...x, current: x.key === d.key }));
      S.over.switching = null;
      setMode('digi');
      await reload();
      toast('ok', `${r.name} is the digitalize demon`, r.rowUpdated ? 'The latest snapshot now adds it.' : 'It is added to the next snapshot.');
    } catch (e) {
      toast('err', `Couldn’t switch to ${d.name}`, String(e && e.message || e), 0);
    } finally { if (S.over.switching) { S.over.switching = null; render(); } }
  }

  /** After a look at the clients: follow the only character in game, or ask when there are several. */
  async function settle(atStart) {
    const inGame = S.state.clients.filter((c) => c.character);
    if (atStart && inGame.length >= 2) { openChooser('startup'); return; }
    if (inGame.length === 1 && inGame[0].character !== S.state.active && (atStart || !S.state.active)) {
      try {
        await selectCharacter(inGame[0].character);
        toast('info', `Showing ${inGame[0].character}`, `That is the character logged in${inGame[0].folder ? ' on ' + inGame[0].folder : ''}.`);
      } catch (e) { toast('err', 'Couldn’t switch character', String(e && e.message || e), 0); }
    }
  }
  $('#who-btn').addEventListener('click', () => openChooser());
  $('#chooser-close').addEventListener('click', () => chooser.close());
  $('#chooser-rescan').addEventListener('click', scanClients);
  chooser.addEventListener('click', (ev) => { if (ev.target === chooser) chooser.close(); });

  // ---------- character picture: a photo of the game window (or a file), cropped here ----------
  // The window takes the photo and does the cutting; the page only shows the source and says which rectangle.
  const picDlg = $('#picture'), picImg = $('#pic-img'), picCrop = $('#pic-crop'), picFrame = $('#pic-frame');
  const P = { src: null, crop: null, busy: '', note: '' };   // src {url,width,height}; crop in source pixels
  const RATIO = 4 / 3;                                        // every picture has this shape: 4 wide to 3 tall
  const picScale = () => (P.src && picImg.clientWidth ? picImg.clientWidth / P.src.width : 1);
  function placeCrop() {
    if (!P.src || !P.crop) return;
    const k = picScale();
    Object.assign(picCrop.style, { left: P.crop.x * k + 'px', top: P.crop.y * k + 'px', width: P.crop.w * k + 'px', height: P.crop.h * k + 'px' });
  }
  /** Keep the selection 4:3, at least 48 px wide (when the picture allows), and inside the picture. Only x, y and w matter. */
  function fitCrop(c) {
    const W = P.src.width, H = P.src.height;
    const w = Math.max(Math.min(48, W, H * RATIO), Math.min(c.w, W, H * RATIO)), hgt = w / RATIO;
    return { x: Math.max(0, Math.min(c.x, W - w)), y: Math.max(0, Math.min(c.y, H - hgt)), w, h: hgt };
  }
  function startCrop() {
    // The capture found the Information window: the top of its character view (the face) is the selection.
    // A small picture is an earlier cut-out of that view: its top again. Otherwise a first guess in the middle.
    if (P.src.view) { P.crop = fitCrop({ x: P.src.view.x, y: P.src.view.y, w: P.src.view.w }); return; }
    if (P.src.width <= 400 && P.src.height <= 700) { P.crop = fitCrop({ x: 0, y: 0, w: P.src.width }); return; }
    const w = P.src.width * 0.3;
    P.crop = fitCrop({ x: (P.src.width - w) / 2, y: (P.src.height - w / RATIO) / 2, w });
  }
  function drawPicture() {
    const who = S.state.active || '';
    $('#picture-title').textContent = `Picture of ${who}`;
    picFrame.hidden = !P.src; $('#pic-empty').hidden = !!P.src;
    if (P.src && picImg.getAttribute('src') !== P.src.url) picImg.src = P.src.url;
    placeCrop();
    $('#pic-help').textContent = P.busy || P.note || (P.src
      ? (P.src.view ? 'The Information window was found, and the top of its character view is selected. ' : '') + 'Drag the box over the face. Drag its corner or turn the mouse wheel to resize it. Every picture has the same shape.'
      : 'With the Information window open in game, a snapshot takes the picture by itself. You can also capture the game window or choose a file here.');
    const auto = $('#pic-auto');
    auto.checked = S.meta.autoPicture !== false; auto.disabled = !!P.busy;
    $('#pic-capture').disabled = $('#pic-file').disabled = !!P.busy;
    $('#pic-save').disabled = !P.src || !!P.busy;
    $('#pic-remove').hidden = !S.meta.picture && !P.src;
    $('#pic-remove').disabled = !!P.busy;
  }
  picImg.addEventListener('load', placeCrop);
  window.addEventListener('resize', placeCrop);
  async function openPicture() {
    if (!S.state.active) { openChooser(); return; }
    P.busy = ''; P.note = ''; P.src = null; P.crop = null;
    picDlg.showModal(); drawPicture();
    try { P.src = await host.call('pictureSource'); } catch (e) { P.src = null; }
    if (P.src) startCrop();
    drawPicture();
  }
  async function picGet(cmd, working) {
    P.busy = working; P.note = ''; drawPicture();
    try {
      const got = await host.call(cmd);
      if (got) { P.src = got; startCrop(); }
    } catch (e) {
      const m = String(e && e.message || e);
      P.note = /No running client has|ImagineClient is not running/i.test(m) ? `${S.state.active} isn’t in game right now, so there is no window to photograph. Log in, or choose a file.` : m;
    } finally { P.busy = ''; drawPicture(); }
  }
  $('#pic-capture').addEventListener('click', () => picGet('pictureCapture', 'Taking a photo of the game window…'));
  $('#pic-file').addEventListener('click', () => picGet('pictureFile', 'Waiting for you to choose a file…'));
  $('#pic-save').addEventListener('click', async () => {
    if (!P.src || !P.crop || P.busy) return;
    P.busy = 'Saving…'; drawPicture();
    try {
      S.state = await host.call('pictureCrop', { x: Math.round(P.crop.x), y: Math.round(P.crop.y), w: Math.round(P.crop.w), h: Math.round(P.crop.h) });
      await reload();
      picDlg.close();
      toast('ok', 'Picture saved', `This one stays: snapshots won’t replace ${S.state.active}’s picture any more. You can switch that back on in the picture window.`, 10000);
    } catch (e) { P.note = String(e && e.message || e); }
    finally { P.busy = ''; drawPicture(); }
  });
  $('#pic-remove').addEventListener('click', async () => {
    P.busy = 'Removing…'; drawPicture();
    try { S.state = await host.call('pictureRemove'); P.src = null; P.crop = null; await reload(); }
    catch (e) { P.note = String(e && e.message || e); }
    finally { P.busy = ''; drawPicture(); }
  });
  $('#pic-auto').addEventListener('change', async (ev) => {
    try { S.state = await host.call('pictureAuto', { on: ev.target.checked }); S.meta.autoPicture = ev.target.checked; }
    catch (e) { P.note = String(e && e.message || e); }
    drawPicture();
  });
  $('#pic-close').addEventListener('click', () => picDlg.close());
  picDlg.addEventListener('click', (ev) => { if (ev.target === picDlg) picDlg.close(); });
  // move (drag the box), resize (drag the corner, or the wheel), and the same from the keyboard
  let drag = null;
  picCrop.addEventListener('pointerdown', (ev) => {
    if (!P.crop) return;
    drag = { resize: ev.target === $('#pic-handle'), x: ev.clientX, y: ev.clientY, start: { ...P.crop } };
    picCrop.setPointerCapture(ev.pointerId); ev.preventDefault();
  });
  picCrop.addEventListener('pointermove', (ev) => {
    if (!drag) return;
    const k = picScale(), dx = (ev.clientX - drag.x) / k, dy = (ev.clientY - drag.y) / k, s0 = drag.start;
    P.crop = drag.resize
      ? fitCrop({ x: s0.x, y: s0.y, w: Math.min(s0.w + Math.max(dx, dy * RATIO), P.src.width - s0.x, (P.src.height - s0.y) * RATIO) })
      : fitCrop({ ...s0, x: s0.x + dx, y: s0.y + dy });
    placeCrop();
  });
  const endDrag = () => { drag = null; };
  picCrop.addEventListener('pointerup', endDrag); picCrop.addEventListener('pointercancel', endDrag);
  const resizeBy = (factor) => {
    if (!P.crop) return;
    const c = P.crop, w = Math.min(c.w * factor, P.src.width, P.src.height * RATIO);   // never past the edges
    P.crop = fitCrop({ x: c.x + (c.w - w) / 2, y: c.y + (c.h - w / RATIO) / 2, w });
    placeCrop();
  };
  $('#pic-stage').addEventListener('wheel', (ev) => { if (P.crop) { ev.preventDefault(); resizeBy(ev.deltaY < 0 ? 1.06 : 1 / 1.06); } }, { passive: false });
  picCrop.addEventListener('keydown', (ev) => {
    if (!P.crop) return;
    const step = (ev.shiftKey ? 40 : 8) / picScale();
    const move = { ArrowLeft: [-step, 0], ArrowRight: [step, 0], ArrowUp: [0, -step], ArrowDown: [0, step] }[ev.key];
    if (move) { P.crop = fitCrop({ ...P.crop, x: P.crop.x + move[0], y: P.crop.y + move[1] }); placeCrop(); ev.preventDefault(); }
    else if (ev.key === '+' || ev.key === '=') { resizeBy(1.06); ev.preventDefault(); }
    else if (ev.key === '-') { resizeBy(1 / 1.06); ev.preventDefault(); }
  });

  function explain(message) {
    const who = S.meta.character || 'your character';
    if (/ImagineClient is not running/i.test(message)) return ['The game isn\u2019t running', `Start NewMoon, log in as ${who}, then take the snapshot again.`];
    if (/No running client has/i.test(message)) return [`${who} isn\u2019t logged in`, 'None of the open game clients has that character logged in. Log in, or click the name at the top left to switch to the character that is in game.'];
    if (/No character is selected/i.test(message)) return ['No character yet', 'Start NewMoon and log in; the app reads the character\u2019s name from the game.'];
    if (/No saved stats yet/i.test(message)) return ['Take the first snapshot undigitalized', 'While digitalized the game counts the demon in your stats, so the first snapshot has to be taken when you are not digitalized (or with the Information window open).'];
    if (/Couldn't find the equipped gear/i.test(message)) return ['Open the Information window for this one', `The app couldn’t find ${who}’s gear list in the game’s memory while the Information window is closed. Open the window in game and take the snapshot again.`];
    if (/access is denied|access denied/i.test(message)) return ['Windows blocked reading the game', 'If the game runs as administrator, start this app as administrator too.'];
    return ['The snapshot didn\u2019t work', message];
  }

  async function takeSnapshot() {
    if (S.busy) return;
    S.busy = true; renderChrome();
    try {
      const r = await host.call('snapshot');
      S.demons = null;   // a snapshot can change the saved demon: ask the host for the list again next time
      await reload();
      if (r.missing) toast('warn', 'Snapshot saved, with gaps', `No saved bonuses for ${r.missing}. Open the Information window in game and take another snapshot.`, 0);
      else if (r.source === 'window' || r.source === 'status') toast('ok', 'Snapshot saved', 'Stats, gear and expertise were all read live.' + (r.picture ? ' The picture was updated from the Information window.' : ''));
      else toast('warn', 'Snapshot saved', 'Gear and expertise are live. The 12 stats and HP/MP are the saved ones, because you were digitalized (the game then counts the demon in them). Snapshot again when not digitalized to refresh them.', 12000);
    } catch (e) {
      const [title, text] = explain(String(e && e.message || e));
      toast('err', title, text, 0);
      if (/No running client has|No character is selected/i.test(String(e && e.message))) scanClients();   // who IS in game?
    } finally {
      S.busy = false; renderChrome();
    }
  }
  $('#snap').addEventListener('click', takeSnapshot);

  // settings
  const dlg = $('#settings');
  function fillSettings() {
    $('#set-character').textContent = S.settings.character || 'None yet';
    $('#set-csv').textContent = S.settings.csv || '—';
    $('#set-data').textContent = S.settings.dataDir || '—';
    document.querySelectorAll('[data-theme-choice]').forEach((b) => b.setAttribute('aria-pressed', String(b.dataset.themeChoice === S.settings.theme)));
    // Alignment: nothing is pressed until it has been set for this character (until then the reader assumes Neutral).
    const set = S.meta.alignmentKnown !== false ? S.meta.alignment : null;
    document.querySelectorAll('[data-align]').forEach((b) => { b.setAttribute('aria-pressed', String(b.dataset.align === set)); b.disabled = !S.state.active || !!S.aligning; });
    $('#align-help').textContent = S.aligning ? 'Recalculating the saved snapshots…'
      : S.alignNote ? S.alignNote
      : !S.state.active ? 'Pick a character first.'
      : !set ? 'Not read yet, so this character is treated as Neutral. The next snapshot reads it from the game, or choose it here.'
      : 'Some gear bonuses only apply to one alignment. Each snapshot reads it from the game; choosing one here overrides it until the next snapshot. Changing it recalculates this character’s saved snapshots; a copy of the spreadsheet is kept.';
  }
  document.querySelectorAll('[data-align]').forEach((b) => b.addEventListener('click', async () => {
    const value = b.dataset.align, label = b.textContent, who = S.state.active;
    if (!who || S.aligning || (S.meta.alignmentKnown !== false && S.meta.alignment === value)) return;
    S.aligning = true; S.alignNote = ''; fillSettings();
    try {
      const r = await host.call('setAlignment', { alignment: value });
      await reload();
      S.alignNote = r.changedRows ? `${who} is now ${label}. ${r.changedRows} of ${r.rows} saved snapshots changed. The spreadsheet as it was is kept as ${r.backup}.`
        : r.rows ? `${who} is now ${label}. None of the saved snapshots depend on it, so no numbers changed.`
        : `${who} is now ${label}.`;
    } catch (e) {
      S.alignNote = '';
      toast('err', 'Couldn’t change the alignment', String(e && e.message || e), 0);
    } finally { S.aligning = false; fillSettings(); }
  }));
  $('#open-settings').addEventListener('click', () => { S.alignNote = ''; fillSettings(); dlg.showModal(); });
  dlg.addEventListener('click', (ev) => { if (ev.target === dlg) dlg.close(); });
  document.querySelectorAll('[data-theme-choice]').forEach((b) => b.addEventListener('click', () => {
    applyTheme(b.dataset.themeChoice);
    host.call('saveSettings', { theme: b.dataset.themeChoice }).catch(() => {});
  }));
  $('#set-switch').addEventListener('click', () => { dlg.close(); openChooser(); });
  $('#set-picture').addEventListener('click', () => { dlg.close(); openPicture(); });
  $('#csv-show').addEventListener('click', () => host.call('reveal', { what: 'csv' }));
  $('#data-show').addEventListener('click', () => host.call('reveal', { what: 'data' }));
  $('#csv-change').addEventListener('click', async () => {
    const before = S.settings.csv;
    Object.assign(S.settings, await host.call('pickCsv'));
    fillSettings();
    if (S.settings.csv === before) return;
    if (!S.settings.csvExists) { toast('info', 'New spreadsheet', 'The first snapshot will create it. Until then the page keeps showing the earlier data.'); return; }
    try { await host.call('rebuild'); await reload(); toast('ok', 'Spreadsheet changed', 'The page now shows the snapshots in that file.'); }
    catch (e) { toast('err', 'Couldn\u2019t read that spreadsheet', String(e && e.message || e), 0); }
  });

  // keyboard
  document.addEventListener('keydown', (ev) => {
    if (document.querySelector('dialog[open], [popover]:popover-open')) return;   // any dialog (settings, characters, picture) or the demon dropdown
    if (ev.key === 'F5') { ev.preventDefault(); takeSnapshot(); return; }
    if (ev.ctrlKey && !ev.shiftKey && !ev.altKey && ev.key >= '1' && ev.key <= String(PAGES.length)) { ev.preventDefault(); go(PAGES[Number(ev.key) - 1][0]); return; }
    if ((ev.ctrlKey && ev.key.toLowerCase() === 'f') || (ev.key === '/' && !/^(INPUT|SELECT|TEXTAREA)$/.test(document.activeElement.tagName))) {
      const box = $('[data-search]');
      if (box) { ev.preventDefault(); box.focus(); box.select(); }
    }
  });

  // game status
  // Cheap check every few seconds; a full look at the clients only when one opened or closed, or (every half
  // minute) while a client is still sitting at the title screen.
  async function pollStatus() {
    if (S.scanning || S.busy) return;
    try {
      const st = await host.call('status');
      const waiting = S.state.clients.some((c) => !c.character) && Date.now() - (S.lastScan || 0) > 30000;
      if (st.clientsChanged || waiting) { await scanClients(); await settle(false); }
    } catch (e) { /* next round */ }
  }

  // ---------- start ----------
  (async () => {
    try { Object.assign(S.settings, await host.call('getSettings')); } catch (e) { /* defaults */ }
    if (Q.get('theme')) applyTheme(Q.get('theme'), true); else applyTheme(S.settings.theme || 'system');
    try { S.state = await host.call('state'); } catch (e) { /* empty state */ }
    try { await reload(); }
    catch (e) {
      $('#main').replaceChildren(h('div', { class: 'page' }, h('div', { class: 'notice notice-warn' }, icon('alert'), h('div', null, h('b', { text: 'Couldn\u2019t load the saved data. ' }), String(e && e.message || e)))));
    }
    window.__nm = { S, takeSnapshot };   // for checking the page from DevTools
    // The saved data is on screen; now see who is in game. One character: show it. Several: ask.
    if (host.native || Q.has('clients')) { await scanClients(); await settle(true); }
    requestAnimationFrame(() => requestAnimationFrame(() => host.call('ready').catch(() => {})));
    if (host.native) setInterval(pollStatus, 5000);
  })();
})();
