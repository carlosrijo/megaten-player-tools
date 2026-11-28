'use strict';
// BABEL: the page inside the desktop window.
// Data comes from the host (MainForm.cs) through host.call(cmd, args); in a plain browser a stand-in reads the repo's
// data\dash files so the page can be tested without the app (see README, "Working on the app's page").
(() => {
  const SVG = 'http://www.w3.org/2000/svg';
  const Q = new URLSearchParams(location.search);
  const $ = (sel, root) => (root || document).querySelector(sel);
  // The website (tools\build-site.ps1 marks its copy of index.html): no reader, game or files behind the page. The pages
  // about the game work; the character pages and the tools that need the reader say so. ?web=1 tries it in the preview.
  const WEB = !(window.chrome && window.chrome.webview) && (!!$('meta[name="babel-mode"][content="web"]') || Q.get('web') === '1');
  if (WEB) document.documentElement.classList.add('web');
  const RELEASES_URL = 'https://github.com/carlosrijo/megaten-player-tools/releases/latest';

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
    back: '<path d="M19 12H5"/><path d="M11 6l-6 6 6 6"/>',
    forward: '<path d="M5 12h14"/><path d="M13 6l6 6-6 6"/>',
    search: '<circle cx="11" cy="11" r="6.5"/><path d="M16 16l4.5 4.5"/>',
    alert: '<path d="M12 4l9 16H3l9-16z"/><path d="M12 10v4.5M12 17.5v.01"/>',
    check: '<circle cx="12" cy="12" r="9"/><path d="M8 12.5l2.8 2.8L16.2 9.5"/>',
    info: '<circle cx="12" cy="12" r="9"/><path d="M12 11v5.5M12 7.8v.01"/>',
    x: '<path d="M6 6l12 12M18 6L6 18"/>',
    chat: '<path d="M4 5h16v11H10l-4.5 3.5V16H4z"/><path d="M8 9.5h8M8 12.5h5"/>',
    book: '<path d="M5 4.5h10.5a3 3 0 0 1 3 3V20H8a3 3 0 0 1-3-3V4.5z"/><path d="M5 17a3 3 0 0 1 3-3h10.5"/>',
    fuse: '<circle cx="8" cy="8" r="3.5"/><circle cx="16" cy="8" r="3.5"/><path d="M8 11.5v1.5a4 4 0 0 0 4 4 4 4 0 0 0 4-4v-1.5M12 17v3.5"/>',
    horns: '<path d="M6.5 3.5c-.6 3 .4 5.4 2.6 6.6M17.5 3.5c.6 3-.4 5.4-2.6 6.6"/><circle cx="12" cy="14.5" r="6"/><path d="M9.6 14h.01M14.4 14h.01M10 17.2h4"/>',
    flask: '<path d="M9 3.5h6M10 3.5v5.2L5.2 17.6A2 2 0 0 0 7 20.5h10a2 2 0 0 0 1.8-2.9L14 8.7V3.5"/><path d="M7.6 14.5h8.8"/>',
    podium: '<path d="M3.5 20h17M4.5 20v-5.5h5V20M9.5 20V9h5v11M14.5 20v-7.5h5V20"/><path d="M12 4.2v2"/>',
    plus: '<path d="M12 5v14M5 12h14"/>',
    gate: '<path d="M5 20.5V10a7 7 0 0 1 14 0v10.5"/><path d="M3 20.5h18"/><path d="M9.5 20.5v-5a2.5 2.5 0 0 1 5 0v5"/>',
    map: '<path d="M3.5 6.5l5.5-2.5 6 2.5 5.5-2.5v13.5l-5.5 2.5-6-2.5-5.5 2.5z"/><path d="M9 4v13.5M15 6.5V20"/>',
    copy: '<rect x="8.5" y="8.5" width="11" height="11" rx="2"/><path d="M5.5 15.5h-1a1 1 0 0 1-1-1v-10a1 1 0 0 1 1-1h10a1 1 0 0 1 1 1v1"/>',
    route: '<circle cx="6" cy="18.5" r="2.2"/><circle cx="18" cy="5.5" r="2.2"/><path d="M8.2 18.5H15a3.2 3.2 0 0 0 0-6.4H9a3.2 3.2 0 0 1 0-6.4h6.8"/>',
    rune: '<path d="M12 4v16M12 4l-5 5M12 4l5 5"/><path d="M7.5 20h9"/>',
    trash: '<path d="M4.5 7h15M9.5 7V4.5h5V7M6.5 7l1 12.5h9l1-12.5M10 10.5v6M14 10.5v6"/>',
    share: '<path d="M12 15V4M8 7.5l4-4 4 4"/><path d="M5 12.5v6a1 1 0 0 0 1 1h12a1 1 0 0 0 1-1v-6"/>',
    download: '<path d="M12 4v11M8 11.5l4 4 4-4"/><path d="M5 15v3.5a1 1 0 0 0 1 1h12a1 1 0 0 0 1-1V15"/>',
    flag: '<path d="M5.5 21V4"/><path d="M5.5 4.5h11.5l-2.2 4.2 2.2 4.3H5.5"/>',
    key: '<circle cx="8" cy="15" r="4.5"/><path d="M11.2 11.8L20 3M16.5 6.5l2.5 2.5M14 9l2 2"/>',
    // shown / hidden (the quest panel); panel.js has the same two
    eye: '<path d="M2.5 12S6 5.5 12 5.5 21.5 12 21.5 12 18 18.5 12 18.5 2.5 12 2.5 12z"/><circle cx="12" cy="12" r="3"/>',
    'eye-off': '<path d="M2.5 12S6 5.5 12 5.5 21.5 12 21.5 12 18 18.5 12 18.5 2.5 12 2.5 12z"/><circle cx="12" cy="12" r="3"/><path d="M4 4l16 16"/>',
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
    // 'babel-' keys; a value saved by 0.4.0 or older (prefix 'nm-') is still read until it is saved again
    get(k) { try { const v = localStorage.getItem('babel-' + k); return v !== null ? v : localStorage.getItem('nm-' + k); } catch (e) { return null; } },
    set(k, v) { try { localStorage.setItem('babel-' + k, v); } catch (e) { /* preferences only */ } },
  };
  // Feature flags: features still being tried out are off unless the run turns them on by name, ?features=a,b
  // (the app: BABEL.exe --query features=a,b). Nothing is saved: the next plain start has them off again.
  //   assistant-picker   Settings > Assistant gets a demon picker: any demon from the game's files stands in the corner
  const FEATURES = new Set((Q.get('features') || '').split(',').map((s) => s.trim()).filter(Boolean));
  const feature = (name) => FEATURES.has(name);
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
    // The website: no character, no game; the theme is kept in the browser. Anything else needs the desktop app.
    if (WEB) {
      const settings = { theme: store.get('theme') || 'system' };
      const answers = {
        state: () => ({ active: null, scanned: true, clients: [], characters: [] }),
        load: () => ({ snapshots: [], gear: [], expertise: [], bonuses: [], ranks: null, digitalize: null, icons: {}, unlocks: [], summoned: null, meta: {} }),
        status: () => ({ gameRunning: false, busy: false, clientsChanged: false }),
        getSettings: () => ({ ...settings }),
        saveSettings: (a) => { if (a && a.theme) { settings.theme = a.theme; store.set('theme', a.theme); } return { ...settings }; },
        updateCheck: () => ({ enabled: true, off: true }),
        menuIcons: () => ({}), log: () => true, ready: () => true, questProgress: () => null, summonedDemon: () => null, assistant: () => null,
      };
      return { native: false, web: true, async call(cmd, args) {
        if (answers[cmd]) return answers[cmd](args);
        throw Object.assign(new Error('This needs BABEL, the desktop app, on your PC.'), { kind: 'user' });
      } };
    }
    // Stand-in for a plain browser: ?empty=1 (first run), ?snap=fail, ?clients=0|1|2 (how many characters are in game)
    const get = (p, dflt) => fetch(p).then((r) => (r.ok ? r.json() : dflt)).catch(() => dflt);
    const settings = { character: 'Rubito', csv: 'C:\\Users\\…\\smt-stats\\rubito-stats.csv', theme: store.get('theme') || 'system', dataDir: '…\\imagine-reader\\data', csvExists: true, updateCheck: true, updatePre: false };
    const wait = (ms) => new Promise((r) => setTimeout(r, ms));
    // Updates: ?update=1 (a new version is out), ?update=pre (a test version, with "Include test versions" ticked),
    // ?update=nosig (out, but not installable from inside), ?update=fail (GitHub can't be reached), ?updated=0.9.0
    // (just updated). Installing runs a pretend download, then says the preview can't install.
    const fakeUpd = { phase: 'idle' };
    const fake = { active: 'Rubito', scanned: false, clients: [], characters: [{ name: 'Rubito', hasData: true, inGame: false, folder: '', level: 0 }] };
    // ?char=<name>: start as another saved character; only their levels (data\characters\<name>\expertise-ranks.json) load
    if (Q.get('char') && Q.get('char') !== 'Rubito') {
      fake.active = Q.get('char');
      fake.characters.push({ name: fake.active, hasData: true, inGame: false, folder: '', level: 0 });
    }
    const fakeClients = [{ pid: 7224, folder: 'client2', character: 'Rubito', level: 95 }, { pid: 30184, folder: 'client1', character: 'Rokujuu', level: 97 }];
    const fakeState = () => {
      fake.characters.forEach((c) => { const g = fake.clients.find((x) => x.character === c.name); c.inGame = !!g; c.folder = g ? g.folder : ''; c.level = g ? g.level : 0; c.picture = fakePic[c.name] || null; });
      return JSON.parse(JSON.stringify(fake));
    };
    // pictures: a drawn stand-in for the game window, cropped here in the browser
    const fakePic = {}, fakeSrc = {}, fakeAuto = {}, fakeGone = new Set();   // fakeGone: snapshots (taken_at) and demons (key) deleted in this preview, in memory only
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
        if (cmd === 'state') {
          (Q.get('chars') || '').split(',').filter(Boolean).forEach((n) => { if (!fake.characters.some((c) => c.name === n)) fake.characters.push({ name: n, hasData: true, inGame: false, folder: '', level: 0 }); });
          return fakeState();
        }
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
          // ?dash=characters/Nobitta: another character's saved page data under Rubito's name (to look at other builds)
          const dd = /^[\w/-]+$/.test(Q.get('dash') || '') ? `../../data/${Q.get('dash')}/` : '../../data/';
          let [snapshots, gear, expertise, bonuses, ranks, digitalize, icons, unlocks] = empty ? [[], [], [], [], null, null, {}, []] : await Promise.all([
            get(dd + 'dash/snapshots.json', []), get(dd + 'dash/gear.json', []), get(dd + 'dash/expertise.json', []),
            get(dd + 'dash/bonuses.json', []), get(dd + 'expertise-ranks.json', null), get(Q.get('digi') || dd + 'dash/digitalize.json', null), get(dd + 'dash/icons.json', {}), get(dd + 'dash/digi-unlocks.json', [])]);
          if (empty && Q.get('empty') !== '1') ranks = await get(`../../data/characters/${encodeURIComponent(fake.active)}/expertise-ranks.json`, null);
          // the summoned demon comes with the page data (dash\summoned.json); here the stand-in list's summoned one
          snapshots = snapshots.filter((x) => !fakeGone.has(x.taken_at));   // deleted here: gone until the page is reloaded
          // (the saved one when there is one, as the app shows it: its real skills, Demon Force, ranks and stats)
          const summoned = empty ? null : Q.get('summoned') === '0' ? null : (await get(dd + 'dash/summoned.json', null)) || (await this.call('compDemons', { cached: true })).find((d) => d.summoned) || null;
          const pf = empty ? null : await get('../../data/cache/profile.json', null);
          const savedAt = pf && /^Updated (\d{4}-\d{2}-\d{2} \d{2}:\d{2})/.exec(String(pf.note || ''));
          const togglesSource = !pf ? null : pf.toggles_source || (pf.toggles && savedAt ? 'saved, ' + savedAt[1] : null);
          return { snapshots, gear, expertise, bonuses, ranks, digitalize, icons, unlocks, summoned, meta: { togglesSource, character: fake.active, alignment: (fake.align && fake.align[fake.active]) || 'NEUTRAL', alignmentKnown: fake.active === 'Rubito' || !!(fake.align && fake.align[fake.active]), csv: settings.csv, dataDir: settings.dataDir, gameRunning: fake.clients.length > 0, picture: fakePic[fake.active] || null, autoPicture: fakeAuto[fake.active] !== false, demonPicture: empty ? null : '../../data/dash/demon.png', skillIcons: empty ? null : '../../data/dash/skill-icons/' } };
        }
        if (cmd === 'snapshot') {
          await wait(1400);
          if (Q.get('snap') === 'fail') throw Object.assign(new Error('ImagineClient is not running.'), { kind: 'user' });
          return { source: Q.get('snap') === 'live' ? 'window' : 'cache', statsNote: 'Saved 2026-11-22 21:06', missing: '', level: 95, ranks: 31 };
        }
        if (cmd === 'status') return { gameRunning: fake.clients.length > 0, busy: false, clientsChanged: false };
        if (cmd === 'menuIcons') return {};   // the menu bar's buttons come from the game's files: just the words here
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
            ['Seraph', 66, 'Divine', '', 2, true, false], ['Pixie', 2, 'Fairy', '', 0, false, false], ['Santa Black Frost', 42, 'Nocturne', 'Kusi', 1, false, false]].map(([name, level, race, mitama, forceStack, equipment, inGame]) =>
            ({ key: name, name, level, race, mitama, forceStack, equipment, inGame: !(args && args.cached) && inGame, current: name === cur, summoned: name === 'Specter' && Q.get('summoned') !== '0', icon: null, loaded: name !== 'Pixie', inComp: name !== 'Seraph', seenAt: '2026-11-23T15:40:23',
              stats: name === 'Specter' ? [4819, 1767, 143, 198, 117, 142, 124, 97, 172, 159, 199, 210, 87, 53] : null,   // made-up numbers for the partner card
              // the Rebirth Planner's fields: the demon's type, growth type, rebirth ranks, Mitama type and rank, slot bonus ids
              type: { Specter: 11004, Shikigami: 11030, Alilat: 10041, Seraph: 22, Pixie: 570, 'Santa Black Frost': 10733 }[name], growthType: { Specter: 215, Shikigami: 123, Alilat: 118, Seraph: 118 }[name] || 113,
              reunion: mitama ? (name === 'Specter' ? [8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8] : name === 'Shikigami' ? [4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4] : [3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]) : name === 'Seraph' ? [4, 4, 4, 0, 0, 0, 0, 0, 0, 0, 0, 0] : [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
              mitamaType: { Kusi: 3, Ara: 1, Nigi: 2 }[mitama] || 0, mitamaRank: name === 'Specter' ? 12 : 0, mitamaSlots: name === 'Specter' ? [65, 66, 67, 68, 1, 2, 3, 4, ...Array(88).fill(0)] : Array(96).fill(0),
              // the Demon Force stack ids (Alilat's as read in game 2026-11-26: seven item effects and a random one; Shikigami: random ones only; Seraph: 2 of its 2 slots)
              forceStackIds: { Alilat: [10144, 10269, 1298, 10271, 10283, 10272, 10118, 10270], Shikigami: [1084, 4, 5, 419, 820, 463, 427, 2057], Seraph: [10001, 10002, 0, 0, 0, 0, 0, 0] }[name] || Array(8).fill(0),
              // stand-ins for the three states of the unlock tag
              unlock: name === 'Alilat' ? { listed: false, level: 0, raceLevel: 1, race, reached: false } : { listed: true, level: name === 'Seraph' ? 8 : 5, raceLevel: name === 'Seraph' ? 5 : 10, race, reached: name !== 'Seraph' },
              // the saved demon's own numbers as its offer, so its card looks as in the app (the others have none here)
              offer: name === cur && S.data && S.data.digitalize ? (({ rate, alignment, stats, skills }) => ({ rate, alignment, stats, skills, combat: {} }))(S.data.digitalize) : null }))
            .filter((d) => !fakeGone.has('demon:' + d.key));
        }
        // Deleting here only hides it in this preview (until the page is reloaded): the files are the app's to change
        if (cmd === 'forgetDemon') {
          await wait(200);
          const d = (await this.call('compDemons', { cached: true })).find((x) => x.key === args.key);
          if (!d) throw Object.assign(new Error('That demon isn’t in the list any more.'), { kind: 'user' });
          if (d.current) throw Object.assign(new Error('That is your digitalize demon. Pick another one (or No demon) first, then remove it.'), { kind: 'user' });
          if (d.inComp === true || d.summoned) throw Object.assign(new Error('That demon is still in your COMP, so it would come back the next time the demons are read. Only demons no longer in your COMP can be removed.'), { kind: 'user' });
          fakeGone.add('demon:' + args.key);
          return true;
        }
        if (cmd === 'deleteSnapshot') {
          await wait(400);
          fakeGone.add(args.takenAt);
          return { backup: 'rubito-stats.backup-before-delete-20261125-120000.csv' };
        }
        // The browser preview has no game and no files to change: say so instead of pretending (the desktop app switches)
        if (cmd === 'useDemon') { await wait(300); throw Object.assign(new Error('This is the browser preview: it shows stand-in demons and can’t change your data. Pick the demon in the desktop app (BABEL.exe).'), { kind: 'user' }); }
        if (cmd === 'setAlignment') {
          await wait(900);
          fake.align = fake.align || {}; fake.align[fake.active] = args.alignment;
          return { changed: true, rows: fake.active === 'Rubito' ? 5 : 0, changedRows: fake.active === 'Rubito' ? 3 : 0, backup: 'rubito-stats.backup-before-alignment-20261122-233212.csv' };
        }
        // The Build Planner's numbers and saved builds are the reader's own: the browser preview has them when it is
        // served by "imagine-reader serve" (src\Build\PreviewServer.cs), which answers these like the app does.
        if (cmd.startsWith('sim')) {
          const r = await fetch('/host/' + cmd, { method: 'POST', body: JSON.stringify(args || {}) }).catch(() => null);
          if (!r || !r.ok) throw Object.assign(new Error('This browser preview has no reader behind it, so it can’t work out a build. Open it through “imagine-reader serve”, or use the desktop app.'), { kind: 'user' });
          const m = await r.json();
          if (!m.ok) throw Object.assign(new Error(m.error), { kind: m.kind });
          return m.result;
        }
        // the assistant's model: the file the reader made (imagine-reader.exe model); ?assistant=none stands in for "game never seen"
        // { demon } (the picker): its model from reference\devil-models.tsv, the file made by "imagine-reader model <model>"
        if (cmd === 'assistant') {
          if (Q.get('assistant') === 'none') return null;
          const id = args && args.demon > 0 ? args.demon : 723;
          const tsv = await fetch('../../reference/devil-models.tsv').then((r) => (r.ok ? r.text() : '')).catch(() => '');
          const row = tsv.split(/\r?\n/).map((l) => l.split('\t')).find((r) => r[0] === String(id));
          if (!row && id !== 723) return null;
          const m = await get(`../../data/cache/models/${row ? row[2] : 'D2_11_003a'}.json`, null);
          return m ? { demon: args && args.demon > 0 ? id : 0, summon: { scale: row ? +row[3] : 1, circle: row ? +row[4] : 1 }, ...m } : null;
        }
        // the summoning effect's pieces: the file made by "imagine-reader model summon"
        if (cmd === 'assistantSummon') return get('../../data/cache/models/summon.json', null);
        // the summoned demon "in game": the saved one (dash\summoned.json); ?summon=<demon id> stands in for another
        // the character's quests "in game": a sample (Demonization with two of its four talks done, Shibuya Yagiya
        // Support 1 with the window's counters, Act 13 at Uriel, Act 3 at the heliport; in Shinjuku Babel, ?zone=<id> for another); ?quests=none: no game, ?completed=none: list not loaded
        // the quest panel: in the browser, a small popup with panel.html (it uses the same sample)
        if (cmd === 'panel') {
          if (!args || args.open !== false) window.open('panel.html' + location.search, 'babel-panel', 'width=380,height=300');
          return true;
        }
        if (cmd === 'questProgress') {
          if (Q.get('quests') === 'none') return null;
          // ?quests=extra: also Act 6 with one blade (valuable 124), Act.8 at its four residents (a counter step the client
          // cannot tick) and Let's Trade! at its random request. The flags also hold a few key items for the Key Items page
          // (Old World Stone, Proof of Deposit, WILDCAT, both compendiums).
          const extra = Q.get('quests') === 'extra'
            ? [{ id: 907, step: 0, counters: [0, 0, 0, 0, 0] }, { id: 909, step: 0, counters: [0, 0, 0, 0, 0] }, { id: 7, step: 0, counters: [0, 0, 0, 0, 0] }] : [];
          return { active: [{ id: 902, step: 0, counters: [0, 0, 0, 0, 0] }, { id: 818, step: 0, counters: [7, 13, 5, 0, 0] }, { id: 312, step: 2, counters: [0, 0, 0, 0, 0] }, { id: 904, step: 0, counters: [0, 0, 0, 0, 0] }, ...extra],
            completed: Q.get('completed') === 'none' ? null : [1, 2, 3, 4, 5, 901], flags: [...(extra.length ? [82, 84, 124] : [82, 84]), 4, 132, 222, 250, 251], zone: Number(Q.get('zone')) || 50101 };
        }
        if (cmd === 'summonedDemon') {
          if (Q.get('summon')) return { type: Number(Q.get('summon')) || 0, name: Q.get('summonName') || 'A demon' };
          const s = await get('../../data/dash/summoned.json', null);
          return s && s.type ? { type: s.type, name: s.name } : { type: 0 };
        }
        if (cmd === 'getSettings' || cmd === 'pickCsv') return { ...settings };
        if (cmd === 'saveSettings') { Object.assign(settings, args); return { ...settings }; }
        // Troubleshooting: the stand-in has no build, game or files to look at, so it shows what the answers look like
        if (cmd === 'about') return { program: 'BABEL', version: '0.0.0', commit: 'browser preview', built: 'not built', windows: 'Windows (the browser preview does not ask)', webview2: 'none: this is a browser',
          text: 'BABEL (browser preview)\r\nThe stand-in for the app has no real version to show.\r\n', dataDir: settings.dataDir, logDir: settings.dataDir + '\\logs', logLevel: 'info' };
        if (cmd === 'doctor') {
          await wait(900);
          const c = (section, name, status, detail, advice, ms) => ({ section, name, status, detail, advice: advice || '', ms: ms || 0 });
          const checks = [c('App', 'Program version', 'pass', '0.0.0 (browser preview)'), c('App', 'WebView2', 'pass', 'This is a browser, not the app'),
            c('Files', 'Data folder', 'pass', settings.dataDir + ' (writable)', '', 3), c('Files', 'Server constants', 'warn', 'C:\\…\\constants.xml does not exist.', 'Not needed: the reader falls back to the usual values.'),
            c('Game', 'Game running', 'warn', 'No ImagineClient is running.', 'Start IMAGINE and log in to take a snapshot.'),
            c('Client 1: client1, Rubito', 'Client build', 'fail', 'Shown only so the page can be tried: nothing here was checked.', 'The real check runs in the desktop app.', 120)];
          const n = (s) => checks.filter((x) => x.status === s).length;
          return { program: 'BABEL (browser preview)', windows: 'Windows', checkedAt: '', checks, text: 'Browser preview: nothing was checked.\r\n',
            summary: { pass: n('pass'), warn: n('warn'), fail: n('fail'), worst: 'fail', text: `${n('pass')} passed, ${n('warn')} warning, ${n('fail')} failed` } };
        }
        if (cmd === 'feedbackFiles') {   // what the files look like; the real ones come from the app
          await wait(600);
          const who = args && args.includeNames ? 'Rubito' : 'Character 1';
          const f = (name, text) => ({ name, text, bytes: text.length });
          return { includeNames: !!(args && args.includeNames), files: [
            f('about.txt', 'BABEL (browser preview)\r\nStand-in files: the real ones come from the desktop app.\r\n'),
            f('doctor.txt', 'Browser preview: nothing was checked.\r\n'),
            f('logs/babel.log', `2026-11-25 10:00:00 INFO snapshot of ${who}: 412 values\r\n`.repeat(400)),
            f('data/config.json', '{ "statsFolder": "%USERPROFILE%\\\\Documents\\\\smt-stats" }'),
            f('data/characters.json', `[{ "Name": "${who}" }]`),
            f('settings.json', '{ "Theme": "system" }'),
            f('data-listing.txt', 'cache\\models\\D2_11_003a.json  182 KB  2026-11-24\r\n'.repeat(60)) ] };
        }
        if (cmd === 'saveBundle') { await wait(300); throw Object.assign(new Error('This is the browser preview: it can’t save a support bundle. Use the desktop app (BABEL.exe).'), { kind: 'user' }); }
        if (cmd === 'buildCheck') return Q.get('build') === 'old' ? { clients: 1, known: 0, unknown: [{ folder: 'client1', pid: 1, fingerprint: 'preview', same: false }] } : { clients: fake.clients.length, known: fake.clients.length, unknown: [] };
        if (cmd === 'log') return true;   // the page's error reports: nothing to write them to here
        if (cmd === 'updateCheck') {
          if (args && args.auto && !settings.updateCheck) return { enabled: true, off: true };
          await wait(args && args.force ? 700 : 150);
          const u = Q.get('update'), base = { enabled: true, current: '0.10.0', checkedAt: '2026-11-27 18:05' };
          if (u === 'fail') return { ...base, error: 'Couldn’t reach GitHub: The server name or address could not be resolved' };
          const pre = u === 'pre' && settings.updatePre;
          if (u !== '1' && u !== 'nosig' && !pre) return { ...base, update: null };
          const v = pre ? '0.11.0-beta.1' : '0.10.1';
          return { ...base, update: { version: v, pre, installable: u !== 'nosig', size: 11157504,
            notes: `## What's new in ${v}\n\n- **Updates**: BABEL looks for a new version when it starts, and installs it when you say so.\n- The Gear page remembers the slot you looked at last.\n- Fixed: the Quest Tracker sometimes showed a finished quest as open.` } };
        }
        if (cmd === 'updateStatus') return { ...fakeUpd };
        if (cmd === 'updateInstall') {
          const total = 11157504;
          for (let done = 0; done <= total; done += 1400000) { Object.assign(fakeUpd, { phase: 'downloading', done, total }); await wait(180); }
          fakeUpd.phase = 'checking'; await wait(500);
          Object.assign(fakeUpd, { phase: 'failed', error: 'This is the browser preview: it can’t install anything.' });
          throw Object.assign(new Error('This is the browser preview: it can’t install anything. Use the desktop app (BABEL.exe).'), { kind: 'user' });
        }
        if (cmd === 'updatePage') return true;
        return true;
      },
    };
  })();

  // ---------- errors the page can't handle go into the app's log ----------
  // window.onerror and unhandled promise rejections are sent to the host ("log" command) so a problem someone sees can be
  // found in logs\ afterwards. A few per minute at most, the same one only once; the host limits them again, and the
  // browser stand-in simply answers true. Nothing here may ever make the page fail.
  (() => {
    const seen = new Set();
    let sent = 0, since = Date.now();
    const report = (kind, message, source, stack) => {
      try {
        if (Date.now() - since > 60000) { since = Date.now(); sent = 0; seen.clear(); }
        const key = kind + '|' + message + '|' + source;
        if (sent >= 10 || seen.has(key)) return;
        seen.add(key); sent++;
        host.call('log', { kind, message: String(message).slice(0, 500), source: String(source || '').slice(0, 200), stack: String(stack || '').slice(0, 1500) }).catch(() => {});
      } catch (e) { /* the report must not cause another one */ }
    };
    window.addEventListener('error', (ev) => report('error', ev.message, `${ev.filename || ''}:${ev.lineno || 0}:${ev.colno || 0}`, ev.error && ev.error.stack));
    window.addEventListener('unhandledrejection', (ev) => { const r = ev.reason; report('unhandledrejection', (r && r.message) || r, '', r && r.stack); });
  })();

  // ---------- what the page knows ----------
  const STATS = [['str', 'STR'], ['mag', 'MAG'], ['vit', 'VIT'], ['int', 'INT'], ['spd', 'SPD'], ['luck', 'LUCK'],
    ['clsr', 'CLS-R'], ['lngr', 'LNG-R'], ['spell', 'Spell'], ['support', 'Support'], ['pdef', 'P.Def'], ['mdef', 'M.Def']];
  const COMBAT = [
    ['crit', 'Critical', ''], ['final_crit', 'Final critical correction', '%'], ['lb_chance', 'Limit break chance', '%'], ['lb_power', 'Limit break power', '%'],
    ['lb_cap', 'Limit break damage cap', ''], ['pursuit_chance', 'Pursuit chance', '%'], ['pursuit_power', 'Pursuit power', '%'],
    ['ta_chance', 'Technical attack chance', '%'], ['ta_power', 'Technical attack power', '%'],
    ['dmg_close', 'Close-range damage dealt', '%'], ['dmg_long', 'Long-range damage dealt', '%'], ['dmg_spell', 'Spell damage dealt', '%'], ['dmg_support', 'Support damage dealt', '%'],
    ['taken_close', 'Close-range damage taken', '%'], ['taken_long', 'Long-range damage taken', '%'], ['taken_spell', 'Spell damage taken', '%'], ['taken_support', 'Support damage taken', '%'],
    ['boost_spin', 'Spin skill boost', '%'], ['boost_attack', 'Attack skill boost', '%'], ['boost_rush', 'Rush skill boost', '%'], ['boost_rapid', 'Rapid skill boost', '%'], ['boost_shot', 'Shot skill boost', '%'],
    ['cooldown', 'Cooldown', '%'], ['move_speed', 'Movement speed', '%'],
  ];
  // The Overview's stat sheet: one line per key stat, in two columns of named groups.
  const SHEET = [
    [['Offence', [['crit', 'Critical'], ['final_crit', 'Final correction'], ['lb_chance', 'Limit break chance'], ['lb_power', 'Limit break power'], ['lb_cap', 'Limit break cap'],
      ['ta_chance', 'Technical chance'], ['ta_power', 'Technical power'], ['pursuit_chance', 'Pursuit chance'], ['pursuit_power', 'Pursuit power']]],
      ['Experience', [['xp_earned', 'EXP earned'], ['xp_partner', 'Partner EXP earned'], ['xp_expertise', 'Expertise earned'], ['soul_earned', 'Soul points earned'], ['digi_points', 'Digitalize points earned'], ['bethel_gain', 'Bethel gain']]],
      ['Skill costs', [['hp_cost', 'HP cost'], ['mp_cost', 'MP cost']]]],
    [['Damage dealt', [['dmg_close', 'Close-range'], ['dmg_long', 'Long-range'], ['dmg_spell', 'Spell'], ['dmg_support', 'Support']]],
      ['Damage taken', [['taken_close', 'Close-range'], ['taken_long', 'Long-range'], ['taken_spell', 'Spell'], ['taken_support', 'Support']]],
      ['Action boosts', [['boost_spin', 'Spin'], ['boost_attack', 'Attack'], ['boost_rush', 'Rush'], ['boost_rapid', 'Rapid'], ['boost_shot', 'Shot']]],
      ['Speed', [['cooldown', 'Cooldown'], ['incantation', 'Incantation'], ['move_speed', 'Movement']]]],
  ];
  /** The Bonuses row behind a key stat (same name, same unit; the cap is called "Limit break cap" there). */
  const BONUS_NAME = { lb_cap: 'Limit break cap' };
  /** Sheet lines that are not combat stats: their Bonuses row is in the "Other" group (same name as the game's, so not the line's label). */
  // (key, the Bonuses row's name, unit, its group, and for the two that no character here has yet: the wordings it may come in)
  const OTHER_STATS = [['xp_earned', 'EXP Earned', '%', 'Other'], ['xp_partner', 'EXP Earned (only with certain partners)', '%', 'Only in some fights'], ['xp_expertise', 'Expertise Earned', '%', 'Other'],
    ['soul_earned', 'Soul Points Earned', '%', 'Other', /^soul points?( earned| gain(ed)?)?$/i], ['digi_points', 'Digitalize Points', '%', 'Other', /^digitali[sz](e|ation) points?( earned| gain(ed)?)?$/i],
    ['bethel_gain', 'Bethel Gain', '%', 'Other', /^bethel( earned| gain(ed)?)?$/i],
    ['incantation', 'Incantation', '%', 'Other'],
    // what every skill costs you to use: negative when something makes skills cheaper (the wording in the data varies a little)
    ['hp_cost', 'HP cost of all skills', '%', 'Other', /^(hp cost of all skills|all skills hp cost)$/i],
    ['mp_cost', 'All skills MP cost', '%', 'Other', /^(all skills mp cost|mp cost of all skills)$/i]];   // the cast time: its Bonuses row has the same parts as cooldown's (gear, expertise, demon, summoned, VIT/INT and SPD)
  const bonusRowOf = (k, label, unit, list) => {   // list: another set of bonus rows (the Character Planner's), else the character's
    const o = OTHER_STATS.find((x) => x[0] === k);
    return (list || S.data.bonuses || []).find((r) => r.group === (o ? o[3] : 'Combat') && (o && o[4] ? o[4].test(String(r.bonus).trim()) : r.bonus === (BONUS_NAME[k] || label)) && (o && o[4] ? true : (r.unit || '') === (unit || '')));
  };
  /**
   * Partner EXP earned: the Demonology "<Race> Sympathy" skills each give +N% XP to your summoned demon when it is of that
   * race (their tooltips say so). The Bonuses page keeps them under "Only in some fights"; here the ones that match the demon
   * summoned now (the summoned card's) are added up. Nobody is out while digitalized.
   */
  function partnerXp(row) {
    const d = S.demons ? S.demons.find((x) => x.summoned) : S.data.summoned;
    const sym = (row && Array.isArray(row.parts) ? row.parts : []).map((p) => ({ p, race: (/^(.+?) Sympathy\b/.exec(String(p.source || '')) || [])[1] })).filter((x) => x.race);
    const mine = d && d.race ? sym.filter((x) => x.race.toLowerCase() === String(d.race).toLowerCase()) : [];
    return { demon: d || null, races: sym.map((x) => x.race), parts: mine.map((x) => x.p), value: round2(mine.reduce((a, x) => a + n0(x.p.value), 0)) };
  }
  /** What the expertise column of a sheet line is made of, named as its parts are ("Clan bonuses": the clan's skills; EXP Earned gets it from there, not from expertise ranks). */
  const expShareName = (row) => {
    const kinds = [...new Set(((row && row.parts) || []).filter((p) => p.kind === 'expertise' || p.kind === 'clan').map((p) => KIND[p.kind]))];
    return kinds.length ? kinds.join(' + ') : 'Expertise';
  };
  /** "Alilat (summoned)": the summoned demon's name as its bonus parts carry it (empty when it gives nothing). */
  const summonedName = () => { for (const r of (S.data.bonuses || [])) { const p = (r.parts || []).find((x) => x.kind === 'summoned'); if (p) return String(p.source); } return ''; };
  const TOP_CHANGES = 5;   // the Overview lists this many changes until "Show all"
  const PAGES = [['overview', 'Overview', 'grid'], ['bonuses', 'Bonuses', 'list'], ['gear', 'Gear', 'shield'], ['expertise', 'Expertise', 'star'], ['digitalize', 'Digitalize', 'bolt'], ['history', 'History', 'chart']];
  // Tools: pages about the game itself, not about the character (they work before the first snapshot too).
  const TOOLS = [['skills', 'Skill Database', 'bolt'], ['ddb', 'Demon Database', 'horns'], ['geardb', 'Item Database', 'shield'], ['quests', 'Quest Database', 'book'], ['keys', 'Key Items', 'key'], ['dcm', 'DCM Locator', 'search'], ['rank', 'Demon Ranking', 'podium'],['dungeons', 'Dungeon Database', 'gate'], ['guides', 'Dungeon Guides', 'map'], ['tracker', 'Quest Tracker', 'flag'], ['fusion', 'Fusion Planner', 'fuse'], ['dplan', 'Digitalize Planner', 'route'], ['rebirth', 'Demon Planner', 'horns'], ['sim', 'Character Planner', 'flask']];
  const isTool = (page) => TOOLS.some((t) => t[0] === page);
  const BONUS_GROUPS = [
    ['Combat', 'Combat', 'You · combat'], ['Stats', 'Stats', 'You · stats'], ['Affinity', 'Affinity', 'You · affinity'], ['Other', 'Other', 'You · other'],
    ['Partner', 'Partner', 'Your partner'], ['Only in some fights', 'Situational', 'Only in some fights · not added to the totals'],
  ];
  const KIND = { item: 'Item', mod: 'Mod', tarot: 'Tarot', soul: 'Soul fusion', appearance: 'Appearance', set: 'Set bonus', 'fusion set': 'Fusion set', expertise: 'Expertise', clan: 'Clan bonuses', dlevel: 'Digitalize level', quest: 'Quest bonus', compendium: 'Compendium', demon: 'Digitalize demon', summoned: 'Summoned demon', stats: 'Your stats' };
  const KIND_ORDER = ['item', 'mod', 'tarot', 'soul', 'appearance', 'set', 'fusion set', 'expertise', 'clan', 'dlevel', 'quest', 'compendium', 'demon'];
  const lowIsGood = (name) => /taken|cooldown|cast time|incantation|cost/i.test(name);

  // Stats that give nothing past a ceiling (from the user, 2026-11-23). The ceiling applies to the whole value
  // (gear + expertise, in the shown mode). To add one: its key from COMBAT above and its ceiling.
  // A negative one is a floor: cooldown stops at 5% of the skill's own (the server's lowest COOLDOWN_TIME).
  const CAPS = { final_crit: 100, lb_chance: 100, ta_chance: 100, pursuit_chance: 100, pursuit_power: 100, cooldown: -95, incantation: -100 };
  const INCANTATION_CAP = -100;   // incantation (cast) time stops at 0
  /** How far a total is past its cap (0 = at it, negative = not there yet); a negative cap is a floor. */
  const pastCap = (total, cap) => round2(cap < 0 ? cap - total : total - cap);
  const capOfBonus = (row) => {   // the same list for a row of the bonus list (your own bonuses only)
    if (!row) return null;
    if (row.group === 'Other' && /^(incantation|cast time)$/i.test(String(row.bonus))) return INCANTATION_CAP;
    if (row.group !== 'Combat') return null;
    const hit = COMBAT.find(([, label]) => label === row.bonus);
    return hit && CAPS[hit[0]] !== undefined ? CAPS[hit[0]] : null;
  };
  /** What VIT/INT and SPD take off a row in the shown mode (cooldown, incantation), or 0. */
  const statsOf = (row) => n0(row && (digi() ? row.stats_digi : row.stats));
  /** The stats part of a row in the shown mode (kind "stats": one normal, one digitalized). */
  const statsPart = (row) => ((row && row.parts) || []).find((p) => p.kind === 'stats' && isTrue(p.digi) === digi())
    || ((row && row.parts) || []).find((p) => p.kind === 'stats' && !isTrue(p.digi));
  /** "From VIT 67, SPD 255: 2.5% per 10 VIT and 1.5% per 10 SPD, rounded down", or null. */
  const statsNote = (row) => { const p = statsPart(row); return p ? `From ${p.source}${digi() && isTrue(p.digi) ? ' (digitalized)' : ''}: ${p.note}` : null; };
  /** "At the 100% cap" / "10% over the cap" as a small tag, or nothing while below it. */
  function capTag(total, cap, unit, style) {
    if (cap === null || cap === undefined || pastCap(total, cap) < 0) return null;
    const over = pastCap(total, cap);
    return h('span', { class: 'tag ' + (over > 0 ? 'tag-over' : 'tag-max'), style: style || null,
      'data-tip': over > 0 ? `Caps at ${fmt(cap, unit)}: the ${fmt(over, unit).replace(/^\+/, '')} past it adds nothing` : `Caps at ${fmt(cap, unit)}: more adds nothing`,
      text: over > 0 ? `${fmt(over, unit).replace(/^\+/, '')} over the cap` : 'At the cap' });
  }

  // ---- toggles (switch skills): only the ones switched on are counted; the page says which parts are theirs ----
  /** The small tag on a source that is a toggle skill switched on. */
  const toggleTag = (style) => h('span', { class: 'tag tag-off', style: style || null, text: 'Toggle · on',
    'data-tip': 'A switch skill, switched on. It is counted because it was on when the game was last read; switched off, it adds nothing.' });
  /** How much of a bonus row comes from toggles that are on (their parts are marked in the data). */
  const toggleShare = (row) => round2((row && Array.isArray(row.parts) ? row.parts : []).filter((p) => isTrue(p.toggle)).reduce((a, p) => a + n0(p.value), 0));
  /** When the toggles were last read ("game, 2026-11-24 14:00" in the data) as "Nov 24, 14:00"; null when unknown. `exact`: read from the game. */
  function togglesWhen() {
    const m = /^(game|saved), (\d{4}-\d{2}-\d{2}) (\d{2}:\d{2})$/.exec(String((S.meta && S.meta.togglesSource) || ''));
    return m ? { text: when(`${m[2]}T${m[3]}`), exact: m[1] === 'game' } : null;
  }
  /** The footnote: which toggle state the numbers reflect. */
  function togglesNote() {
    const w = togglesWhen();
    if (!w) return 'Toggles (switch skills): none has been read from the game yet, so none is counted as on. A snapshot reads them.';
    return `Toggles as they were on ${w.text}${w.exact ? '' : ' or earlier'}: only the toggles that were on then are counted. A snapshot reads them again.`;
  }
  /** "−700% of this is from toggles that are on", or null when no toggle is in it. */
  const toggleShareText = (row, what) => { const v = toggleShare(row); return v ? `${fmt(v, row.unit, true)} of ${what || 'this'} is from toggles that are on` : null; };

  const S = {
    data: null, meta: {}, v: null,
    settings: { character: 'Rubito', csv: '', theme: 'system', dataDir: '' },
    mode: Q.get('mode') || store.get('mode') || 'normal',
    page: Q.get('page') || (WEB ? 'skills' : 'overview'),   // always opens on Overview (the last page is not remembered); the website on the Skill Database
    busy: false, scanning: false, lastScan: 0,
    state: { active: null, scanned: false, clients: [], characters: [] },   // who is in game, whose data is shown
    compare: null,
    over: { open: null, demon: store.get('demon-open') === '1', allChanges: false, switching: null, delSnap: false, deletingSnap: false, aff: store.get('aff') || null },   // (aff: the affinity series shown on its own)   // Overview: the opened stat, demon details shown, every change listed, the demon being switched to
    demons: null,   // the COMP demons as last listed (the picker shows these at once, then looks at the game again)
    bonus: { q: '', group: 'all', open: new Set() },
    gear: { view: 'pieces', sel: null, cols: {}, colq: '', sort: null, short: Q.has('short') ? Q.get('short') === '1' : store.get('short') === '1' },
    dcm: { q: '', kind: 'all', open: new Set(), data: null, notes: {}, state: 'idle' },
    quest: { q: '', type: 'available', open: new Set(), data: null, state: 'idle', live: null, liveRaw: null, typeTouched: false },
    keys: { q: '', cat: 'all', held: 'all', open: new Set(), pg: 0, data: null, state: 'idle', busy: false },
    guides: { q: '', tag: 'all', open: new Set(), outOpen: new Set(), data: null, state: 'idle' },   // Dungeon Guides: filters, opened dungeons, opened outcomes
    gdb: { q: '', kind: 'all', cls: 'all', known: true, gender: '', align: '', src: [], range: '', aff: [], maxLv: '', sort: 'name', dir: 1, fopen: false, open: new Set(), pg: 0, data: null, state: 'idle' },
    // Fusion Planner: the engine once loaded, which view, the picked demons (forward), the demon wanted and the ones planned before it (reverse)
    dpl: { state: 'idle', eng: null, from: store.get('dpl-from') === 'scratch' ? 'scratch' : 'mine', sel: null, goals: (() => { try { const g = JSON.parse(store.get('dpl-goals') || '[]'); return Array.isArray(g) ? g : []; } catch (e) { return []; } })() },
    fus: { state: 'idle', eng: null, view: ['reverse', 'tree'].includes(store.get('fus-view')) ? 'tree' : 'forward', slots: [null, null, null], third: false,   // 'reverse' = the old "Find a recipe", now part of the tree
      // the Fusion tree: the demon wanted (undefined until the saved one is looked up), what it may start from, the demons the player has,
      // the way picked for a demon (by demon id), the clicked demon (its place in the tree) and the folded repeats opened
      tree: { target: undefined, pg: store.get('fus-tree-pg') !== '0', events: store.get('fus-tree-events') === '1', zoom: [0.75, 0.5].includes(Number(store.get('fus-tree-zoom'))) ? Number(store.get('fus-tree-zoom')) : 1, sel: null, over: new Map(), open: new Set(),
        wq: '', wmode: 'best', wst: { pg: 0 },   // the clicked demon's list of ways: the search, which list (best | all), its page
        have: (() => { try { const v = JSON.parse(store.get('fus-have') || '[]'); return new Set(Array.isArray(v) ? v.filter((x) => Number.isInteger(x)) : []); } catch (e) { return new Set(); } })() } },
    rbp: { state: 'idle', P: null, src: store.get('rbp-src') === 'comp' ? 'comp' : 'db', demonId: num(store.get('rbp-demon')), compKey: store.get('rbp-comp') || null, compSnap: (() => { try { const v = JSON.parse(store.get('rbp-snap') || 'null'); return v && typeof v === 'object' ? v : null; } catch (e) { return null; } })(), comp: null, compState: 'idle', plan: null, openSteps: false,
      view: 'home', id: null, name: '', builds: null, naming: false, confirmDel: false, saveState: '', navOpen: store.get('rbp-nav') !== '0' },   // view: the page of builds ('home') or the planner ('edit'), for the saved build `id` (null: the unsaved plan)   // Rebirth Planner: the plan is rbpPlan() (kept in storage)
    dun: { q: '', kind: 'all', open: new Set(), mob: new Set(), data: null, state: 'idle',   // Dungeon Database: opened places, opened enemies
      group: store.get('dun-group') !== '0', shown: new Set() },   // listed under the place you enter them from; the groups opened (all start folded)
    // Demon Rank: which demons, the level they are compared at ('base', '1' or '99'), the column sorted (an index of RANK_COLS) and its direction (1 = highest first)
    rank: { q: '', kind: 'own', race: '', at: 'base', mt: false, sort: 0, dir: 1 },
    ddb: { q: '', kind: 'contract', race: '', align: '', sort: 'level', open: new Set(), level: {}, mtView: new Set(), data: null, state: 'idle', only: null,   // only: a demon id a link opened
      variants: store.get('ddb-variants') !== '0' },   // list the variants (Lucky, Inexperienced ...) too; saved for next time
    // Skill Database: the action type shown ('' = all), who can get it, element, sort, opened skills, how many rows drawn
    skl: { q: '', type: '', who: 'get', el: '', sort: 'name', open: new Set(), pg: 0, data: null, state: 'idle', byDemon: null, byDemonFrom: null },
    // Build Planner: the character as read (init, base), the build being tried and its result (build, cur), what is open
    sim: { state: 'idle', error: '', init: null, base: null, cur: null, build: null, catalog: {}, tab: 'gear', open: null, expOpen: new Set(), allDiff: false, cap: null, seq: 0, timer: null, working: false, redraw: null,
      // saved builds: the list, which page is up ('home' = the builds, 'edit' = one build), the build in the editor
      // (id null = the unsaved one), the cards' results, and whether the list under "Build Planner" in the left bar is open
      view: 'home', id: null, name: '', builds: [], cards: new Map(), jobs: Promise.resolve(), who: null,
      // the base of the build in the editor: a saved character's name, or '' for none (then level and align count);
      // inits / bases: what the reader said about each base, asked once per reading (the promise, and the answer)
      char: '', level: 0, align: 'NEUTRAL', inits: new Map(), bases: new Map(), useSeq: 0,
      demon: '',   // the build's digitalize demon: one of its base's COMP demons ("type:name"), or '' = not digitalized
      saveState: '', storeTimer: null, naming: false, confirmDel: false, navOpen: store.get('sim-nav') !== '0' },
    exp: { q: '', closed: new Set(), view: store.get('exp-view') === 'plan' ? 'plan' : 'mine' },   // view: 'mine' (as read) or 'plan' (the planner)
    // Expertise page planner: whose plan, the character as read (init), the planned points and cap, its result (cur), the open rows
    xp: { who: '', state: 'idle', error: '', init: null, cur: null, points: null, cap: null, demon: '', open: new Set(), seq: 0, loadSeq: 0, timer: null, working: false, redraw: null },
    dg: { q: '', closed: new Set() },
    extra: { q: '', closed: new Set() },   // Bonuses: the quest bonus and compendium groups that are closed   // Digitalize: the search and the closed races
    hist: { keys: (store.get('hist') ?? 'lngr').split(',').filter(Boolean), slot: {}, asking: null, deleting: false, top: 0, focus: null },   // the stats ticked on the History graph (the old value was one key)
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
      // what the demon summoned beside you gives you (only a normal total: nobody is out while digitalized)
      const summoned = n0((bonusRowOf(k, label, unit) || {}).summoned);
      // what VIT and SPD take off cooldown, each mode with its own stats
      const statsRow = bonusRowOf(k, label, unit) || {}, stats = n0(statsRow.stats), statsDigi = n0(statsRow.stats_digi);
      by[k] = { stat: k, label, unit, gear, gearDigi, exp, demon, summoned, stats, statsDigi };
    });
    // XP lines: no snapshot column, they come from the Bonuses rows (gear, expertise / clan, demon as that page works them out)
    OTHER_STATS.forEach(([k, label, unit]) => {
      const row = bonusRowOf(k, label, unit);
      // always a line, 0% when the character has nothing for it (no Bonuses row then): the sheet reads the same for everyone
      if (!row) { by[k] = { stat: k, label, unit, gear: 0, gearDigi: 0, exp: 0, demon: 0, summoned: 0, stats: 0, statsDigi: 0 }; return; }
      if (k === 'xp_partner') { by[k] = { stat: k, label, unit, gear: 0, gearDigi: 0, exp: 0, demon: 0, summoned: 0, stats: 0, statsDigi: 0 }; return; }   // worked out in stat(): it follows the summoned demon
      const gear = n0(row.gear);
      by[k] = { stat: k, label, unit, gear, gearDigi: row.gear_digi === null || row.gear_digi === undefined ? gear : n0(row.gear_digi),
        exp: n0(row.expertise), demon: n0(row.demon), summoned: n0(row.summoned), stats: n0(row.stats), statsDigi: n0(row.stats_digi) };
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
      // a set's part names it by its shown name and says which worn set it is in "set_key" (older saved data: "(set N)" in the name)
      const key = (part.kind === 'set' || part.kind === 'fusion set') ? (part.set_key || setKey(part.source)) : (part.kind === 'expertise' ? null : 'item:' + part.source);
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
    let r = S.v.by[k];
    if (!r) return null;
    if (k === 'xp_partner') r = { ...r, exp: digi() ? 0 : partnerXp(bonusRowOf(k, r.label, r.unit)).value };
    const gear = digi() ? r.gearDigi : r.gear;
    const demonNow = digi() ? r.demon : 0, summonedNow = digi() ? 0 : r.summoned, statsNow = digi() ? r.statsDigi : r.stats;
    return { ...r, gearNow: gear, demonNow, summonedNow, statsNow, total: round2(gear + r.exp + demonNow + summonedNow + statsNow), extra: round2(r.gearDigi - r.gear) };
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
  /** A tooltip's description lines; when `ja` is set the text is BABEL's translation of it: greyed, with a note first. */
  const tlTipLines = (text, ja) => (ja ? [h('div', { class: 'tt-line tl-name tl-note', text: TL_DESC_NOTE }), h('div', { class: 'tl-name' }, ...tipLines(text))] : tipLines(text));
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
  // The Build Planner's saved builds hang under its entry (drawSimNav); the small arrow on the entry folds them away.
  const simNavBox = h('div', { class: 'nav-sub', hidden: true });
  const simNavToggle = h('button', { type: 'button', class: 'nav-fold', hidden: true, 'aria-label': 'Show or hide your saved builds',
    onclick: () => { S.sim.navOpen = !S.sim.navOpen; store.set('sim-nav', S.sim.navOpen ? '1' : '0'); drawSimNav(); } }, icon('chevron'));
  // the same for the Demon Planner's saved builds (drawRbpNav)
  const rbpNavBox = h('div', { class: 'nav-sub', hidden: true });
  const rbpNavToggle = h('button', { type: 'button', class: 'nav-fold', hidden: true, 'aria-label': 'Show or hide your saved demon builds',
    onclick: () => { S.rbp.navOpen = !S.rbp.navOpen; store.set('rbp-nav', S.rbp.navOpen ? '1' : '0'); drawRbpNav(); } }, icon('chevron'));
  // Related tools sit together under one fold-away entry ("Databases", "Planners"), open every time the app starts; the
  // entry folds them away for this session only (nothing is saved, so they are open again next launch). Inside a group
  // the repeated word is left off each name ("Skill Database" -> "Skills", "Fusion Planner" -> "Fusion"); the page's own
  // title keeps the full name. Tools in no group stay loose, below the groups.
  const NAV_GROUPS = [
    { id: 'dbs', title: 'Databases', ids: ['skills', 'ddb', 'geardb', 'quests', 'keys', 'dungeons'],
      short: (l) => (l.endsWith(' Database') ? l.slice(0, -' Database'.length) + 's' : l) },
    { id: 'planners', title: 'Planners', ids: ['fusion', 'dplan', 'rebirth', 'sim'], short: (l) => l.replace(/ Planner$/, '') },
  ];
  S.navOpen = {};
  const groupOf = (id) => NAV_GROUPS.find((g) => g.ids.includes(id));
  NAV_GROUPS.forEach((g) => {
    S.navOpen[g.id] = true;
    g.box = h('div', { class: 'nav-dbs', id: `nav-${g.id}` });
    g.head = h('button', { type: 'button', class: 'nav-item nav-dbhead', 'aria-expanded': 'true', 'aria-controls': `nav-${g.id}`, 'data-tip': `Show or hide the ${g.title.toLowerCase()}`,
      onclick: () => { S.navOpen[g.id] = !S.navOpen[g.id]; drawGroupNav(); } },
      h('span', { class: 'nav-dbchev' }, icon('chevron')), h('span', { class: 'nav-label', text: g.title }), h('span', { class: 'nav-key', text: String(g.ids.length) }));
  });
  function drawGroupNav() {
    NAV_GROUPS.forEach((g) => {
      g.box.hidden = !S.navOpen[g.id];
      g.head.setAttribute('aria-expanded', S.navOpen[g.id] ? 'true' : 'false');
      // folded away while you are on one of them: the entry says so, since the page's own button is out of sight
      g.head.classList.toggle('has-current', !S.navOpen[g.id] && g.ids.includes(S.page));
    });
  }
  const looseTools = [];
  TOOLS.forEach(([id, fullLabel, ic]) => {
    const g = groupOf(id), label = g ? g.short(fullLabel) : fullLabel;
    const btn = h('button', { type: 'button', class: 'nav-item', 'data-page': id, 'data-tip': label, onclick: () => (id === 'sim' ? simGoHome() : id === 'rebirth' ? rbpGoHome() : go(id)) },
      icon(ic), h('span', { class: 'nav-label', text: label }));
    const entry = id === 'sim' ? [h('div', { class: 'nav-group' }, btn, simNavToggle), simNavBox] : id === 'rebirth' ? [h('div', { class: 'nav-group' }, btn, rbpNavToggle), rbpNavBox] : [btn];
    if (g) g.box.append(...entry); else looseTools.push(...entry);
  });
  nav.append(...NAV_GROUPS.flatMap((g) => [g.head, g.box]), ...looseTools);
  setTimeout(() => drawRbpNav(), 0);   // (the saved demon builds, once the page's code is all set up)
  function go(page) {
    if (S.page === page) return;
    remember();
    if (S.page === 'ddb') S.ddb.only = null;   // a demon a link opened alone: the whole list again next time
    S.page = page;
    render(); $('#main').scrollTop = 0;
  }

  // ---------- back and forward ----------
  // Every page change (the left bar, Ctrl+1..6, a link to another page, a Build Planner build) leaves the page you were
  // on in a list, so Back (the top bar's arrow, Alt+Left, the mouse's back button) brings it back as you left it:
  // its search, filters, opened rows and where it was scrolled to. Forward undoes a Back until you go somewhere new.
  // What a page keeps of itself: the fields of its S part that say what is shown (not its loaded data, and not the
  // choices it also saves for next time, such as a grouping or a chart's series).
  const PAGE_KEEP = {
    overview: { over: ['open', 'allChanges'] },
    bonuses: { bonus: ['q', 'group', 'open'], extra: ['q', 'closed'] },
    gear: { gear: ['view', 'sel', 'colq'] },
    expertise: { exp: ['q', 'closed'] },
    digitalize: { dg: ['q', 'closed'] },
    skills: { skl: ['q', 'type', 'who', 'el', 'sort', 'open', 'pg'] },
    ddb: { ddb: ['q', 'kind', 'race', 'align', 'sort', 'open', 'mtView', 'only'] },
    geardb: { gdb: ['q', 'kind', 'cls', 'known', 'gender', 'align', 'src', 'range', 'aff', 'maxLv', 'sort', 'dir', 'fopen', 'open', 'pg'] },
    rank: { rank: ['q', 'kind', 'race', 'at', 'mt', 'sort', 'dir'] },
    dcm: { dcm: ['q', 'kind', 'open'] },
    rebirth: { rbp: ['openSteps'] },   // (which build is open is `rebirth` in here() and step())
    dungeons: { dun: ['q', 'kind', 'open', 'mob', 'shown'] },
    guides: { guides: ['q', 'tag', 'open', 'outOpen'] },
    quests: { quest: ['q', 'type', 'open'] },
  };
  const keepCopy = (v) => (v instanceof Set ? new Set(v) : v);
  const NAV_MAX = 50;
  const trail = { back: [], fwd: [], quiet: false };
  /** Where you are now, to come back to. */
  function here() {
    const kept = {};
    Object.entries(PAGE_KEEP[S.page] || {}).forEach(([part, keys]) => { kept[part] = {}; keys.forEach((k) => { kept[part][k] = keepCopy(S[part][k]); }); });
    return { page: S.page, kept, scroll: $('#main').scrollTop, sim: S.page === 'sim' ? { view: S.sim.view, id: S.sim.id } : null, rbp: S.page === 'rebirth' ? { view: S.rbp.view, id: S.rbp.id } : null };
  }
  function remember() {
    if (trail.quiet) return;
    trail.back.push(here());
    if (trail.back.length > NAV_MAX) trail.back.shift();
    trail.fwd = [];
  }
  const pageName = (id) => (PAGES.concat(TOOLS).find((p) => p[0] === id) || [])[1] || '';
  const stepName = (e) => (e.page === 'rebirth' && e.rbp && e.rbp.view === 'edit' ? (e.rbp.id ? ((rbpBuilds().find((b) => b.id === e.rbp.id) || {}).name || 'a demon build') : 'the unsaved demon build')
    : e.page === 'sim' && e.sim && e.sim.view === 'edit' ? (e.sim.id ? ((S.sim.builds || []).find((b) => b.id === e.sim.id) || {}).name || 'a build' : 'the unsaved build') : pageName(e.page));
  /** Back (dir -1) or forward (+1) one step. */
  function step(dir) {
    const from = dir < 0 ? trail.back : trail.fwd, to = dir < 0 ? trail.fwd : trail.back;
    if (!from.length) return;
    const e = from.pop();
    to.push(here());
    if (S.page === 'ddb') S.ddb.only = null;
    Object.entries(e.kept).forEach(([part, vals]) => Object.entries(vals).forEach(([k, v]) => { S[part][k] = keepCopy(v); }));
    const main = $('#main');
    const scroll = () => { main.scrollTop = e.scroll; };
    S.page = e.page;
    trail.quiet = true;   // (a Build Planner build opening here is this step, not a new one)
    try {
      if (e.sim && e.sim.view === 'edit' && (S.sim.view !== 'edit' || S.sim.id !== e.sim.id)) simOpen(e.sim.id);
      else if (e.sim && e.sim.view === 'home' && S.sim.view === 'edit') simGoHome();
      else if (e.rbp && e.rbp.view === 'edit' && (S.rbp.view !== 'edit' || S.rbp.id !== e.rbp.id)) rbpOpen(e.rbp.id);
      else if (e.rbp && e.rbp.view === 'home' && S.rbp.view === 'edit') rbpGoHome();
      else render();
    } finally { trail.quiet = false; }
    scroll();
    requestAnimationFrame(scroll);   // (again once a page that draws in two goes has its rows)
  }
  function drawNavButtons() {
    const b = $('#nav-back'), f = $('#nav-fwd');
    const lastB = trail.back[trail.back.length - 1], lastF = trail.fwd[trail.fwd.length - 1];
    b.disabled = !lastB; f.disabled = !lastF;
    b.dataset.tip = lastB ? `Back to ${stepName(lastB)} (Alt+Left)` : 'Back (Alt+Left)';
    f.dataset.tip = lastF ? `Forward to ${stepName(lastF)} (Alt+Right)` : 'Forward (Alt+Right)';
  }
  $('#nav-back').addEventListener('click', () => step(-1));
  $('#nav-fwd').addEventListener('click', () => step(1));
  // The mouse's side buttons. (WebView2 would otherwise go back in its own history: there is none, the page is one.)
  const sideButton = (ev) => ev.button === 3 || ev.button === 4;
  window.addEventListener('mousedown', (ev) => { if (sideButton(ev)) ev.preventDefault(); });
  window.addEventListener('mouseup', (ev) => {
    if (!sideButton(ev)) return;
    ev.preventDefault();
    if (!document.querySelector('dialog[open], [popover]:popover-open')) step(ev.button === 3 ? -1 : 1);
  });
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
    drawSimNav();
    drawRbpNav();
    drawGroupNav();
    drawNavButtons();
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
  /**
   * Where a window is in the game's menu, as its own line: "Menu > [pic] PC > [pic] Status Equipment", with the
   * game's own pictures of the bar's button and of the entry (host "menuIcons", cut from the client's files; just the
   * words for one it has no picture of, or while the game's folder isn't known).
   */
  let menuIcons = null;   // name (PC, DEVIL ..., Status Equipment ...) -> data: URI, once asked
  function menuPath(button, menu, item) {
    const pic = (key) => {
      const src = menuIcons && menuIcons[key];
      return h('img', { class: 'menu-btn', alt: '', 'data-menu': key, src: src || null, hidden: !src });
    };
    if (!menuIcons) {
      menuIcons = {};
      host.call('menuIcons').then((m) => {
        menuIcons = m && typeof m === 'object' ? m : {};
        document.querySelectorAll('img.menu-btn[data-menu]').forEach((el) => { const src = menuIcons[el.dataset.menu]; if (src) { el.src = src; el.hidden = false; } });
      }).catch(() => {});
    }
    return h('span', { class: 'menu-path' }, 'Menu > ', pic(button), `${menu} > `, pic(item), item);
  }
  const SNAP_ABOUT = 'A snapshot reads your stats, gear and expertise from the running game and works out every bonus. It only reads; nothing in the game is changed.';
  /** What to do in game before a snapshot, so it reads everything: the first-run page and the top bar's info button. */
  function snapshotSteps(name) {
    return h('ol', { class: 'steps' },
        h('li', null, h('span', { text: name ? `Start IMAGINE and log in as ${name}.` : 'Start IMAGINE and log in.' })),
        // The fusion window loads every COMP demon's details (tested in game); a click in the COMP only loads that one.
        h('li', null, h('span', null, 'At the ', h('b', { text: 'Cathedral of Shadows' }), ', start a ', h('b', { text: 'Double Fusion' }), ' with the Master, then cancel it.',
          h('span', { class: 'step-note', text: 'That loads every demon in your COMP, so the app can read your digitalize demon’s stats and skills. Nothing is fused.' }))),
        // The game only loads these when their window is opened (quest list, compendium count): without them the
        // quest and compendium bonuses are left out of the totals.
        h('li', null, h('span', null, 'Open the ', h('b', { text: 'Quest Completion Bonuses' }), ' and ', h('b', { text: 'Demonic Compendium' }), ' windows once.',
          menuPath('QUEST', 'Quest', 'View Quest Bonuses'),
          menuPath('DEVIL', 'Devil', 'Demonic Compendium'),
          h('span', { class: 'step-note', text: 'The game only loads your quest and compendium counts when you open them. Without them, those bonuses are left out.' }))),
        h('li', null, h('span', null, 'Open the ', h('b', { text: 'Information' }), ' window and leave it open.',
          menuPath('PC', 'PC', 'Status Equipment'),
          h('span', { class: 'step-note', text: 'It gives the exact stats, your tarot and soul enchants and your picture. If you are digitalized, the first snapshot needs it.' }))),
        h('li', null, h('span', { text: 'Press Take snapshot (or F5).' })));
  }
  function openSnapSteps() {
    const d = $('#snap-steps');
    $('#snap-steps-body').replaceChildren(h('p', { class: 'field-help', text: SNAP_ABOUT }), snapshotSteps(S.meta.character || S.state.active));
    if (!d.open) d.showModal();
  }
  function firstRun() {
    const name = S.meta.character || S.state.active;
    if (!name) return h('div', { class: 'page' }, h('div', { class: 'empty' },
      h('h2', { class: 'empty-title', text: 'No character yet' }),
      h('p', { class: 'page-sub', text: 'Start IMAGINE and log in. The app reads your character’s name from the game by itself; if two game clients are open it asks which one to look at.' }),
      h('button', { type: 'button', class: 'btn btn-primary', onclick: () => openChooser() }, 'Look for my character')));
    return h('div', { class: 'page' }, h('div', { class: 'empty' },
      h('h2', { class: 'empty-title', text: `No snapshot of ${name} yet` }),
      h('p', { class: 'page-sub', text: SNAP_ABOUT }),
      snapshotSteps(name),
      h('button', { type: 'button', class: 'btn btn-primary', onclick: takeSnapshot }, icon('camera'), 'Take snapshot')));
  }

  // ---------- Overview ----------
  /** Alignment (LAW / NEUTRAL / CHAOS) as a colour class and as a word. */
  const alignClass = (a) => (/^(LAW|NEUTRAL|CHAOS)$/i.test(String(a || '')) ? 'align-' + String(a).toLowerCase() : '');
  const alignName = (a) => (a ? String(a).charAt(0) + String(a).slice(1).toLowerCase() : '');

  /**
   * The game's mark for a Mitama demon on the top left corner of its picture, as in the COMP bar (the game's own
   * image, icons.json "mitama"; white there, tinted here as the game tints it). Nothing when the demon isn't one or
   * the image was never read from the client.
   */
  function mitamaMark(pic, mitama) {
    const src = mitama && S.data.icons && S.data.icons.mitama;
    if (src) pic.append(h('i', { class: 'mitama-mark', style: `--mark:url("${src}")` }));
    return pic;
  }

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
    mitamaMark(pic, DG.mitama);
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
    // the race's Digitalization Level passives by level ("DLv 2"), with the icons the "Unlocked Digitalization passives" list shows
    const passivesAt = new Map();
    (S.data.expertise || []).filter((r) => isDlevel(r) && r.skill && DG.race
      && r.expertise.replace(/^Digitalize: /, '').toLowerCase() === String(DG.race).toLowerCase()).forEach((r) => {
      const m = /(\d+)/.exec(String(r.rank || ''));
      if (m) passivesAt.set(+m[1], (passivesAt.get(+m[1]) || []).concat(r));
    });
    const assists = h('div', { class: 'demon-assists' },
      h('span', { class: 'stat-label', text: 'Digitalize boosts' }),
      isTrue(DG.assists_known)
        // every level, L1-L10, one per row; level 1 offers no assist, and a level with no pick shows a dash
        ? (() => {
          const byLevel = new Map(), iconAt = new Map();
          (DG.assists || []).forEach((a, i) => { const m = /^L(\d+) (.*)$/.exec(a); if (m) { byLevel.set(+m[1], m[2]); iconAt.set(+m[1], (DG.assist_icons || [])[i]); } });
          // the digitalize window's picture of the boost; its lock where none is learned yet (level 1 has no boost: nothing)
          const boostIcon = (lv, t) => { const icon = t ? iconAt.get(lv) : lv > 1 ? DG.assist_lock_icon : null; return skillIcon({ icon }); };
          const top = Math.max(10, ...byLevel.keys(), ...passivesAt.keys());
          return h('div', { class: 'dg-assist-list' }, Array.from({ length: top }, (_, i) => {
            const lv = i + 1, t = byLevel.get(lv), ps = passivesAt.get(lv) || [];
            const lines = [t ? `Level ${lv} assist: ${t}` : lv === 1 ? 'Level 1 has no assist' : `No assist picked at level ${lv} (or not reached yet)`,
              ...ps.map((r) => `${r.rank} passive: ${r.skill}${r.effects ? ' · ' + String(r.effects).split('; ').join(' · ') : ''}${isTrue(r.counted) ? '' : ' (not counted)'}`)];
            const row = h('div', { class: 'dg-assist' + (t ? '' : ' none') },
              h('b', { text: 'L' + lv }),
              boostIcon(lv, t),
              h('span', { text: t ? short(t) : '—' }),
              h('span', { class: 'dg-assist-icons' }, ps.map((r) => { const ic = skillIcon(r); if (!isTrue(r.counted)) ic.classList.add('off'); return ic; })));
            return withTip(row, () => h('div', null, lines.map((l) => h('div', { class: 'tt-line', text: l }))));
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
      // two columns: who it is, its stats, skills and features | its boosts, one level per row, down the whole card
      h('div', { class: 'demon-main' }, id, extra.length ? h('div', { class: 'demon-extra' }, extra) : null, bars, SK ? SK.parts : null),
      assists);
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
   * The radar's scale: from the deepest weakness (or 0) up to `high`, with rings at a clean step, 4 at most however wide
   * the range is. The outer ring is `high` rounded up to a clean number (half a step when the step is 100 or more, so
   * 208 gets 250 and not 300). The centre is the deepest weakness rounded down a little (not a whole step, which
   * would give a small weakness a quarter of the radar to itself). Returns the centre, the rim and the ring values.
   */
  function affinityScale(low, high) {
    let lo = 0, hi = 0, rings = [];
    for (const step of [10, 20, 25, 50, 100, 200, 250, 500, 1000, 2000, 5000]) {
      const fine = step >= 100 ? 25 : step >= 50 ? 10 : 5, round = step >= 100 ? step / 2 : step;
      lo = Math.min(0, Math.floor(low / fine) * fine);
      hi = Math.ceil(high / round) * round;
      rings = [];
      for (let v = (Math.floor(lo / step) + 1) * step; v < hi; v += step) rings.push(v);   // (the centre itself is not a ring)
      rings.push(hi);
      if (rings.length <= 4) break;
    }
    return { lo, hi, rings };
  }

  /**
   * Affinity, beside the stat sheet: one radar, one spoke per element, three things on the same percentage scale:
   * the boost to what you deal, the cap that boost stops at, and how much of the element you resist (below 0 is a
   * weakness). The buttons under it pick what is shown: all three, or just one of them (its numbers then appear on
   * the spokes, and the scale fits what is shown).
   */
  const AFF_SERIES = [['boost', 'Boost'], ['cap', 'Cap'], ['res', 'Resistance']];
  /**
   * The radar's numbers as a table under it, in the stat sheet's style (a heading, one line per element with a rule
   * between): Boost, Cap (100% plus what raises it) and Resistance. Same elements and order as the radar's spokes; the
   * line's tooltip gives each number's split into gear, expertise and (digitalized) the demon.
   */
  /** What an affinity number is made of, as [name, value] pairs: yours (gear, expertise, digitalize demon), or the partner's (P: from your gear, from the rest of you, its own). */
  const affParts = (e, P) => {
    const DG = S.data.digitalize;
    return (P ? [['your gear', e.gear], ['your expertise and the rest', e.exp], [P.own, e.demon]] : [['gear', e.gear], ['expertise', e.exp], ...(digi() && DG ? [[DG.demon, e.demon]] : [])]).filter(([, v]) => v);
  };
  /**
   * The summoned partner's affinities, in the shape of `affinity()`, for a demon user's Overview. Boost and cap: the
   * bonus list's "Partner" rows (what you give it). Resistance: the demon's own, from the Demon Database (its Mitama
   * form's when it is fused; a fresh demon's: no Demon Force, Mitama slots or skills), plus what you give it.
   * In an entry, `gear` is from your gear, `exp` from the rest of you, `demon` the demon's own.
   */
  function partnerAffinity() {
    const d = S.demons ? S.demons.find((x) => x.summoned) : S.data.summoned;
    const by = { boost: new Map(), resistance: new Map(), cap: new Map() };
    const at = (map, name) => { if (!map.has(name)) map.set(name, { name, gear: 0, exp: 0, demon: 0 }); return map.get(name); };
    (S.data.bonuses || []).forEach((r) => {
      if (r.group !== 'Partner' || !isTrue(r.counted)) return;
      const m = /^(.+?) (affinity cap|affinity|resist)$/i.exec(String(r.bonus || ''));
      if (!m) return;
      const k = m[2].toLowerCase(), e = at(by[k === 'affinity cap' ? 'cap' : k === 'resist' ? 'resistance' : 'boost'], m[1].charAt(0).toUpperCase() + m[1].slice(1));
      e.gear = round2(e.gear + n0(r.gear)); e.exp = round2(e.exp + n0(r.total) - n0(r.gear));
    });
    const r0 = d && S.ddb.data ? rbpResolve(d) : null, v = r0 ? ((r0.fused || n0(d.mitamaType) > 0) && r0.x.mt ? r0.x.mt : r0.x) : null;
    if (v && Array.isArray(v.res)) S.ddb.data.elements.forEach((name, i) => { if (n0(v.res[i])) at(by.resistance, name).demon = n0(v.res[i]); });
    const rank = (n) => { const i = AFF_ORDER.indexOf(n); return i < 0 ? AFF_ORDER.length : i; };
    const names = [...new Set([...by.boost.values(), ...by.resistance.values(), ...by.cap.values()].filter((e) => round2(e.gear + e.exp + e.demon)).map((e) => e.name))]
      .sort((a, b) => rank(a) - rank(b) || a.localeCompare(b));
    const list = (map) => names.map((name) => { const e = map.get(name) || { name, gear: 0, exp: 0, demon: 0 }; return { ...e, total: round2(e.gear + e.exp + e.demon) }; });
    return { aff: { boost: list(by.boost), resistance: list(by.resistance), cap: list(by.cap).map((e) => ({ ...e, raise: e.total, total: round2(100 + e.total) })) },
      demon: d || null, own: d ? `${d.name}’s own` : 'its own', ownKnown: !!v };
  }
  function affinityTable(aff, P) {
    const split = (e) => { const parts = affParts(e, P); return parts.length ? ` (${parts.map(([name, v]) => `${name} ${fmt(v, '%')}`).join(' + ')})` : ''; };
    const cell = (text, cls) => h('span', { class: 'aff-td' + (cls ? ' ' + cls : ''), text });
    const head = h('div', { class: 'aff-tr aff-th' }, h('span', { class: 'list-head', text: 'Affinity' }), h('span', { class: 'list-head', text: 'Boost' }), h('span', { class: 'list-head', text: 'Cap' }), h('span', { class: 'list-head', text: 'Resist' }));
    // Weapon-based is not an element: a skill whose base affinity is Weapon-Based (the basic attack, Cleave, Dragon Sweep...) takes
    // your weapon's element and adds this boost to that element's own. Its Bonuses row is in the "Other" group, so it is not a spoke.
    const wb = P ? null : weaponBoost();
    const wbRow = !wb ? null : h('div', { class: 'aff-tr aff-wb', tabindex: 0,
      'data-tip': `Weapon-based boost ${fmt(wb.total, '%')}${split(wb)}. Added to the boost of the element your weapon gives, for weapon-based skills only (basic attack, Cleave, Dragon Sweep...).`
        + ' In the server code it is capped on its own at 100% (plus any cap raise, and none raises this one), apart from the element’s cap. Not checked on NewMoon’s server.' },
      h('span', { class: 'aff-name', text: 'Weapon-based' }), cell(fmt(wb.total, '%')), cell('100%', 'zero'), cell('–', 'zero'));
    const rows = aff.boost.map((e, i) => {
      const cp = aff.cap[i], rs = aff.resistance[i];
      return h('div', { class: 'aff-tr', tabindex: 0,
        'data-tip': `${e.name}: boost ${fmt(e.total, '%')}${split(e)} · cap ${fmt(cp.total)}%${cp.raise ? ` (100% + ${fmt(cp.raise)}%)` : ''} · resistance ${fmt(rs.total, '%')}${split(rs)}` },
        h('span', { class: 'aff-name', text: e.name }),
        cell(e.total ? fmt(e.total, '%') : '–', e.total ? '' : 'zero'),
        cell(`${fmt(cp.total)}%`, cp.raise ? 'raised' : 'zero'),
        cell(rs.total ? fmt(rs.total, '%') : '–', rs.total < 0 ? 'bad' : rs.total ? '' : 'zero'));
    });
    return h('div', { class: 'aff-table', role: 'table', 'aria-label': 'Affinity by element' }, head, rows, wbRow);
  }
  /** The "Weapon based Affinity" bonus (BOOST_WEAPON) in the shown mode, split like the radar's series; null when you have none. */
  function weaponBoost() {
    const r = (S.data.bonuses || []).find((x) => x.group === 'Other' && /^weapon.based affinity$/i.test(String(x.bonus).trim()));
    if (!r) return null;
    const e = { name: 'Weapon-based', gear: round2(n0(digi() && r.gear_digi !== null && r.gear_digi !== undefined ? r.gear_digi : r.gear)), exp: n0(r.expertise), demon: digi() ? n0(r.demon) : 0 };
    e.total = round2(e.gear + e.exp + e.demon);
    return e.total ? e : null;
  }
  function affinityCard(P) {   // P: the partner's (partnerAffinity()), for a demon user's view; else yours
    const aff = P ? P.aff : affinity(), n = aff.boost.length;
    // (no heading of its own: it shares the row's, "Key stats and affinities", with the stat sheet beside it; the partner's has one, as the cards beside it)
    const card = h('section', { class: 'card', 'aria-label': P ? 'Your partner’s affinities' : 'Affinity' });
    // what the partner's numbers are, and what they leave out
    const about = !P ? null : h('p', { class: 'note', text: (P.ownKnown ? `Resistance is ${P.own} as a demon of its kind has it, plus what you give it. ` : P.demon ? `${P.own} resistances aren’t loaded yet, so only what you give it is shown. ` : 'No demon is summoned, so this is only what you give a partner. ')
      + 'Boost and cap are what you give it. Its Demon Force, Mitama slots and skills are not included.' });
    if (n < 3) {   // a radar needs three spokes: the table alone (or a line when there is nothing)
      card.append(n || (!P && weaponBoost()) ? affinityTable(aff, P) : h('p', { class: 'note', text: 'No affinity boosts or resistances yet.' }), about);
      return card;
    }
    const focus = AFF_SERIES.some(([k]) => k === S.over.aff) ? S.over.aff : null;
    const val = { boost: aff.boost.map((e) => e.total), cap: aff.cap.map((e) => e.total), res: aff.resistance.map((e) => e.total) };
    // what is shown: all three, or only the one picked
    const show = focus ? [focus] : AFF_SERIES.map(([k]) => k);
    // the scale fits what is shown. Next to the boost and resistance the cap does not set it (a cap of 800% would squeeze
    // every boost into the middle): the outer ring fits those, and at least the usual 100% cap, and a cap past it stops
    // at the rim, marked there with its real value. A cap shown on its own is what the scale fits.
    const fit = show.filter((k) => k !== 'cap' || show.length === 1);
    const { lo, hi, rings } = affinityScale(Math.min(0, ...fit.map((k) => Math.min(...val[k]))), Math.max(show.length > 1 ? 100 : focus === 'res' ? 10 : 50, ...fit.map((k) => Math.max(...val[k]))));
    const W = 346, H = 318, cx = W / 2, cy = H / 2, R = 94;   // sized for the card's column, with room for a two-line label on every spoke
    const svg = s('svg', { viewBox: `0 0 ${W} ${H}`, width: W, height: H, role: 'img',
      'aria-label': 'Affinity by element. ' + aff.boost.map((e, i) => `${e.name}: boost ${fmt(e.total, '%')}, cap ${fmt(val.cap[i])}%, resistance ${fmt(val.res[i], '%')}`).join('; ') });
    const ang = (i) => -Math.PI / 2 + (i * 2 * Math.PI) / n;
    const at = (i, v, r0) => { const r = r0 !== undefined ? r0 : R * Math.max(0, Math.min(1, (v - lo) / (hi - lo))); return [cx + r * Math.cos(ang(i)), cy + r * Math.sin(ang(i))]; };
    const path = (pts) => pts.map(([x, y], i) => `${i ? 'L' : 'M'}${x.toFixed(1)},${y.toFixed(1)}`).join('') + 'Z';
    const all = aff.boost.map((_, i) => i);
    // grid: the spokes, a few rings with their numbers between the first two spokes, the 0 ring a little stronger
    all.forEach((i) => { const [x, y] = at(i, hi); svg.append(s('line', { class: 'gridline', x1: cx, y1: cy, x2: x, y2: y })); });
    const ringLabels = [];   // (added after the series, so a filled area is not over a number)
    for (const v of rings) {
      svg.append(s('path', { class: v === 0 && lo < 0 ? 'zeroline' : 'gridline', d: path(all.map((i) => at(i, v))) }));
      const [x, y] = at(0.5, v);
      ringLabels.push(Object.assign(s('text', { class: 'ring-label', x: x + 3, y: y + 3 }), { textContent: fmt(v) + (v === hi ? '%' : '') }));
    }
    // the series shown (the ones not picked are left out); a value past the rim is drawn at the rim
    const draw = { boost: 'circle', res: 'square', cap: null };
    // (drawn resistance first, then the cap, the boost last: the boost lies over the resistances, the user's call)
    ['res', 'cap', 'boost'].filter((k) => show.includes(k)).forEach((k) => {
      const g = s('g', { class: `aff-s aff-${k}` });
      g.append(s('path', { class: 'aff-area', d: path(all.map((i) => at(i, val[k][i]))) }), s('path', { class: 'aff-line', d: path(all.map((i) => at(i, val[k][i]))) }));
      if (draw[k]) all.forEach((i) => {
        if (!val[k][i]) return;
        const [x, y] = at(i, val[k][i]), bad = k === 'res' && val[k][i] < 0 ? ' bad' : '';
        g.append(draw[k] === 'circle' ? s('circle', { class: 'aff-dot' + bad, cx: x, cy: y, r: 4 })
          : s('rect', { class: 'aff-dot' + bad, x: x - 3.5, y: y - 3.5, width: 7, height: 7, rx: 1 }));
      });
      svg.append(g);
    });
    svg.append(...ringLabels);
    // Labels go outside the rim, each centred on its spoke: the element, and under it the number of the series picked
    // (under "All", the real cap where it runs past the rim, with a small arrow on the rim). Near the top and bottom
    // neighbours sit side by side, so a label that would run into one already placed moves further out along its spoke. Each spoke's tooltip has all three numbers and where they come from.
    const split = (e) => { const parts = affParts(e, P); return parts.length > 1 ? ` (${parts.map(([name, v]) => `${name} ${fmt(v, '%')}`).join(' + ')})` : parts.length ? ` (${parts[0][0]})` : ''; };
    const placed = [], raised = aff.cap.filter((c) => c.raise);
    all.forEach((i) => {
      const e = aff.boost[i], rs = aff.resistance[i], cp = aff.cap[i], capText = fmt(cp.total) + '%', c = Math.cos(ang(i)), sn = Math.sin(ang(i));
      // the number of the series brought forward: nothing rather than "0%", and the cap without a plus sign
      const over = show.includes('cap') && cp.total > hi, quiet = !e.total && !rs.total && !cp.raise;
      const value = focus === 'cap' ? capText : focus && val[focus][i] ? fmt(val[focus][i], '%') : over ? 'cap ' + capText : '';
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
      // a cap past the rim: its line stops there, with a small arrow pointing outwards
      if (over) { const [bx, by] = at(i, 0, R + 1), [tx, ty] = at(i, 0, R + 7); svg.append(s('path', { class: 'aff-over', d: `M${(bx - 3.5 * sn).toFixed(1)},${(by + 3.5 * c).toFixed(1)}L${tx.toFixed(1)},${ty.toFixed(1)}L${(bx + 3.5 * sn).toFixed(1)},${(by - 3.5 * c).toFixed(1)}Z` })); }
      svg.append(s('g', { 'data-tip': `${e.name}: boost ${fmt(e.total, '%')}${split(e)} · cap ${capText}${cp.raise ? ` (100% + ${fmt(cp.raise)}%)` : ''} · resistance ${fmt(rs.total, '%')}${split(rs)}` },
        s('line', { x1: cx, y1: cy, x2: rx, y2: ry, stroke: 'transparent', 'stroke-width': 16 }),
        Object.assign(s('text', { class: 'aff-name' + (quiet ? ' none' : ''), x: lx, y: top, 'text-anchor': anchor }), { textContent: e.name }),
        value ? Object.assign(s('text', { class: 'aff-value' + (focus === 'res' && val.res[i] < 0 ? ' bad' : '') + (over && !focus ? ' over' : ''), x: lx, y: top + 12, 'text-anchor': anchor }), { textContent: value }) : null));
    });
    // the legend is the control: all three, or only one of them
    const pick = (k, label, mark) => h('button', { type: 'button', 'aria-pressed': String(focus === k), onclick: () => { S.over.aff = k; store.set('aff', k || ''); render(); } },
      mark ? h('i', { class: 'aff-key aff-key-' + k }) : null, label);
    card.append(h('div', { class: 'chart radar' }, svg),
      h('div', { class: 'seg aff-pick', role: 'group', 'aria-label': 'Show' }, pick(null, 'All'), AFF_SERIES.map(([k, label]) => pick(k, label, true))));    // a line about the series shown on its own; nothing under "All" (the user asked for that hint to go)
    if (focus) card.append(h('p', { class: 'note', text: focus === 'cap' ? 'Boosts stop at 100%' + (raised.length ? `, except where something of yours raises ${P ? 'your partner’s' : 'the'} cap: ${raised.map((c) => `${c.name} ${fmt(c.total)}%`).join(', ')}.` : '. Nothing of yours raises it for any element.')
      : focus === 'res' ? (P ? 'How much of each element your partner resists. Inside the 0 ring is a weakness.' : 'How much of each element you resist. Inside the 0 ring is a weakness.')
      : P ? 'What you add to the damage your partner deals with each element.' : 'What you add to the damage you deal with each element.' }));
    card.append(affinityTable(aff, P), about);   // the same numbers as lines, under the radar
    return card;
  }


  /**
   * The demon summoned when the game was last read (a snapshot, or "Load demons from the game"), to the right of your
   * stats: the picker's card without its bonus list. It comes with the page data, so it is drawn with the rest;
   * nothing is drawn when no demon was summoned.
   */
  function summonedCard() {
    // with the page data (dash\summoned.json), so it is there at once; the demon list, once loaded, is newer
    const d = S.demons ? S.demons.find((x) => x.summoned) : S.data.summoned;
    if (!d) return h('div', { class: 'summoned none' });
    const tag = h('span', { class: 'tag tag-demon', tabindex: 0, text: 'Summoned',
      'data-tip': 'Your summoned demon' + (d.seenAt ? ` when the game was last read (${when(d.seenAt)})` : '') + '. A snapshot, or "Load demons from the game" in the demon list, reads it again.' });
    return h('div', { class: 'summoned' }, demonTile(d, false, null, true, tag));
  }
  /** After the list of demons changed (loaded, or read from the game): only that card, so an open demon list stays put. */
  function refreshSummoned() {
    const el = $('.page-head .summoned'); if (el) el.replaceWith(summonedCard());
    const pc = $('.page-head .partner-card'); if (pc) pc.replaceWith(partnerCard());   // a demon user's view: the partner card instead
  }

  /**
   * What you wear, in one line under the demon's: every worn piece as its icon from the game in a slot-like tile, then
   * the complete sets. No names or numbers here (those are the Gear page's): hovering a tile shows the game's tooltip,
   * a click opens that piece on the Gear page. Null before any gear was read.
   */
  function gearBand() {
    const items = S.v.gear.filter((g) => g.kind !== 'set'), sets = S.v.gear.filter((g) => g.kind === 'set');
    if (!items.length) return null;
    const open = (g) => { S.gear.view = 'pieces'; if (g) S.gear.sel = g.row; go('gear'); };
    // a demon user's view marks each piece with who it boosts (a dot on its corner), counted beside the title
    const du = demonUser(), WHO = { partner: 'boosts your partner', you: 'boosts you', both: 'boosts you and your partner' };
    const count = (w) => S.v.gear.filter((g) => gearFor(g) === w).length;
    const tile = (g) => withTip(h('button', { type: 'button', class: 'gear-pc', onclick: () => open(g),
      'aria-label': (g.kind === 'set' ? `${setTitle(g)}, complete set` : `${g.slot}: ${g.item}`) + (du && WHO[gearFor(g)] ? ', ' + WHO[gearFor(g)] : '') },
      gearIcon(g), du && WHO[gearFor(g)] ? h('i', { class: 'who-dot who-' + gearFor(g) }) : null), gearTip(g));
    return h('section', { class: 'card band gear-band', 'aria-label': 'Gear' },
      du ? h('div', { class: 'band-head' }, h('h3', { class: 'band-title', text: 'Gear' }),
        legend(...[['who-partner', 'Partner', 'partner'], ['who-both', 'Both', 'both'], ['who-you', 'You', 'you']].filter(([, , w]) => count(w)).map(([cls, text, w]) => [cls, `${text} ${count(w)}`])))
        : h('h3', { class: 'band-title', text: 'Gear' }),
      // one run of tiles that wraps in a narrow window; the sets stay together (their label never ends a line alone)
      h('div', { class: 'band-items' }, items.map(tile),
        sets.length ? h('span', { class: 'gear-sets' }, h('span', { class: 'stat-label', text: sets.length === 1 ? 'Set' : 'Sets' }), sets.map(tile)) : null),
      h('button', { type: 'button', class: 'btn btn-sm', 'data-tip': 'Every piece and what it adds, on the Gear page', onclick: () => open(null) }, 'View more'));
  }

  // ---------- Overview for a demon user ----------
  // A demon user's gear mostly boosts the partner, so their Overview is about the partner. Which view a character
  // gets is the player's pick (a switch beside the name), remembered per character.
  const duKey = () => 'du-' + (S.meta.character || '');
  const demonUser = () => store.get(duKey()) === '1';
  const viewSwitch = () => {
    const pick = (on, label, tip) => h('button', { type: 'button', 'aria-pressed': String(demonUser() === on), 'data-tip': tip,
      onclick: () => {
        if (demonUser() === on) return;
        const flip = () => { store.set(duKey(), on ? '1' : '0'); S.over.open = null; render(); };
        // The browser's view transition: the picture, your stats and the demon's card each glide and resize to
        // their place in the other view, the rest cross-fades (app.css, "vt-view"). Skipped where it isn't
        // supported, or when the system asks for less motion.
        const root = document.documentElement;
        if (!document.startViewTransition || matchMedia('(prefers-reduced-motion: reduce)').matches || root.classList.contains('vt-view')) { flip(); return; }
        root.classList.add('vt-view');   // the parts carry their transition names only while this is on
        const done = () => root.classList.remove('vt-view');
        try { document.startViewTransition(flip).finished.then(done, done); } catch (e) { done(); flip(); }
      } }, label);
    return h('div', { class: 'seg', role: 'group', 'aria-label': 'Overview for' },
      pick(false, 'Player', 'Your own key stats and affinities'), pick(true, 'Demon user', 'What your partner demon gets from you, and what you keep for yourself'));
  };
  /** Your own stats a demon user's head still shows (the rest are in the Player view). */
  const DU_STATS = ['HP', 'MP', 'VIT'];
  /** Bonus parts that come from what you wear. */
  const GEAR_KINDS = new Set(['item', 'mod', 'tarot', 'soul', 'appearance', 'set', 'fusion set']);
  /** Who a worn piece boosts: 'partner', 'you', 'both', or 'none' (nothing counted, or only in some fights). */
  const gearFor = (g) => {
    let p = 0, y = 0;
    (g.parts || []).forEach(({ row }) => { if (row.group === 'Partner') p++; else if (row.group !== 'Only in some fights') y++; });
    return p && y ? 'both' : p ? 'partner' : y ? 'you' : 'none';
  };
  /**
   * What the partner gets from you: the "Partner" rows of the bonus list (gear, expertise, digitalize levels,
   * compendium), in the Key stats groups, then its stats, its affinities and the rest. Only lines something adds to.
   * No cap bars: the partner's caps aren't known here. A click opens where a line comes from.
   */
  function partnerSheet() {
    const rows = (S.data.bonuses || []).filter((r) => r.group === 'Partner' && r.counted !== false);
    const byName = new Map(rows.map((r) => [String(r.bonus).trim().toLowerCase(), r]));
    const used = new Set();
    const take = (...names) => { for (const n of names) { const r = n && byName.get(String(n).toLowerCase()); if (r && !used.has(r)) { used.add(r); return r; } } return null; };
    const namesOf = (k) => { const c = COMBAT.find((x) => x[0] === k), o = OTHER_STATS.find((x) => x[0] === k); return [BONUS_NAME[k], c && c[1], o && o[1], k === 'dmg_support' ? 'Support Damage' : null]; };
    // The player's sheet, line for line (the user's layout): every Key stats line, 0 where nothing of yours adds to the
    // partner's. Then what the player's sheet has no line for: what you add to its stats, and the rest.
    const unitOf = (k) => { const c = COMBAT.find((x) => x[0] === k), o = OTHER_STATS.find((x) => x[0] === k); return c ? c[2] : o ? o[2] : ''; };
    const groups = new Map(SHEET.flat().map(([title, stats]) => [title, stats.filter(([k]) => k !== 'xp_partner').map(([k, label]) => [take(...namesOf(k)), label, unitOf(k), k])]));
    groups.set('Added to its stats', ['Max HP', 'Max MP', ...STATS.map(([, label]) => label)].map((n) => [take(n), n]).filter(([r]) => r));
    rows.forEach((r) => { if (/ (affinity cap|affinity|resist)$/i.test(String(r.bonus))) used.add(r); });   // the affinity card beside the sheet has these
    groups.set('Other', rows.filter((r) => !used.has(r)).map((r) => [r, r.bonus]));
    // a line with no sources can't be opened (the user's call): no chevron, not a button
    const plain = (label, text, bad) => h('div', { class: 'sheet-row static', tabindex: 0, 'data-tip': 'Nothing of yours adds to this for your partner' },
      h('span'), h('span', { class: 'sheet-name' }, h('span', { class: 'sheet-label', text: label })), h('span'), h('span', { class: 'sheet-value' + (bad ? ' bad' : ''), text }));
    const none = (label, unit) => plain(label, fmt(0, unit));
    const line = ([r, label, unit, k]) => {
      if (!r) return none(label, unit);
      const id = 'p:' + r.row, open = S.over.open === id, f = (v) => fmt(v, r.unit, true);
      const src = new Map();   // per kind of source, without what only counts while digitalized (no partner is out then)
      (Array.isArray(r.parts) ? r.parts : []).filter((p) => !isTrue(p.digi) && n0(p.value)).forEach((p) => {
        const gear = GEAR_KINDS.has(p.kind), name = gear ? 'Gear' : KIND[p.kind] || 'Other';
        const e = src.get(name) || { cls: gear ? 'sw-gear' : p.kind === 'expertise' || p.kind === 'clan' ? 'sw-exp' : p.kind === 'demon' || p.kind === 'summoned' ? 'sw-demon' : 'sw-base', v: 0, from: [] };
        e.v = round2(e.v + n0(p.value)); e.from.push(p.source); src.set(name, e);
      });
      if (!src.size) return plain(label, f(r.total), n0(r.total) < 0 && !lowIsGood(r.bonus));
      const row = h('button', { type: 'button', class: 'sheet-row', 'aria-expanded': String(open), 'data-tip': [...src].map(([name, e]) => `${name} ${f(e.v)}`).join(' · '),
        onclick: () => { S.over.open = open ? null : id; render(); } },
        h('span', { class: 'chev' }, icon('chevron')), h('span', { class: 'sheet-name' }, h('span', { class: 'sheet-label', text: label })), h('span'),
        h('span', { class: 'sheet-value' + (n0(r.total) < 0 && !lowIsGood(r.bonus) ? ' bad' : ''), text: f(r.total) }));
      if (!open) return row;
      return [row, h('div', { class: 'sheet-detail' },
        [...src].map(([name, e]) => h('div', { class: 'sheet-src' }, h('i', { class: 'swatch ' + e.cls }), h('span', null, name, h('small', { text: [...new Set(e.from)].join(', ') })), h('b', { text: f(e.v) }))),
        h('button', { type: 'button', class: 'btn btn-sm', 'data-tip': 'Every source of this, on the Bonuses page', onclick: () => { S.bonus.q = r.bonus; S.bonus.group = 'all'; S.bonus.open.add(r.row); go('bonuses'); } }, 'View more'))];
    };
    const col = (titles) => h('div', { class: 'sheet-col' }, titles.filter((t) => (groups.get(t) || []).length).map((t) => h('div', { class: 'sheet-group' }, h('h3', { class: 'list-head', text: t }), groups.get(t).map(line))));
    // the player's two columns, each with one of the extra groups under it
    return h('section', { class: 'card sheet', 'aria-label': 'Your partner’s key stats' },
      h('div', { class: 'sheet-cols' }, SHEET.map((c, i) => col([...c.map(([title]) => title), i === 0 ? 'Other' : 'Added to its stats']))));
  }
  /**
   * A demon user's main card, under the head: the summoned partner (who it is and its skills, as the summoned card),
   * its own numbers as its status window showed them when it was last read (Max HP / MP and the 12 stats, bars scaled
   * within their group as yours), and what it gives you while it is out (the "summoned" parts of the bonus list).
   */
  function partnerCard() {
    const d = S.demons ? S.demons.find((x) => x.summoned) : S.data.summoned;
    if (!d) return h('section', { class: 'card partner-card none', 'aria-label': 'Your partner' }, h('h3', { class: 'band-title', text: 'Your partner' }),
      h('p', { class: 'note', text: 'No demon was summoned when the game was last read. Summon your partner and take a snapshot, or load your demons from the game in the demon list.' }));
    const tag = h('span', { class: 'tag tag-demon', tabindex: 0, text: 'Summoned', 'data-tip': 'Your summoned demon' + (d.seenAt ? ` when the game was last read (${when(d.seenAt)})` : '') + '. A snapshot reads it again.' });
    // Who it is, in five rows as tall as the five rows of its stats: the title, the name, level and race, alignment,
    // Mitama (each on a row of its own: the user's layout). The picture, framed in its alignment, spans them.
    const A = d.offer && d.offer.alignment;
    const pic = h('span', { class: 'demon-pic ' + alignClass(A), 'aria-hidden': 'true' }, h('span', { text: String(d.name || '?').charAt(0) }));
    if (d.icon) pic.append(h('img', { src: d.icon, alt: '' }));
    mitamaMark(pic, d.mitama);
    const who = h('div', { class: 'partner-who' }, pic,
      h('h3', { class: 'list-head', text: 'Your partner' }),
      h('span', { class: 'dgt-name-line' }, h('span', { class: 'dgt-name', text: d.name }), tag),
      h('span', { class: 'dgt-line', text: [`Lv ${d.level}`, d.race].filter(Boolean).join(' · ') }),
      h('span', { class: 'dgt-line', text: A ? alignName(A) : 'Alignment not read' }),
      h('span', { class: 'dgt-line', text: d.mitama ? `${d.mitama} Mitama · rank ${n0(d.mitamaRank)}` : 'No Mitama',   // as the planner's tag words it
        'data-tip': d.mitama ? 'M-Type: the Mitama it was fused with, and its Mitama rank (what the fusion keeps of the rebirth ranks before it)' : null }));
    let bars = h('p', { class: 'note', text: 'Its stats aren’t saved yet: take a snapshot with it summoned.' });
    if (Array.isArray(d.stats) && d.stats.length === 14) {
      const rows = [['HP', 'hpmp'], ['MP', 'hpmp'], ...STATS.map(([, label], i) => [label, i < 6 ? 'base' : i < 10 ? 'atk' : 'def'])].map(([label, group], i) => ({ label, group, v: Math.max(0, n0(d.stats[i])) }));
      const gmax = {};
      rows.forEach((e) => { gmax[e.group] = Math.max(gmax[e.group] || 1, e.v); });
      bars = h('div', { class: 'dg-bars who-stats' }, rows.map((e) => h('div', { class: 'srow dg-row', tabindex: 0, 'data-tip': `${d.name}’s ${e.group === 'hpmp' ? 'Max ' : ''}${e.label} ${fmt(e.v)}, as its status window showed it` },
        h('span', { class: 'srow-label', text: e.label === 'Support' ? 'Supp' : e.label }),
        h('div', { class: 'strack' }, e.v > 0 ? h('i', { class: 'demon', style: `width:${(e.v / gmax[e.group]) * 100}%` }) : null),
        h('span', { class: 'srow-value', text: fmt(e.v) }))));
    }
    // Its skill bar, Demon Force, demon equipment and rebirth ranks, as a Demon Planner build card draws them
    // (rbpMiniBars on the demon as last read, with an empty plan: nothing planned, so the bars are what it has).
    // The planner's data loads on first use; the card is drawn again when it is there.
    let build = null;
    if (d.loaded !== false) {
      // (the page's demon says only whether it wears equipment; the worn parts are in equipmentIds)
      build = rbpMiniBars({ src: 'comp', comp: { ...d, equipment: Array.isArray(d.equipmentIds) ? d.equipmentIds : null }, demonId: null, plan: null }, true);
      if (!build && !partnerCard.waiting) {
        partnerCard.waiting = true;
        loadRebirth();
        let tries = 0;
        const again = () => {
          const busy = [S.rbp.state, S.ddb.state, S.skl.state].some((x) => x === 'idle' || x === 'loading');
          if (busy && ++tries < 100) { setTimeout(again, 200); return; }
          partnerCard.waiting = false;
          if (S.page === 'overview' && S.rbp.state === 'ready' && S.ddb.state === 'ready') { if (demonUser()) render(); else refreshSummoned(); }   // (the whole page: the affinity card reads the Demon Database too)
        };
        setTimeout(again, 200);
      }
    }
    return h('section', { class: 'card partner-card ' + alignClass(A), 'aria-label': 'Your partner' },
      h('div', { class: 'partner-top' }, who, bars),
      build ? h('div', { class: 'partner-build' }, build) : null);
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
    // a row of its own above everything: the Player / Demon user switch on the left (over your picture), Export build
    // on the right (over the demon's picture)
    page.append(h('div', { class: 'over-top' }, viewSwitch(),
      S.meta.character ? h('button', { type: 'button', class: 'btn btn-primary btn-sm', 'data-tip': 'A code of what you wear and your expertise ranks, for another player to import into their Build Planner',
        onclick: (ev) => exportCurrentBuild(ev.currentTarget) }, icon('share'), 'Export build') : null));
    // A demon user's head: the partner card sits beside you and takes most of the row; of your own stats only the
    // ones that matter then are kept (HP, MP and VIT: the user's list).
    const du = demonUser();
    page.append(h('div', { class: 'page-head with-portrait' + (du ? ' du' : '') }, portrait, h('div', { class: 'page-head-text' },
      h('div', { class: 'who-line' },
        h('p', { class: 'page-who' }, h('b', { class: 'who-big', text: S.meta.character || '' }), [`Level ${fmt(L.level)}`, align].filter(Boolean).map((t) => ` · ${t}`).join('')),
        h('div', { class: 'who-right' },
          legend(['sw-base', 'Base'], ['sw-bonus', 'Bonus'], ...(digi() && DG ? [['sw-demon', DG.demon]] : [])))),
      h('div', { class: 'dg-bars who-stats' }, mine.filter((e) => !du || DU_STATS.includes(e.label)).map(statRow))), du ? partnerCard() : summonedCard()));
    // three one-line bands under your stats: your expertise, your demon (always there: a demon, or "No demon"), your gear
    page.append(expertiseBand(), demonCard());
    const worn = gearBand();
    if (worn) page.append(worn);

    const missing = String(L.gear_missing || '').trim();
    if (missing) page.append(h('div', { class: 'notice notice-warn' }, icon('alert'), h('div', null,
      h('b', { text: `No saved bonuses for ${missing}. ` }), 'They are not in the totals yet. Open the Information window in game once, then take a snapshot.')));
    else if (!isTrue(L.stats_live)) page.append(h('div', { class: 'notice notice-warn' }, icon('alert'), h('div', null,
      h('b', { text: `The 12 stats and HP/MP are from ${when(L.stats_from)}. ` }), isTrue(L.stats_digitalized)
        ? 'Gear and expertise are current. The stats are kept from then because you were digitalized (the game then shows the demon’s share too). Take a snapshot when not digitalized to refresh them.'
        : 'Gear and expertise are current. The 12 stats couldn’t be read from the game at the last snapshot, so the saved ones are shown. Take another snapshot to refresh them.')));

    // Key stats: one line each (name, how close to its cap when it has one, total). Where the total comes from is a
    // click away, under the line; every single source is on the Bonuses page.
    const sheetRow = ([k, label]) => {
      const r = stat(k);
      if (!r) return null;
      const f = (v) => fmt(v, r.unit, true);
      const cap = CAPS[k] === undefined ? null : CAPS[k];
      const over = cap !== null && pastCap(r.total, cap) > 0 ? pastCap(r.total, cap) : 0;
      const plain = (v) => fmt(v, r.unit).replace(/^\+/, '');
      const bonusRow0 = bonusRowOf(k, r.label, r.unit);
      const px = k === 'xp_partner' ? partnerXp(bonusRow0) : null;
      // the partner line: which demon it follows and why it may be nothing (the Sympathy skills each cover one race)
      const pxNote = !px ? null : digi() ? 'Nobody is out while digitalized, so this is 0 here.'
        : !px.demon ? 'No demon is summoned (or none was read yet), so this is 0. Summon one and take a snapshot, or load your demons from the game.'
        : px.value ? `${px.demon.name} is ${px.demon.race}: the Sympathy skill for that race adds this to its XP.`
        : `${px.demon.name} is ${px.demon.race}, and no Sympathy skill you have covers that race (${px.races.length ? 'they cover ' + px.races.join(', ') : 'you have no Sympathy skill yet'}).`;
      const srcs = [['sw-gear', 'Gear', r.gearNow], ['sw-exp', px ? 'Demonology Sympathy' : expShareName(bonusRow0), r.exp], ...(digi() && DG ? [['sw-demon', DG.demon, r.demonNow]] : []),
        ...(!digi() ? [['sw-demon', summonedName(), r.summonedNow]] : []), ['sw-base', 'Your stats', r.statsNow]].filter(([, , v]) => v);
      const open = S.over.open === k;
      // a bar only where there is a cap, so every bar on the sheet means the same thing: how close to the cap
      // (full and soft red when past it, with an "Over cap" tag beside the name; by how much is in the tooltip and under the opened line)
      const meter = cap === null ? h('span') : h('span', { class: 'cap-bar' + (over ? ' over' : '') }, h('i', { style: `width:${Math.max(0, Math.min(100, (r.total / cap) * 100))}%` }));
      const canOpen = srcs.length > 0;   // a line with no sources can't be opened (the user's call; Partner EXP earned too: why it is 0 is in its tooltip)
      const row = h(canOpen ? 'button' : 'div', { type: canOpen ? 'button' : null, class: 'sheet-row' + (canOpen ? '' : ' static'), 'aria-expanded': canOpen ? String(open) : null, tabindex: canOpen ? null : 0,
        'data-tip': (srcs.length ? srcs.map(([, name, v]) => `${name} ${f(v)}`).join(' · ') : pxNote || 'Nothing adds to this yet')
          + (cap === null ? '' : `. Caps at ${fmt(cap, r.unit)}` + (over ? `: ${plain(over)} over, which adds nothing.` : '.')),
        onclick: canOpen ? () => { S.over.open = open ? null : k; render(); } : null },
        canOpen ? h('span', { class: 'chev' }, icon('chevron')) : h('span'),
        // past the cap: a short "Over cap" tag beside the name (by how much is in the tooltip and under the opened line)
        h('span', { class: 'sheet-name' }, h('span', { class: 'sheet-label', text: label }), over ? h('span', { class: 'tag tag-over', text: 'Over cap' }) : null), meter,
        h('span', { class: 'sheet-value' + (r.total < 0 && !lowIsGood(r.label) ? ' bad' : ''), text: fmt(r.total, r.unit) }));
      if (!open || !canOpen) return row;
      const bonusRow = bonusRow0;
      const line = (cls, name, value, note) => h('div', { class: 'sheet-src' }, h('i', { class: 'swatch ' + cls }),
        h('span', null, name, note ? h('small', { text: note }) : null), h('b', { text: value }));
      return [row, h('div', { class: 'sheet-detail' },
        srcs.length ? srcs.map(([cls, name, v]) => line(cls, name, f(v),
          cls === 'sw-gear' && r.extra ? (digi() ? `${f(r.extra)} of it only while digitalized` : `${f(r.extra)} more while digitalized`)
            : cls === 'sw-exp' ? toggleShareText(bonusRow0, 'it')
            : name === 'Your stats' ? statsNote(bonusRow0) : null))
          : px ? null : h('p', { class: 'note', text: 'Nothing adds to this yet.' }),
        pxNote ? h('p', { class: 'note', text: pxNote }) : null,
        toggleShare(bonusRow0) ? h('p', { class: 'note', text: togglesNote() }) : null,
        cap === null ? null : h('p', { class: 'note', text: `Caps at ${fmt(cap, r.unit)}: ` + (over ? `the ${plain(over)} above it adds nothing, so those points could go elsewhere.`
          : pastCap(r.total, cap) >= 0 ? 'reached, more adds nothing.' : `${plain(Math.abs(pastCap(r.total, cap)))} to go.`) }),
        bonusRow ? h('button', { type: 'button', class: 'btn btn-sm', 'data-tip': 'Every source of this, on the Bonuses page', onclick: () => { S.bonus.q = bonusRow.bonus; S.bonus.group = 'all'; S.bonus.open.add(bonusRow.row); go('bonuses'); } },
          'View more') : null)];
    };
    const sheetGroup = ([title, stats]) => h('div', { class: 'sheet-group' }, h('h3', { class: 'list-head', text: title }), stats.map(sheetRow));

    // Demon user: the Player view's row, with only the partner's numbers (the user's call: your own are a click away,
    // in the Player view). Left, the Key stats sheet as what the partner gets from you (the Bonuses page's "Partner"
    // rows); right, the affinity card as the partner's. No "What changed": that compares your own snapshots.
    if (demonUser()) {
      const d = S.demons ? S.demons.find((x) => x.summoned) : S.data.summoned;
      page.append(sect('Key stats and affinities', digi() ? 'Your partner’s. Nobody is out while digitalized: this is what a summoned partner gets' : d ? `${d.name}’s, from you` : 'Your partner’s. No demon was summoned when the game was last read', null,
        h('div', { class: 'over-grid' }, h('div', { class: 'over-left' }, partnerSheet()), affinityCard(partnerAffinity()))));
      page.append(h('p', { class: 'note', text: 'Not included: your partner’s own bonuses from its skills, buffs, and bonuses that only count in some fights (those are on the Bonuses page). Your own key stats and affinities are in the Player view.' }));
      return page;
    }    const sheet = h('section', { class: 'card sheet', 'aria-label': 'Key stats' }, h('div', { class: 'sheet-cols' }, SHEET.map((col) => h('div', { class: 'sheet-col' }, col.map(sheetGroup)))));

    // One row under one heading: on the left the stat sheet with What changed (its own heading and snapshot picker)
    // under it, on the right affinity, whose radar and table are about as tall as those two together. Affinity carries
    // no heading of its own (one used to push its box below the sheet's: the user's "misalignment"). The sheet and
    // affinity follow the shown mode, so the heading says which.
    page.append(sect('Key stats and affinities', digi() && DG ? `While digitalized with ${DG.demon}` : 'Not digitalized', null,
      h('div', { class: 'over-grid' }, h('div', { class: 'over-left' }, sheet, changesCard()), affinityCard())));

    page.append(h('p', { class: 'note', text: 'Not included anywhere: buffs, clan and title bonuses, and your partner demon\u2019s own stats. The game only works those out on the server. Pick your digitalize demon above to add what it gives.' }));
    return page;
  }

  /**
   * Expertise on Overview, one compact line above the demon's (as the gear line): the points spent against the cap (as
   * the game's Expertise window shows them: points / 100, "pt") under the title, then each chain expertise the
   * character has unlocked as a chip, its name and class-rank. The skills its reached ranks give are in the chip's
   * tooltip (passive, then active; a switched-off toggle is dimmed) and a click opens the Expertise page on that chain.
   * No stats here: what they add is on the Expertise and Bonuses pages.
   */
  function expertiseBand() {
    const R = S.data.ranks || {}, P = R.points || null;
    const pt = (v) => (n0(v) / 100).toFixed(2) + 'pt';
    // points / cap over a thin bar; what is free and how the cap is made up are in its tooltip
    let points = null;
    if (P && P.cap) {
      const free = n0(P.cap) - n0(P.total);
      points = h('div', { class: 'band-points', tabindex: 0, 'data-tip': (free > 0 ? `${pt(free)} free. ` : 'At the cap. ')
          + `Cap ${pt(P.cap)} = 17000.00pt + 1000.00pt per 10 levels (level ${P.level})` + (P.level === 99 ? ' + 1000.00pt at 99' : '') + ` + ${P.extension} expansions × 1000.00pt` },
        h('span', { class: 'band-pts', text: `${pt(P.total)} / ${pt(P.cap)}` }),
        h('span', { class: 'band-bar' }, h('i', { style: `width:${Math.max(0, Math.min(100, (n0(P.total) / n0(P.cap)) * 100))}%` })));
    } else if (P) points = h('div', { class: 'band-points' }, h('span', { class: 'band-pts', text: `${pt(P.total)}, no cap` }));
    // Two parts (the user's layout). Left: the title with the points, the bar, and under them the unlocked chains as a
    // list of name and class-rank, in two columns when the window has room. Right: every skill those ranks give as one
    // run of icons, chain after chain (passives then actives), with no names between them: an icon's tooltip says
    // which skill and which chain. Skills come from the ranks file ("skills": every skill of the reached ranks,
    // actives too); a passive's row in expertise.json says whether a toggle is switched off. Older data without
    // "skills" lists the passives from expertise.json only.
    const chains = ((R.ranks) || []).filter((r) => r.chain);
    const rank = (c) => h('button', { type: 'button', class: 'exp-rank', 'data-tip': `Open ${c.name} on the Expertise page`, 'aria-label': `${c.name}, class ${c.class} rank ${c.rank}`,
      onclick: () => { S.exp.q = c.name; S.exp.view = 'mine'; go('expertise'); } }, h('span', { text: c.name }), h('b', { text: `${c.class}-${c.rank}` }));
    const icons = (c) => {
      const rows = S.data.expertise.filter((r) => r.expertise === c.name && !isExtra(r));
      const offIds = new Set(rows.filter((r) => r.toggle && !r.toggle_on).map((r) => r.skill_id));
      const all = Array.isArray(c.skills) ? c.skills.map((s) => ({ ...s, off: offIds.has(s.id) }))
        : rows.map((r) => ({ id: r.skill_id, name: r.skill || '—', icon: r.icon, rank: r.rank || '', active: false, off: r.toggle && !r.toggle_on }));
      return all.filter((s) => !s.active).concat(all.filter((s) => s.active)).map((s, i) => {
        const ic = skillIcon(s);
        ic.dataset.tip = `${s.name} · ${c.name}, ${s.active ? 'active' : 'passive'} (${s.rank})` + (s.off ? ', toggle switched off' : '');
        if (s.off) ic.classList.add('off');
        if (i === 0) ic.classList.add('first');   // a little air before each chain's icons
        return ic;
      });
    };
    return h('section', { class: 'card band exp-band', 'aria-label': 'Expertise' },
      h('div', { class: 'band-head' },
        h('div', { class: 'exp-top' }, h('h3', { class: 'band-title', text: 'Expertise' }), points),
        P ? null : h('span', { class: 'note', text: 'Take a snapshot to read your expertise points and cap.' }),
        chains.length ? h('div', { class: 'exp-ranks', style: `--rows:${Math.ceil(chains.length / 2)}` }, chains.map(rank)) : h('span', { class: 'note', text: 'No chain expertise unlocked yet.' })),
      h('div', { class: 'band-items exp-icons-run' }, chains.map(icons)),
      h('button', { type: 'button', class: 'btn btn-sm', 'data-tip': 'Every expertise and what it adds, on the Expertise page', onclick: () => { S.exp.q = ''; S.exp.view = 'mine'; go('expertise'); } }, 'View more'));
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
    const sfx = digi() ? '_digi' : '';
    const groups = [
      ['Character', [['level', 'Level', ''], ['max_hp', 'Max HP', ''], ['max_mp', 'Max MP', '']]],
      ['Stats', STATS.map(([k, label]) => [k, label, ''])],
      [digi() ? 'Gear bonuses, digitalized' : 'Gear bonuses', COMBAT.map(([k, label, unit]) => [k + sfx, label, unit])],
    ];
    // Each snapshot in the picker says how many of those lines differ from the latest (many snapshots differ in none),
    // counted the same way as the list below.
    const differs = (x) => groups.reduce((n, [, keys]) => n + keys.filter(([k]) => { const a = num(x[k]), b = num(L[k]); return a !== null && b !== null && a !== b; }).length, 0);
    const countText = (n) => (n ? `${n} change${n === 1 ? '' : 's'}` : 'no changes');
    // The picker is the app's own dropdown (a popover, as the digitalize demon's), not a native <select>: the browser
    // draws a native list itself, so its scrollbar can't be given the sidebar's look.
    // Each snapshot in it has a bin on its right: a click tints that row and asks "Delete this snapshot?" inside it
    // (the list stays where it was); the dropdown stays open after a delete, so several can go one after another.
    const rows = h('div', { class: 'char-list' });
    let asking = null;   // the snapshot (taken_at) asked about
    const drawRows = (focus) => {
      const top = rows.scrollTop;
      rows.replaceChildren(...earlier.slice().reverse().map((x) => {
        const on = x.taken_at === base.taken_at, ask = asking === x.taken_at;
        const at = (cls) => `.snap-row[data-at="${CSS.escape(x.taken_at)}"] ${cls}`;
        return h('div', { class: 'snap-row' + (ask ? ' asking' : ''), 'data-at': x.taken_at },
          h('button', { type: 'button', class: 'char snap-opt', 'aria-current': on ? 'true' : null,
            onclick: () => { menu.hidePopover(); if (!on) { S.compare = x.taken_at; S.over.delSnap = false; render(); } } },
            h('span', { class: 'char-text' }, h('span', { class: 'char-name', text: when(x.taken_at) }), h('span', { class: 'char-line', text: countText(differs(x)) }))),
          ask ? h('div', { class: 'snap-ask' }, h('span', { class: 'dgt-line', text: 'Delete this snapshot? The spreadsheet is copied first.' }),
            h('span', { class: 'dg-ask-btns' },
              h('button', { type: 'button', class: 'btn btn-sm snap-yes', onclick: () => deleteSnapshot(x, true) }, 'Delete'),
              h('button', { type: 'button', class: 'btn btn-ghost btn-sm snap-keep', onclick: () => { asking = null; drawRows(at('.snap-x')); } }, 'Keep')))
            : h('button', { type: 'button', class: 'btn btn-ghost btn-square btn-sm snap-x', 'aria-label': `Delete the snapshot of ${when(x.taken_at)}`, 'data-tip': 'Delete this snapshot',
              onclick: () => { asking = x.taken_at; drawRows(at('.snap-keep')); } }, icon('trash')));
      }));
      rows.scrollTop = top;
      const el = focus && rows.querySelector(focus);
      if (el) {
        el.focus({ preventScroll: true });
        // the asked row's question in view, by as little as it takes (only the list scrolls)
        const row = el.closest('.snap-row'), bottom = row.offsetTop - rows.offsetTop + row.offsetHeight;
        if (bottom > rows.scrollTop + rows.clientHeight) rows.scrollTop = bottom - rows.clientHeight;
      }
    };
    drawRows();
    const menu = h('div', { class: 'menu snap-menu', id: 'snap-menu', popover: 'auto', 'aria-label': 'Compare with snapshot',
      ontoggle: (ev) => { if (ev.newState === 'open') openSnapMenu(menu, sel); else if (asking) { asking = null; drawRows(); } } }, rows);
    const sel = h('button', { type: 'button', class: 'select snap-btn', popovertarget: 'snap-menu', 'aria-haspopup': 'listbox', 'aria-label': `Compare with snapshot: ${when(base.taken_at)}` },
      h('span', { text: `${when(base.taken_at)} · ${countText(differs(base))}` }), icon('chevron'));
    card.append(menu);
    // the picked snapshot can be deleted (a row of the spreadsheet, copied first); the latest is not in the picker
    const del = S.over.delSnap
      ? h('span', { class: 'snap-del' }, h('span', { class: 'card-sub', text: 'Delete this snapshot?' }),
        h('button', { type: 'button', class: 'btn btn-sm', disabled: S.over.deletingSnap ? '' : null, onclick: () => deleteSnapshot(base) }, S.over.deletingSnap ? 'Deleting…' : 'Delete'),
        h('button', { type: 'button', class: 'btn btn-ghost btn-sm', onclick: () => { S.over.delSnap = false; render(); } }, 'Keep'))
      : h('button', { type: 'button', class: 'btn btn-ghost btn-square btn-sm', 'aria-label': `Delete the snapshot of ${when(base.taken_at)}`,
        'data-tip': `Delete the snapshot of ${when(base.taken_at)}. The spreadsheet is copied first.`, onclick: () => { S.over.delSnap = true; render(); } }, icon('trash'));
    const wrap = sect('What changed', h('span', { class: 'snap-pick' }, h('label', { class: 'card-sub', style: 'display:inline-flex;align-items:center;gap:8px' }, 'since', sel), del), null, card);

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
  /** What changed's snapshot dropdown, just opened: under its button (above it when there is no room below), the picked
   *  snapshot in view and focused; arrow keys move through the list; scrolling the page closes it (its button moves away). */
  function openSnapMenu(menu, btn) {
    const r = btn.getBoundingClientRect(), w = menu.offsetWidth, hgt = menu.offsetHeight;
    menu.style.left = Math.max(8, Math.min(r.left, window.innerWidth - w - 8)) + 'px';
    menu.style.top = (r.bottom + 6 + hgt > window.innerHeight - 8 ? Math.max(8, r.top - 6 - hgt) : r.bottom + 6) + 'px';
    const cur = menu.querySelector('[aria-current="true"]');
    // the picked one in the middle of the list (only the list scrolls: scrollIntoView would move the page too)
    const list = menu.querySelector('.char-list');
    if (menu.dataset.top) { list.scrollTop = Number(menu.dataset.top); delete menu.dataset.top; if (cur) cur.focus({ preventScroll: true }); }   // reopened after a delete: where it was
    else if (cur) { list.scrollTop = cur.offsetTop - list.offsetTop - (list.clientHeight - cur.offsetHeight) / 2; cur.focus({ preventScroll: true }); }
    if (!menu.dataset.keys) {
      menu.dataset.keys = '1';
      menu.addEventListener('keydown', (ev) => {
        if (ev.key !== 'ArrowDown' && ev.key !== 'ArrowUp') return;
        const rows = [...menu.querySelectorAll('.snap-opt')], i = rows.indexOf(document.activeElement);
        const next = rows[ev.key === 'ArrowDown' ? Math.min(rows.length - 1, i + 1) : Math.max(0, i - 1)];
        if (next) { next.focus(); ev.preventDefault(); }
      });
    }
    $('#main').addEventListener('scroll', () => { if (menu.matches(':popover-open')) menu.hidePopover(); }, { once: true });
  }
  /** Delete one earlier snapshot (its spreadsheet row; the app copies the spreadsheet first), then show the page again.
   *  `fromMenu`: asked in the snapshot dropdown, which stays open afterwards, scrolled where it was. */
  async function deleteSnapshot(x, fromMenu) {
    if (S.over.deletingSnap) return;
    S.over.deletingSnap = true;
    const list = fromMenu && $('#snap-menu .char-list'), top = list ? list.scrollTop : 0;
    if (fromMenu) { const b = $('#snap-menu .snap-yes'); if (b) { b.disabled = true; b.textContent = 'Deleting…'; } }
    else render();
    try {
      const r = await host.call('deleteSnapshot', { takenAt: x.taken_at });
      if (!fromMenu || S.compare === x.taken_at) S.compare = null;   // from the menu: still compared with the one picked, unless that was it
      S.over.delSnap = false;
      await reload();
      toast('ok', `Snapshot of ${when(x.taken_at)} deleted`, r && r.backup ? `The spreadsheet was copied first: ${r.backup}` : '');
    } catch (e) {
      toast('err', 'Couldn’t delete the snapshot', String(e && e.message || e), 0);
    } finally {
      S.over.deletingSnap = false; render();
      const m = fromMenu && $('#snap-menu');
      if (m) { m.dataset.top = String(top); m.showPopover(); }   // openSnapMenu keeps the list there instead of centring the picked one
    }
  }

  // ---------- Bonuses ----------
  function partsList(row, onlySource) {
    const own = statsPart(row);   // of the two "your stats" parts (normal, digitalized), only the shown mode's
    const parts = (Array.isArray(row.parts) ? row.parts : []).filter((p) => p.kind !== 'stats' || p === own)
      .filter((p) => !onlySource || String(p.source).toLowerCase().includes(onlySource))
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
        h('span', null, String(p.source).replace(/\s*\((?:fusion )?set \d+\)\s*$/, ''), crystal ? h('span', { class: 'part-from', text: crystal }) : null, isTrue(p.digi) && p.kind !== 'stats' ? h('span', { class: 'tag ' + (digi() ? 'tag-digi' : 'tag-off'), style: 'margin-left:8px', text: 'Digi only' }) : null, isTrue(p.toggle) ? toggleTag('margin-left:8px') : null,
          p.kind === 'summoned' ? h('span', { class: 'tag ' + (digi() ? 'tag-off' : 'tag-demon'), style: 'margin-left:8px', 'data-tip': String(p.source).replace(/ \(summoned\)$/, '') + ' gives you this only while it is summoned beside you, not while you are digitalized', text: 'While summoned' }) : null,
          notes.length ? h('small', { text: notes.join(' · ') }) : null),
        h('span', { class: 'pv', text: p.value === null || p.value === undefined ? '' : fmt(p.value, row.unit, true) }));
    }));
  }

  function pageBonuses() {
    // Rows that only the digitalize demon gives appear in the Digitalized view only.
    const demonOnly = (r) => (r.parts || []).length > 0 && r.parts.every((p) => p.kind === 'demon');
    const summonedOnly = (r) => (r.parts || []).length > 0 && r.parts.every((p) => p.kind === 'summoned');   // nobody is out while digitalized
    const all = S.data.bonuses.filter((r) => (digi() ? !summonedOnly(r) : !demonOnly(r)));
    const SM = digi() ? '' : summonedName();   // "Alilat (summoned)": a column of its own while a partner is out
    const DG = S.data.digitalize, ncol = digi() || SM ? 5 : 4;
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
          const ownStats = statsPart(r), nParts = (r.parts || []).filter((p) => p.kind !== 'stats' || p === ownStats).length, fromStats = statsOf(r);
          const toggle = () => { if (S.bonus.open.has(r.row)) S.bonus.open.delete(r.row); else S.bonus.open.add(r.row); draw(); };
          const cell = (v, cls) => h('td', { class: 'num ' + (n0(v) ? cls || '' : 'zero'), text: num(v) === null ? '—' : n0(v) === 0 ? '0' : fmt(v, r.unit, true) });
          const tr = h('tr', { class: 'row-click' + (open ? ' open' : ''), onclick: (ev) => { if (!ev.target.closest('button')) toggle(); } },
            h('td', null, h('button', { type: 'button', class: 'row-btn', 'aria-expanded': String(open), onclick: toggle },
              h('span', { class: 'chev' }, icon('chevron')),
              h('span', null, r.bonus, h('span', { class: 'src-count', text: nParts === 1 ? '1 source' : `${nParts} sources` }),
                counted && extra ? h('span', { class: 'tag ' + (digi() ? 'tag-digi' : 'tag-off'), style: 'margin-left:8px', text: digi() ? `incl. ${fmt(extra, r.unit, true)} digi` : `${fmt(extra, r.unit, true)} digitalized` }) : null,
                counted && fromStats ? h('span', { class: 'tag tag-off', style: 'margin-left:8px', 'data-tip': statsNote(r), text: `incl. ${fmt(fromStats, r.unit, true)} from your stats` }) : null,
                counted ? capTag(n0(total), capOfBonus(r), r.unit, 'margin-left:8px') : null,
                counted && toggleShare(r) ? h('span', { class: 'tag tag-off', style: 'margin-left:8px', 'data-tip': toggleShareText(r, 'this total'), text: `${fmt(toggleShare(r), r.unit, true)} from toggles` }) : null))),
            cell(gear), cell(r.expertise), digi() ? cell(r.demon) : null, SM ? cell(r.summoned === undefined ? 0 : r.summoned) : null,
            counted ? cell(total, 'emph')
              : h('td', { class: 'num zero', 'data-tip': 'Depends on the fight, so it is not added up', text: '—' }));
          body.append(tr);
          // Found through an item's name: list just that item's lines, unless the row was opened by hand.
          if (open) {
            const lines = counted ? [toggleShareText(r, 'this total')].filter(Boolean) : [];
            body.append(h('tr', { class: 'detail' }, h('td', { colspan: ncol }, lines.map((t) => h('p', { class: 'note', text: t })), partsList(r, viaSource && !S.bonus.open.has(r.row) ? q : null))));
          }
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
        SM ? h('th', { scope: 'col', class: 'num', text: SM, 'data-tip': 'Counts only while this demon is summoned beside you. It is left out of the digitalized totals.' }) : null,
        h('th', { scope: 'col', class: 'num', text: digi() ? 'Total, digitalized' : 'Total' }))), body)));
    page.append(h('p', { class: 'note', text: 'Gear includes tarot, soul fusions, mods, complete sets and appearance bonuses. For bonuses that only apply in some fights, the largest single value is shown.'
      + ' Cooldown and incantation totals also count what your stats take off: 2.5% per 10 VIT (cooldown) or INT (incantation), and 1.5% per 10 SPD for both. Cooldown stops at 95% off.'
      + (SM ? ` ${SM.replace(/ \(summoned\)$/, '')}'s column counts only while ${SM.replace(/ \(summoned\)$/, '')} is summoned beside you; it is not in the digitalized totals.` : '') }));
    page.append(h('p', { class: 'note', text: togglesNote() }));
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
  // The short name in lists: "Set bonus" / "Fusion set", or "2 sets" when several sets share the same pieces (the sets' numbers are internal and not shown).
  const setLabel = (g) => (setIds(g).length > 1 ? `${setIds(g).length} sets` : g.slot === 'Fusion set' ? 'Fusion set' : 'Set bonus');
  // The hover tooltip of a worn piece: the game's own tooltip text (kept from the snapshot), under its icon and name.
  // A set has no tooltip in the game: it lists its pieces and what the set adds.
  // The order of a piece's tooltip sections (the user's, not the game's); the set bonus always goes before them.
  const TIP_ORDER = ['appearance bonus', 'basic features', 'characteristics', 'tarot fusion', 'soul fusion', 'modification slot'];
  const gearTip = (g) => () => {
    const box = h('div', { class: 'tt' });
    if (g.kind === 'set') {
      const worn = /\((\d+)\/(\d+),/.exec(String(g.item || ''));
      box.append(tipHead(gearIcon(g), setTitle(g), [g.slot, worn ? `${worn[1]} of ${worn[2]} worn` : '', setIds(g).length > 1 ? `${setIds(g).length} sets` : ''].filter(Boolean).join(' · ')));
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
      h('div', { class: 'tt-sec', text: setIds(s).length > 1 ? `Set bonus · ${setIds(s).length} sets` : 'Set bonus' }),
      h('div', { class: 'tt-line', text: `Pieces: ${s.shown_as || ''}` }),
      ...s.parts.map(({ row, part }) => h('div', { class: 'tt-line', text: `* ${row.bonus} ${part.value === null || part.value === undefined ? '' : fmt(part.value, row.unit, true)}${isTrue(part.digi) ? ' (digitalized)' : ''}` }))]);
    box.append(...sets, ...secs.map((b, i) => ({ b, i })).sort((x, y) => rank(x.b) - rank(y.b) || x.i - y.i).flatMap((x) => x.b.filter(noSetLine)));
    if (!lines.length) box.append(h('div', { class: 'tt-line', text: 'No tooltip was read for this piece.' }));
    gearTipPics(box, g);
    return box;
  };
  /**
   * Puts pictures in a worn piece's tooltip sections: the piece lending the basic features or the characteristics,
   * the two crystals, a set's pieces. The page's gear data has only the piece's own picture; the others come with the
   * Build Planner's reading of the shown character (host "simInit": the same pieces, with a picture per part), asked
   * once and kept. When that isn't in yet, the tooltip shows without them and gets them when it arrives; with no
   * reader behind the page (or a character never read) it simply stays as it was. The game's text already names the
   * crystal and the set's pieces: there the picture joins that line.
   */
  function gearTipPics(box, g) {
    const me = S.meta.character || '';
    if (!me || !g.item_id) return;
    const put = (init) => {
      const res = init && init.result, x = res && (res.slots || []).find((r) => !r.empty && r.id === g.item_id && String(r.slot).toLowerCase() === String(g.slot).toLowerCase());
      if (!x || box.dataset.pics) return;
      box.dataset.pics = '1';
      const pics = (list) => (list || []).filter(Boolean).map((src) => h('img', { class: 'tt-src-pic', src, alt: '' }));
      const mine = simPlain(x.name);
      const sets = (res.sets || []).filter((s) => s.kind === 'set' && mine && String(s.name || '').includes(mine));
      let setAt = 0;
      [...box.querySelectorAll('.tt-sec')].forEach((sec) => {
        const title = sec.textContent.trim().toLowerCase(), next = sec.nextElementSibling;
        const line = (icons, name) => { if (icons.some(Boolean)) sec.after(h('div', { class: 'tt-line tt-why tt-src' }, pics(icons), h('span', { text: name }))); };
        const into = (icons) => { if (next && next.classList.contains('tt-line') && icons.some(Boolean)) { next.classList.add('tt-src'); next.prepend(...pics(icons)); } };
        if (title.startsWith('set bonus')) into((sets[setAt++] || {}).icons || []);
        else if (title === 'appearance bonus') line([x.icon], x.name);
        else if (title === 'basic features') line([x.basic ? x.basic_icon : x.icon], x.basic || x.name);
        else if (title === 'characteristics') line([x.special ? x.special_icon : x.icon], x.special || x.name);
        else if (title === 'tarot fusion') into([x.tarot_icon]);
        else if (title === 'soul fusion') into([x.soul_icon]);
      });
    };
    const ready = S.sim.bases.get(simBaseKey(me));
    if (ready) put(ready.init); else simBase(me).then((entry) => put(entry.init)).catch(() => { /* no reader, or nothing read for this character: no pictures */ });
  }
  // The hover tooltip of any other bonus source: its icon and name, and the game's description when it is a skill.
  // A summoned demon's Demon Force entry or epitaph is headed by its item's name (the note then leaves that name out).
  const sourceTip = (p, iconOf) => () => h('div', { class: 'tt' },
    p.item_name ? tipHead(gearIcon(iconOf), p.item_name, `${KIND[p.kind] || p.kind || ''} · ${p.source}`)
      : tipHead(gearIcon(iconOf), String(p.source).replace(/ \((Other learned skills|Quest bonus[^)]*\)|Demonic Compendium[^)]*\))\)?$/, ''), KIND[p.kind] || p.kind || ''),
    ...tipLines(p.item_tip), ...tipLines(p.tip),
    // a Demon Force entry's note says only "Demon Force" once its item's name is the title: left out
    p.note && !(p.item_name && p.note === `Demon Force · ${p.item_name}`) ? h('div', { class: 'tt-line tt-gap', text: p.item_name ? String(p.note).replace(` · ${p.item_name}`, '') : p.note }) : null);
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
    if (isSet && m) meta.append(h('span', { class: 'tag', text: `${m[1]} of ${m[2]} worn` }), ...(setIds(g).length > 1 ? [h('span', { class: 'tag', text: `${setIds(g).length} sets` })] : []));
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
        const cap = capOfBonus(c.row), whole = round2(t + n0(c.row.expertise) + statsOf(c.row));   // the cap counts expertise (and what your stats take off) too
        const past = cap === null ? -1 : pastCap(whole, cap), also = statsOf(c.row) ? 'expertise and your stats' : 'expertise';
        if (cap !== null && past >= 0) return h('td', { class: 'num capped', 'data-tip': past > 0
          ? `With ${also} this is ${fmt(whole, c.row.unit)}: ${fmt(past, c.row.unit).replace(/^\+/, '')} past the ${fmt(cap, c.row.unit).replace(/^\+/, '')} cap, which adds nothing`
          : `With ${also} this is exactly at the ${fmt(cap, c.row.unit).replace(/^\+/, '')} cap`, text: (t ? fmt(t, c.row.unit, true) : '0') + (past > 0 ? ' ▲' : ' ●') });
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
  const skillIcon = (r, folder) => {   // folder: another character's skill icons (the Build Planner's imported base); else the shown character's
    const box = h('span', { class: 'skill-ico', 'aria-hidden': 'true' });
    if (r.icon && (folder || S.meta.skillIcons)) {
      const img = h('img', { src: (folder || S.meta.skillIcons) + r.icon + '.png', alt: '', loading: 'lazy' });
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
                : (() => { const n = mine.filter((r) => r.skill).length; return n === 1 ? '1 passive' : `${n} passives`; })()].filter(Boolean).join(' · ') }),   // (a line with no skill is a note, e.g. a rank to pick from)
            o.tag ? o.tag(name) : null,
            anyCounted ? null : h('span', { class: 'tag tag-off', text: 'Not counted' }),
            // closed: its passives as a row of small icons (hidden by the stylesheet while the group is open)
            // (a passive the game has no icon for is left out of the row; it is in the list when the group is open)
            o.noIcons ? null : h('span', { class: 'exp-icons' }, mine.filter((r) => r.icon).map((r) => {
              const ic = skillIcon(r);
              if (!isTrue(r.counted)) ic.classList.add('off');
              if (o.richTips) return expSkillTip(ic, { id: r.skill_id, name: r.skill, icon: r.icon, rank: r.rank, expertise: r.expertise, effects: r.effects, off: r.toggle && !r.toggle_on, counted: isTrue(r.counted) });
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
    page.append(h('div', { class: 'sect', id: o.id || null }, h('div', { class: 'toolbar' },
      h('h2', { class: 'section-title', text: o.heading }), h('span', { class: 'toolbar-spacer' }),
      o.search ? searchBox(st.q, o.search, (v) => { st.q = v; draw(); }) : null,
      h('button', { type: 'button', class: 'btn btn-sm', onclick: () => { st.closed.clear(); draw(); } }, 'Expand all'),
      h('button', { type: 'button', class: 'btn btn-sm', onclick: () => { names.forEach((n) => st.closed.add(n)); draw(); } }, 'Collapse all')), listHost));
    draw();
  }

  /**
   * The game-style tooltip of an expertise skill's icon (the Expertise page's summary and closed groups): icon and name,
   * which expertise and rank give it, then the game's own description (skill-db.json, loaded on first use); until that
   * is in, or for a skill it lacks, the effects the reader counted. s: { id, name, icon, rank, active, off, expertise, effects, counted }.
   */
  function expSkillTip(el, s) {
    loadSkillData();
    el.removeAttribute('aria-hidden'); el.setAttribute('role', 'img'); el.setAttribute('aria-label', s.name || 'Skill');
    return withTip(el, () => {
      const x = sklById(s.id), fx = String(s.effects || '').split('; ').filter(Boolean);
      const where = [s.expertise, s.rank ? String(s.rank).replace(/^Class (\d+) Rank (\d+)$/, '$1-$2') : null].filter(Boolean).join(' ');
      return h('div', { class: 'tt' },
        tipHead(s.icon || !x ? skillIcon(s) : sklIcon(x), s.name || '—', [s.active ? 'Active skill' : 'Passive skill', where].filter(Boolean).join(' · ')),
        ...(x && x.desc ? tipLines(x.desc) : fx.map((t) => h('div', { class: 'tt-line', text: t }))),
        s.off ? h('div', { class: 'tt-line tt-gap tt-why', text: 'Toggle switched off: its effects are not counted.' })
          : s.counted === false ? h('div', { class: 'tt-line tt-gap tt-why', text: 'Not counted.' }) : null);
    });
  }

  /** Expertise points as the Expertise page shows them: the game's number / 100, with no ".00pt" (the user's request). */
  const expPt = (v) => (n0(v) / 100).toLocaleString('en-US', { maximumFractionDigits: 2 });

  /**
   * The Expertise page's opening summary, in the Overview Expertise band's style (the user's request): a top line with
   * the counts at the left and points / cap at the right (points / 100, as the game's Expertise window) over a thin
   * bar; then one compact line per raised expertise, regular ones by points and then the chains: its name and class-rank
   * (orange), how far it is to its highest class (max_class from the client's ExpertClassData; Class <max> Rank 0 is
   * "Max"), its points (chains cost none), and one run of the icons of every skill its reached ranks give (passives,
   * then actives). A line filters the passive list below to that expertise.
   */
  function expSummary(rows) {
    const R = S.data.ranks || {}, P = R.points || null, all = (R.ranks || []).filter((r) => n0(r.points) > 0);
    const pt = expPt;
    const card = h('section', { class: 'card exp-sum' });
    const wrap = sect('Summary', null, null, card);
    if (!all.length) { card.append(h('p', { class: 'note', text: 'No expertise read yet. Take a snapshot.' })); return wrap; }
    const maxed = (r) => r.max_class > 0 && n0(r.class) * 10 + n0(r.rank) >= r.max_class * 10;
    const byPoints = (a, b) => n0(b.points) - n0(a.points) || String(a.name).localeCompare(b.name);
    const regular = all.filter((r) => !r.chain).sort(byPoints), chains = all.filter((r) => r.chain).sort(byPoints);
    const counted = rows.filter((r) => isTrue(r.counted)).length;
    const plural = (n, one, many) => `${n} ${n === 1 ? one : many}`;
    const counts = [plural(regular.length, 'expertise raised', 'expertises raised'), `${all.filter(maxed).length} at their highest class`,
      plural(chains.length, 'chain', 'chains'), plural(counted, 'passive counted', 'passives counted')].join(' · ');
    const free = P && P.cap ? n0(P.cap) - n0(P.total) : 0;
    card.append(h('div', { class: 'exp-top' }, h('span', { class: 'exp-sum-counts', text: counts }),
      P ? h('div', { class: 'band-points', 'data-tip': P.cap ? `Cap ${pt(P.cap)} = 17,000 + 1,000 per 10 levels (level ${P.level})`
          + (P.level === 99 ? ' + 1,000 at 99' : '') + ` + ${P.extension} expansions × 1,000` : null },
        h('span', { class: 'band-pts', text: P.cap ? `${pt(P.total)} / ${pt(P.cap)} · ${free > 0 ? pt(free) + ' free' : 'at the cap'}` : `${pt(P.total)}, no cap` }),
        P.cap ? h('span', { class: 'band-bar' }, h('i', { style: `width:${Math.max(0, Math.min(100, (n0(P.total) / n0(P.cap)) * 100))}%` })) : null) : null));
    const line = (r) => {
      const top = r.max_class > 0 ? r.max_class * 100000 : 0, done = maxed(r);
      // the skills its reached ranks give (expertise-ranks.json "skills", actives too); older data: its passives from expertise.json
      const mine = S.data.expertise.filter((x) => x.expertise === r.name && !isExtra(x));
      const offIds = new Set(mine.filter((x) => x.toggle && !x.toggle_on).map((x) => x.skill_id));
      const skills = Array.isArray(r.skills) ? r.skills.map((s) => ({ ...s, off: offIds.has(s.id) }))
        : mine.map((x) => ({ id: x.skill_id, name: x.skill || '—', icon: x.icon, rank: x.rank || '', active: false, off: x.toggle && !x.toggle_on }));
      const ordered = skills.filter((s) => s.icon && !s.active).concat(skills.filter((s) => s.icon && s.active));
      const icons = ordered.map((s, i) => {
        const ic = expSkillTip(skillIcon(s), { ...s, expertise: r.name, effects: (mine.find((x) => x.skill_id === s.id) || {}).effects });
        if (s.off) ic.classList.add('off');
        if (s.active && (i === 0 || !ordered[i - 1].active)) ic.classList.add('first');   // a little air before the actives
        return ic;
      });
      return h('button', { type: 'button', class: 'exp-sum-row', 'aria-label': `${r.name}, class ${r.class} rank ${r.rank}`,
        'data-tip': `Show ${r.name}’s passives below` + (r.chain ? ' (a chain: worked out from the expertises it is made of; it costs no points)' : ''),
        onclick: () => { S.exp.q = r.name; render(); const el = document.getElementById('exp-list'); if (el) el.scrollIntoView({ block: 'start' }); } },
        h('span', { class: 'exp-rank' }, h('span', { class: 'exp-sum-name', text: r.name }), h('b', { text: `${r.class}-${r.rank}` })),
        h('span', { class: 'band-bar', 'aria-hidden': 'true' }, top ? h('i', { style: `width:${Math.min(100, (n0(r.points) / top) * 100)}%` }) : null),
        h('span', { class: 'exp-sum-max' + (done ? ' done' : ''), text: done ? 'Max' : top ? `of ${r.max_class}-0` : '' }),
        h('span', { class: 'exp-sum-pt', text: r.chain ? '' : pt(r.points) }),
        h('span', { class: 'band-items exp-icons-run' }, icons));
    };
    const block = (title, list) => (list.length ? [h('h4', { class: 'list-head', text: title }), h('div', { class: 'exp-sum-list' }, list.map(line))] : null);
    card.append(...[block('Expertise', regular), block('Chain expertise', chains)].flat().filter(Boolean));
    return wrap;
  }

  /**
   * The Expertise page's cap section: what the cap is made of (the level's part, then the raises) and, one line per
   * place that raises it, a chip per raise: taken (orange), still open (outline), ready to claim (a Shiho reward whose
   * race already has the DLevel) or not known (dashed: the Black Frost exchanges leave nothing the app can read).
   * expertise-ranks.json points.raises (calc::expertise_cap): NewMoon's raises are only there on NewMoon. What the
   * game's cap holds beyond the raises found is "other"; when it is 0 the unknown ones can't have been taken.
   */
  function expCap() {
    const P = (S.data.ranks || {}).points || null, X = P && P.raises;
    if (!P || P.cap == null) return null;
    const pt = expPt, k = (n) => pt(n * 100000), lvl = n0(P.level);
    const base = 1700000 + Math.floor(lvl / 10) * 100000 + (lvl === 99 ? 100000 : 0);
    const next = lvl < 90 ? Math.floor(lvl / 10) * 10 + 10 : lvl < 99 ? 99 : null;
    const card = h('section', { class: 'card exp-sum exp-cap' });
    const head = `${pt(base)} for level ${lvl}` + (next ? ` (${k(1)} more at level ${next})` : '') + ` + ${k(n0(P.extension))} from raises`;
    if (!X) {
      card.append(h('p', { class: 'note', text: `${head}. Take a snapshot to see which raises you have.` }));
      return sect('Expertise cap', pt(P.cap), null, card);
    }
    const other = X.other, known = (s) => s.got != null || other === 0;   // nothing unexplained: the unknown ones weren't taken
    const list = X.sources, max = list.reduce((a, s) => a + n0(s.amount), 0);
    const open = list.filter((s) => !(s.got === true)).reduce((a, s) => a + (known(s) ? n0(s.amount) : 0), 0);
    const taken = n0(P.extension), ready = list.filter((s) => s.ready);
    card.append(h('div', { class: 'exp-top' }, h('span', { class: 'exp-sum-counts', text: head }),
      h('div', { class: 'band-points', 'data-tip': `Every raise there is: ${k(max)}` + (X.newmoon ? ', NewMoon’s own included' : '') },
        h('span', { class: 'band-pts', text: `${k(taken)} of ${k(max)} raised` + (open ? ` · ${k(open)} still to get` : '') }),
        h('span', { class: 'band-bar' }, h('i', { style: `width:${Math.max(0, Math.min(100, (taken / (max || 1)) * 100))}%` })))));
    const groups = [];
    list.forEach((s) => { const g = groups.find((x) => x.name === s.group); if (g) g.list.push(s); else groups.push({ name: s.group, list: [s], newmoon: s.newmoon }); });
    const chip = (s) => {
      const state = s.got === true ? 'got' : s.ready ? 'ready' : known(s) ? 'open' : 'unknown';
      const said = { got: 'You have it', ready: 'Ready to claim: one of these races has the DLevel', open: 'Not taken yet', unknown: 'The game data doesn’t say whether you have it' }[state];
      return h('span', { class: `tag cap-chip cap-${state}`, 'data-tip': `${s.name} · ${k(s.amount)}\n${s.how}\n${said}` },
        (s.short ?? s.name) ? h('span', { text: s.short ?? s.name }) : null, h('b', { text: `+${(n0(s.amount) * 1000).toLocaleString('en-US')}` }));
    };
    card.append(h('div', { class: 'cap-list' }, groups.map((g) => {
      const got = g.list.filter((s) => s.got === true).reduce((a, s) => a + n0(s.amount), 0), all = g.list.reduce((a, s) => a + n0(s.amount), 0);
      return h('div', { class: 'cap-row' },
        h('span', { class: 'cap-group' }, h('span', { text: g.name }), g.newmoon ? h('span', { class: 'cap-server', text: 'NewMoon' }) : null),
        h('span', { class: 'cap-chips' }, g.list.map(chip)),
        h('span', { class: 'exp-sum-pt' + (got === all ? ' done' : ''), text: `${k(got)} / ${k(all)}` }));
    })));
    const notes = [];
    if (ready.length) notes.push(`${ready.length === 1 ? 'One Shiho reward is' : `${ready.length} Shiho rewards are`} ready to claim: ${k(ready.reduce((a, s) => a + n0(s.amount), 0))}.`);
    if (other > 0) notes.push(`${k(other)} of your cap isn’t matched to a raise above` + (X.newmoon ? ' (the Black Frost exchanges leave nothing the app can read: most likely those).' : '.'));
    if (other < 0) notes.push(`The raises found add up to ${k(-other)} more than the game’s cap: one of them is read wrong.`);
    if (X.newmoon == null) notes.push('NewMoon’s own raises show when the app sees the game is on NewMoon (the game folder’s server list and the client’s connection).');
    if (list.some((s) => s.got == null && !/^Black Frost/.test(s.name))) notes.push('Some raises couldn’t be read from the game: take a snapshot with the character logged in.');
    notes.forEach((t) => card.append(h('p', { class: 'note', text: t })));
    return sect('Expertise cap', pt(P.cap), null, card);
  }

  // ---------- Expertise page: the planner ----------
  // Other ranks on top of the shown character's, with what they do to the numbers. The same reader commands as the
  // Build Planner ("simInit" for the character as last read, "simulate" for the plan: its worn gear as read, only the
  // points changed) and the same editor (expEditor). The plan is kept per character in the page's own storage; nothing
  // touches the game or the character's files.
  const xpKey = (who) => 'exp-plan:' + who;
  /** The plan kept for this character: { points: {expertise: points} (what differs from the ranks as read), cap }. */
  function xpKept(who) {
    try { const v = JSON.parse(store.get(xpKey(who)) || 'null'); return v && typeof v === 'object' ? { points: v.points || {}, cap: v.cap ?? null } : { points: {}, cap: null }; } catch (e) { return { points: {}, cap: null }; }
  }
  /** The expertises the plan changes (ids). */
  const xpChanged = () => { const X = S.xp; if (!X.init) return []; const p0 = X.init.build.points; return Object.keys(X.points).filter((id) => n0(X.points[id]) !== n0(p0[id])); };
  function xpSave() {
    const X = S.xp;
    if (!X.init) return;
    store.set(xpKey(X.who), JSON.stringify({ points: Object.fromEntries(xpChanged().map((id) => [id, n0(X.points[id])])), cap: X.cap }));
  }
  async function xpLoad() {
    const X = S.xp;
    if (X.state !== 'idle' || !X.who) return;
    const tok = ++X.loadSeq, who = X.who;
    X.state = 'loading';
    try {
      const init = await host.call('simInit', { base: who });
      if (tok !== X.loadSeq) return;
      const kept = xpKept(who), ids = new Set(init.expertise.map((d) => String(d.id)));
      X.init = init; X.points = simClone(init.build.points); X.cap = kept.cap; X.cur = init.result; X.demon = simDemonDefault(init); X.state = 'ready';
      Object.entries(kept.points).forEach(([id, v]) => { if (ids.has(String(id))) X.points[id] = n0(v); });
      if (xpChanged().length) xpRun(false);
    } catch (e) {
      if (tok !== X.loadSeq) return;
      X.state = 'failed'; X.error = String((e && e.message) || e);
    }
    if (S.page === 'expertise') render();
  }
  /** Asks the reader for the plan's result; only the answer to the latest change is used. */
  async function xpRun(keep) {
    const X = S.xp, seq = ++X.seq;
    clearTimeout(X.timer);
    if (keep !== false) xpSave();
    X.working = true;
    try {
      const res = await host.call('simulate', { base: X.who, demon: X.demon, slots: X.init.build.slots, points: X.points });
      if (seq !== X.seq) return;
      X.cur = res;
    } catch (e) {
      if (seq !== X.seq) return;
      toast('err', 'That plan could not be worked out', String((e && e.message) || e));
    }
    X.working = false;
    if (X.redraw) X.redraw();
  }
  const xpRunSoon = () => { const X = S.xp; clearTimeout(X.timer); X.working = true; if (X.redraw) X.redraw(); X.timer = setTimeout(() => xpRun(), 90); };

  /** The skills a changed expertise gains or loses: the ranks between the one as read and the planned one. */
  function xpUnlocks(d, from, to) {
    const lo = Math.min(from, to), hi = Math.max(from, to);
    return d.unlocks.filter((u) => u.rank > lo && u.rank <= hi).map((u) => ({ rank: u.rank, pick: u.pick, skills: u.skills }));
  }

  function expPlanner() {
    const X = S.xp, me = S.meta.character || '';
    if (X.who !== me) Object.assign(X, { who: me, state: 'idle', init: null, cur: null, points: null, loadSeq: X.loadSeq + 1 });
    xpLoad();
    if (X.state !== 'ready') {
      const body = X.state === 'failed'
        ? h('div', { class: 'notice notice-warn' }, icon('alert'), h('div', null, h('b', { text: 'The planner can’t start. ' }), X.error, ' ',
          h('button', { type: 'button', class: 'btn btn-sm', style: 'margin-left:8px', onclick: () => { X.state = 'idle'; render(); } }, 'Try again')))
        : h('p', { class: 'note', text: me ? 'Reading your expertise…' : 'No character shown yet.' });
      return sect('Expertise planner', null, null, body);
    }
    // the app's digitalized / not digitalized switch decides the demon, as on the other pages
    const dm = simDemonDefault(X.init);
    if (X.demon !== dm) { X.demon = dm; xpRun(false); }
    const I = X.init, defs = I.expertise, byId = new Map(defs.map((d) => [String(d.id), d]));
    const resetBox = h('div', { class: 'toolbar' }), out = h('aside', { class: 'sim-out xp-out' });
    const capOf = () => (X.cap === null || X.cap === undefined ? X.init.result.expertise.cap : X.cap);
    const drawReset = () => {
      const dirty = xpChanged().length > 0 || (X.cap !== null && X.cap !== undefined);
      const allOpen = defs.filter((d) => !d.chain).every((d) => X.open.has(d.id));
      simFill(resetBox,
        h('button', { type: 'button', class: 'btn btn-sm', 'data-tip': allOpen ? 'Hide what each expertise’s ranks unlock' : 'Show what every rank of every expertise unlocks',
          onclick: () => { defs.filter((d) => !d.chain).forEach((d) => (allOpen ? X.open.delete(d.id) : X.open.add(d.id))); render(); } }, allOpen ? 'Collapse all' : 'Expand all'),
        h('button', { type: 'button', class: 'btn btn-sm', disabled: dirty ? null : true, 'data-tip': 'Back to your ranks and cap as last read',
          onclick: () => { clearTimeout(X.timer); X.seq++; X.working = false; X.points = simClone(I.build.points); X.cap = null; xpSave(); X.cur = I.result; render(); } }, 'Reset to your ranks'));
    };
    const ed = expEditor({ defs, P: X.points, P0: I.build.points, cap0: I.result.expertise.cap, RP: (S.data.ranks && S.data.ranks.points) || {}, open: X.open,
      cap: capOf, setCap: (v) => { X.cap = v; xpSave(); },
      ranksNow: () => X.cur.expertise.ranks, ranksBefore: () => I.result.expertise.ranks,
      onRank: () => { drawReset(); xpRunSoon(); }, onCap: () => drawReset() });

    function drawOut() {
      const changed = xpChanged(), cur = X.cur, diff = simDiff(I.result, cur);
      const groups = [['Stats', 'Your stats'], ['Combat', 'Combat'], ['Affinity', 'Affinity'], ['Other', 'Other'], ['Partner', 'Your partner'], ['Only in some fights', 'Only in some fights']];
      const list = h('div', { class: 'sim-others' });
      groups.forEach(([g, title]) => {
        const rows = diff.filter((x) => x.group === g);
        if (!rows.length) return;
        list.append(h('div', { class: 'chg-group', text: title }), ...rows.map((x) => h('div', { class: 'chg' }, h('span', null, x.label, x.fresh ? chgDot('new') : x.gone ? chgDot('gone') : null),
          h('span', { class: 'chg-vals' }, x.fresh ? null : fmt(x.from, x.unit) + ' → ', h('b', { text: x.gone ? 'gone' : fmt(x.to, x.unit) })), simDelta(x))));
      });
      // the skills the plan gains or loses, per changed regular expertise
      // the changed expertises (ranks from the points), then the chains the reader worked out a new rank for
      const moved = [...changed.map((id) => byId.get(String(id))).filter((d) => d && !d.chain)
        .map((d) => ({ d, from: Math.floor(n0(I.build.points[d.id]) / 10000), to: Math.floor(n0(X.points[d.id]) / 10000) })),
        ...defs.filter((d) => d.chain).map((d) => ({ d, from: n0(I.result.expertise.ranks[d.id]), to: n0(cur.expertise.ranks[d.id]) })).filter((m) => m.from !== m.to)];
      const skillRows = moved.map(({ d, from, to }) => {
        const up = to > from;
        const steps = xpUnlocks(d, from, to);
        if (!steps.length) return null;
        return h('div', { class: 'xp-unlock ' + (up ? 'good' : 'bad') },
          h('span', { class: 'xp-unlock-head' }, h('b', { text: d.name }), h('span', { class: 'card-sub', text: `${Math.floor(from / 10)}-${from % 10} → ${Math.floor(to / 10)}-${to % 10}` + (d.chain ? ' · chain' : '') })),
          h('ul', { class: 'sim-unlocks xp-unlock-list' }, steps.map((u) => h('li', null, h('span', { class: 'sim-unlock-rank', text: (up ? '+ ' : '− ') + `${Math.floor(u.rank / 10)}-${u.rank % 10}` }),
            h('span', null, unlockSkills(u, d.name))))));
      }).filter(Boolean);
      simFill(out, h('section', { class: 'card sim-card' },
        h('div', { class: 'card-head' }, h('h3', { class: 'card-title', text: changed.length ? `With your ${changed.length} change${changed.length === 1 ? '' : 's'}` : 'As you are now' }),
          h('span', { class: 'card-sub', role: 'status', text: X.working ? 'working it out…' : '' })),
        !changed.length ? h('p', { class: 'note', text: 'Move a rank on the left: the skills it unlocks or loses and what it does to your numbers are listed here.' }) : null,
        skillRows.length ? [h('h4', { class: 'list-head', text: 'Skills' }), h('div', { class: 'xp-unlocks' }, skillRows)] : null,
        diff.length ? [h('h4', { class: 'list-head', text: 'Numbers' }), list] : changed.length && !X.working ? h('p', { class: 'note', text: 'These ranks move none of your numbers.' }) : null),
        h('p', { class: 'note', text: `Starts from your gear and ranks as last read (${when(I.meta.stats_at)}, level ${I.meta.level}). As everywhere in the app: no buffs and nothing from the partner demon. Ranks with a choice count the skill you learned in game.` }));
    }
    X.redraw = () => { if (S.page !== 'expertise' || !out.isConnected) return; ed.live(); drawOut(); drawReset(); };
    drawReset(); drawOut();
    return sect('Expertise planner', null, resetBox, h('div', { class: 'sim-layout xp-layout' }, ed.box, out));
  }

  function pageExpertise() {
    const rows = S.data.expertise.filter((r) => !isDlevel(r) && !isExtra(r));
    const page = h('div', { class: 'page' });
    // two views (the user's request): your expertise as read, and the planner with every expertise there is
    const plan = S.exp.view === 'plan';
    const view = (id, label, tip) => h('button', { type: 'button', 'aria-pressed': String(S.exp.view === id), 'data-tip': tip,
      onclick: () => { if (S.exp.view === id) return; S.exp.view = id; store.set('exp-view', id); render(); $('#main').scrollTop = 0; } }, label);
    const toggle = h('div', { class: 'seg', role: 'group', 'aria-label': 'View' },
      view('mine', 'Your expertise', 'Your ranks, cap and passives as last read'), view('plan', 'Planner', 'Every expertise on a slider: try other ranks and see what they change'));
    page.append(pageHead(plan ? 'Every expertise and what its ranks unlock, starting from your ranks. Try others and see the skills and numbers they change. Nothing here touches the game or your data.'
      : 'Passive skills your expertise ranks unlock (this assumes you acquired every one), and your other learned passives. Digitalization Levels are on the Digitalize page; the quest bonus and Demonic Compendium are on Bonuses.', toggle));
    if (plan) { page.append(expPlanner()); return page; }
    page.append(expSummary(rows));
    const cap = expCap();
    if (cap) page.append(cap);
    // the expertise share of each bonus, without what the Digitalization Levels, quest bonus and compendium put into it
    const totals = S.data.bonuses.filter((r) => isTrue(r.counted)).map((r) => [r, round2(n0(r.expertise) - shareOf(r, ['dlevel', 'quest', 'compendium']))]).filter(([, v]) => v);
    page.append(sect('What expertise adds in total', `${rows.filter((r) => isTrue(r.counted)).length} passives counted`, null,
      totalsCard(null, null, totals, 'No expertise bonuses counted yet.')));
    passiveList(page, rows, S.exp, { id: 'exp-list', richTips: true, heading: 'Every passive', search: 'Search an expertise, passive or effect', rankTip: 'The rank that unlocks this passive', empty: 'No expertise read yet. Take a snapshot.' });
    return page;
  }

  // ---------- Digitalize ----------
  /**
   * The digitalize demon's own skills (digitalize.json "skills"), for the demon card: `icons` is the one-line version
   * (the icons of the skills in its slots, no features; the game's description on hover), `parts` the opened one
   * (the skills, then the features: kind, icon and name, and what each gives). A skill you don't get while
   * digitalized is dimmed (the reason, "why" in the data, is not shown: the user didn't want the label). Null when the saved data has no skills yet.
   */
  /** For a demon skill you don't get while digitalized (greyed): the reason, as the last line of its tooltip. Not a label on the page: the user took that out. */
  const skillWhy = (s) => (!isTrue(s.counted) && s.why ? h('div', { class: 'tt-line tt-gap tt-why', text: 'Not applied while digitalized: ' + String(s.why).charAt(0).toLowerCase() + String(s.why).slice(1) + '.' }) : null);

  function demonSkills(DG) {
    if (!DG || !Array.isArray(DG.skills)) return null;
    const row = (s) => {
      const ic = skillIcon(s);
      // a feature shown with the Features boost's picture (it has none of its own): tinted, so it isn't taken for the boost
      if (s.kind === 'Feature' && /^digitalize_/.test(String(s.icon || ''))) ic.classList.add('as-feature');
      const name = h('span', { class: 'skill', tabindex: 0 }, ic, h('span', { text: s.name }));
      if (s.tip) withTip(name, () => h('div', null, tipHead(skillIcon(s), s.name, s.kind === 'Equipment' ? 'Demon equipment' : s.kind), ...tipLines(s.tip), skillWhy(s)));
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
      return withTip(ic, () => h('div', null, tipHead(skillIcon(s), s.name, s.kind), ...tipLines(s.tip), skillWhy(s)));
    })) : null;
    const part = (label, rows, none) => [
      h('div', { class: 'dg-skill-head' }, h('h4', { class: 'section-title', text: label }), rows.length ? h('span', { class: 'card-sub', text: count(rows) }) : null),
      rows.length ? h('div', { class: 'dg-skill-list' }, rows.map(row)) : h('p', { class: 'note', text: none })];
    return { icons, parts: h('div', { class: 'demon-skills' }, part('Skills', slots, 'No skills in its slots.'), part('Features', feats, 'It has no features.')) };
  }

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
    if (d.inComp === false) tags.push(h('span', { class: 'tag tag-off', 'data-tip': 'It was not in your COMP when the game was last read. It is kept because it was read before, so you can still see what it gives.', text: 'Not in your COMP' }));
    return tags.length ? h('span', { class: 'dgt-tags' }, tags) : null;
  }

  /**
   * One COMP demon as a card, for the demon dropdown: who it is, the share of its stats you get, the skills in its
   * slots and the key bonuses it would add ("offer", worked out by the reader with your learned boosts).
   * `pick` runs on a click; `on` marks the demon in use.
   * `compact` (the Overview's summoned demon): no bonus list or unlock tags, not a button, and `tag` after the name.
   */
  function demonTile(d, on, pick, compact, tag, iconFolder) {   // iconFolder: another character's skill icons (the Build Planner's imported base)
    const O = d.offer || null;
    const pic = h('span', { class: 'demon-pic ' + alignClass(O && O.alignment), 'aria-hidden': 'true' }, h('span', { text: String(d.name || '?').charAt(0) }));
    if (d.icon) pic.append(h('img', { src: d.icon, alt: '' }));
    mitamaMark(pic, d.mitama);
    const slots = O ? (O.skills || []).filter((s) => s.kind !== 'Feature' && s.kind !== 'Equipment') : [];
    const icons = slots.map((s) => {
      const ic = skillIcon(s, iconFolder);
      // dimmed = you don't get it while digitalized (the race's boost isn't learned): no meaning for the summoned demon, which uses all its skills
      if (!compact && !isTrue(s.counted)) ic.classList.add('off');
      if (compact) { ic.removeAttribute('aria-hidden'); ic.tabIndex = 0; ic.setAttribute('role', 'img'); ic.setAttribute('aria-label', s.name); }
      return withTip(ic, () => h('div', null, tipHead(skillIcon(s, iconFolder), s.name, s.kind), ...tipLines(s.tip), skillWhy(s)));
    });
    const adds = compact ? [] : O ? COMBAT.filter(([k]) => n0(O.combat && O.combat[k])).map(([k, label, unit]) => h('span', { class: 'dgt-add' }, h('span', { text: label }), h('b', { text: fmt(O.combat[k], unit, true) }))) : [];
    const hp = O && (O.stats || []).find((s) => s.stat === 'Max HP'), mp = O && (O.stats || []).find((s) => s.stat === 'Max MP');
    return h(compact ? 'div' : 'button', compact ? { class: 'dg-tile static' } : { type: 'button', class: 'dg-tile' + (on ? ' on' : ''), 'aria-current': on ? 'true' : null, onclick: pick },
      h('span', { class: 'dgt-head' }, pic,
        h('span', { class: 'dgt-id' }, compact && tag ? h('span', { class: 'dgt-name-line' }, h('span', { class: 'dgt-name', text: d.name }), tag) : h('span', { class: 'dgt-name', text: d.name }),
          h('span', { class: 'dgt-line', text: [`Lv ${d.level}`, d.race, O && O.alignment ? alignName(O.alignment) : null, d.mitama ? `${d.mitama} Mitama` : null].filter(Boolean).join(' · ') })),
        compact ? null : (on ? h('span', { class: 'tag tag-demon', text: 'In use' }) : null)),
      compact ? null : demonUnlockTags(d),
      O ? h('span', { class: 'dgt-line' }, `${fmt(O.rate)}% of its stats`, hp ? ` · HP ${fmt(hp.value, '', true)}` : '', mp ? ` · MP ${fmt(mp.value, '', true)}` : '')
        : d.forceStack || d.equipment ? h('span', { class: 'dgt-line', text: [d.forceStack ? `force stack ${d.forceStack}` : null, d.equipment ? 'equipment' : null].filter(Boolean).join(' · ') }) : null,
      // listed by the game without its details (stats, skills, growth, Mitama): there is nothing to work out yet
      d.loaded === false ? h('span', { class: 'dgt-line', text: 'Not read yet. Click it in the COMP in game, or open the fusion window at the Cathedral of Shadows (that loads your whole COMP), then load the demons again.' }) : null,
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
  // One series' mark in the list, the column heads and the tooltip: its colour, and its dash pattern once the colours run out
  // (six colours, then the same six dashed, then dotted), so two lines are never told apart by colour alone.
  const HIST_COLOURS = 6, HIST_DASH = ['', '6 4', '2 4'];
  const histStyle = (i) => ({ colour: `var(--s${(i % HIST_COLOURS) + 1})`, dash: HIST_DASH[Math.floor(i / HIST_COLOURS) % HIST_DASH.length] });
  function histSwatch(i) {
    const st = histStyle(i), svg = s('svg', { class: 'hist-sw', viewBox: '0 0 20 8', width: 20, height: 8, 'aria-hidden': 'true' });
    svg.append(s('line', { x1: 1, x2: 19, y1: 4, y2: 4, style: `stroke:${st.colour};stroke-width:2.5;stroke-linecap:round;` + (st.dash ? `stroke-dasharray:${st.dash}` : '') }));
    return svg;
  }
  /** Value axis for values between min and max: the finest nice step (1, 2, 2.5 or 5 times a power of ten) that needs no more than
   * `most` intervals, so the top is never far above the highest value (232% gets 0..250, not 0..400). */
  function niceScale(min, max, most) {
    if (min === max) { const pad = Math.abs(min) * 0.1 || 1; min -= pad; max += pad; }
    const mag = Math.pow(10, Math.floor(Math.log10((max - min) / most)));
    for (let k = 0; k < 80; k++) {
      const step = [1, 2, 2.5, 5][k % 4] * mag * Math.pow(10, Math.floor(k / 4)), lo = Math.floor(min / step + 1e-9) * step, hi = Math.ceil(max / step - 1e-9) * step;
      if ((hi - lo) / step <= most + 1e-9) {
        const ticks = []; for (let i = 0; lo + i * step <= hi + step / 2; i++) ticks.push(Number((lo + i * step).toFixed(8)));
        return { lo, hi, ticks };
      }
    }
    return { lo: min, hi: max, ticks: [min, max] };
  }
  /** `series`: [{ label, unit, style, pts: [{ t, v, at }] }], one per ticked stat. Two graphs laid over each other, sharing the
   *  time axis: the absolutes (stats, HP, critical...) with their value axis on the left, which also draws the gridlines, and
   *  the percentages (limit break chance, damage dealt...) with theirs on the right, each fitted to its own lines. Without
   *  absolutes the gridlines come from the percentages' axis.
   *  `snapTimes` ([{ t, at }]) is for the empty graph (nothing ticked): the time axis and gridlines with nothing plotted. */
  function drawHistory(el, series, snapTimes) {
    el.replaceChildren();
    const empty = !series.length;
    const live = series.filter((q) => q.pts.length);
    if (!empty && !live.length) { el.append(h('p', { class: 'note', text: series.length === 1 ? `No snapshot has a value for ${series[0].label} yet.` : 'No snapshot has a value for the stats ticked yet.' })); return; }
    const atOf = new Map(); live.forEach((q) => q.pts.forEach((p) => atOf.set(p.t, p.at)));
    if (empty) snapTimes.forEach((p) => atOf.set(p.t, p.at));
    const times = [...atOf.keys()].sort((a, b) => a - b), ats = times.map((t) => atOf.get(t));
    const axisL = live.filter((q) => q.unit !== '%'), axisR = live.filter((q) => q.unit === '%');
    const W = Math.max(320, el.clientWidth), H = 415, m = { t: 16, r: axisR.length ? 58 : live.length === 1 ? 64 : 16, b: 30, l: axisL.length || empty ? 52 : 16 };
    const t0 = times[0], t1 = times[times.length - 1];
    const x = (t) => (t1 === t0 ? (m.l + W - m.r) / 2 : m.l + ((t - t0) / (t1 - t0)) * (W - m.l - m.r));
    const svg = s('svg', { viewBox: `0 0 ${W} ${H}`, height: H, role: 'img', 'aria-label': empty ? 'Empty graph' : `${series.map((q) => q.label).join(', ')} across ${times.length} snapshots` });
    const yAt = (frac) => H - m.b - frac * (H - m.t - m.b);
    const level = [0, 0.25, 0.5, 0.75, 1];
    // each graph's axis has its own steps (at most 6 intervals); the left one draws the gridlines, or the right one when there is no left
    const fit = (group) => {
      const vs = group.flatMap((q) => q.pts.map((p) => p.v)), lo = Math.min(...vs), hi = Math.max(...vs), sc = niceScale(lo, hi, 6);
      group.forEach((q) => { q.y = (v) => yAt((v - sc.lo) / (sc.hi - sc.lo)); });
      sc.at = (v) => yAt((v - sc.lo) / (sc.hi - sc.lo));
      return sc;
    };
    const text = (extra, str) => Object.assign(s('text', extra), { textContent: str });
    if (empty) {
      level.forEach((f) => svg.append(s('line', { class: f === 0 ? 'axisline' : 'gridline', x1: m.l, x2: W - m.r, y1: yAt(f), y2: yAt(f), style: 'stroke:var(--axis);stroke-opacity:.55' })));
      svg.append(text({ x: (m.l + W - m.r) / 2, y: yAt(0.5), dy: '-0.4em', 'text-anchor': 'middle' }, 'Tick a stat in the list to plot it.'));
    } else {
      const sl = axisL.length ? fit(axisL) : null, sr = axisR.length ? fit(axisR) : null, base = sl || sr;
      base.ticks.forEach((v) => {
        svg.append(s('line', { class: v === base.lo ? 'axisline' : 'gridline', x1: m.l, x2: W - m.r, y1: base.at(v), y2: base.at(v) }));
        if (sl) svg.append(text({ x: m.l - 8, y: base.at(v), dy: '0.32em', 'text-anchor': 'end' }, fmt(v, axisL[0].unit)));
      });
      if (sr) sr.ticks.forEach((v) => svg.append(s('line', { class: 'axisline', x1: W - m.r, x2: W - m.r + 4, y1: sr.at(v), y2: sr.at(v) }),
        text({ x: W - m.r + 8, y: sr.at(v), dy: '0.32em', 'text-anchor': 'start' }, fmt(v, '%'))));
    }
    // x labels: newest always shown; older ones only where they fit
    const oneDay = ats.every((a) => sameDay(a, ats[0]));
    const room = oneDay ? 64 : 104, kept = [times.length - 1];
    if (times.length > 1 && x(t1) - x(t0) >= room) kept.push(0);
    for (let i = times.length - 2; i > 0; i--) if (kept.every((j) => Math.abs(x(times[j]) - x(times[i])) >= room)) kept.push(i);
    kept.forEach((i) => {
      const anchor = times.length === 1 ? 'middle' : i === times.length - 1 ? 'end' : i === 0 ? 'start' : 'middle';
      svg.append(text({ x: x(times[i]), y: H - m.b + 18, 'text-anchor': anchor }, oneDay ? clock(ats[i]) : when(ats[i])));
    });
    live.forEach((q) => {
      if (q.pts.length > 1) svg.append(s('path', { class: 'line', style: `stroke:${q.style.colour};` + (q.style.dash ? `stroke-dasharray:${q.style.dash}` : ''),
        d: q.pts.map((p, i) => `${i ? 'L' : 'M'}${x(p.t).toFixed(1)},${q.y(p.v).toFixed(1)}`).join('') }));
    });
    const cross = s('line', { class: 'cross', y1: m.t, y2: H - m.b, visibility: 'hidden' });
    svg.append(cross);
    const multi = series.length > 1, r0 = multi ? 4 : 5;
    const dots = live.flatMap((q) => q.pts.map((p) => Object.assign(s('circle', { class: 'dot', style: `fill:${q.style.colour}`, cx: x(p.t), cy: q.y(p.v), r: r0 }), { _t: p.t })));
    svg.append(...dots);
    if (!multi && !empty && !axisR.length) {
      const q = live[0], last = q.pts[q.pts.length - 1];
      svg.append(text({ class: 'endlabel', x: x(last.t) + 10, y: q.y(last.v), dy: '0.32em' }, fmt(last.v, q.unit)));
    }
    const hit = s('rect', { x: m.l - 10, y: m.t, width: W - m.l - m.r + 20, height: H - m.t - m.b, fill: 'transparent' });
    hit.addEventListener('pointermove', (ev) => {
      const box = svg.getBoundingClientRect(), mx = ((ev.clientX - box.left) / box.width) * W;
      let best = 0; times.forEach((t, i) => { if (Math.abs(x(t) - mx) < Math.abs(x(times[best]) - mx)) best = i; });
      const t = times[best];
      cross.setAttribute('x1', x(t)); cross.setAttribute('x2', x(t)); cross.setAttribute('visibility', 'visible');
      dots.forEach((d) => d.setAttribute('r', d._t === t ? r0 + 1.5 : r0));
      if (!multi) { const p = live[0].pts.find((q) => q.t === t); showTip(`${when(ats[best])} · ${series[0].label} ${fmt(p.v, series[0].unit)}`, ev.clientX, ev.clientY); return; }
      showTip(h('div', { class: 'hist-tip' }, h('div', { class: 'tt-title', text: when(ats[best]) }), series.map((q) => {
        const p = q.pts.find((z) => z.t === t);
        return h('div', { class: 'hist-tip-row' }, histSwatch(q.slot), h('span', { text: q.label }), h('b', { class: 'num', text: p ? fmt(p.v, q.unit) : '—' }));
      })), ev.clientX, ev.clientY);
    });
    hit.addEventListener('pointerleave', () => { cross.setAttribute('visibility', 'hidden'); dots.forEach((d) => d.setAttribute('r', r0)); hideTip(); });
    if (!empty) svg.append(hit);
    el.append(svg);
  }

  /** Delete one earlier snapshot from the History table (its spreadsheet row; the app copies the spreadsheet first). */
  async function deleteHistorySnapshot(x) {
    const H = S.hist;
    if (H.deleting) return;
    H.deleting = true; render();
    try {
      const r = await host.call('deleteSnapshot', { takenAt: x.taken_at });
      if (S.compare === x.taken_at) S.compare = null;
      H.asking = null;
      await reload();
      toast('ok', `Snapshot of ${when(x.taken_at)} deleted`, r && r.backup ? `The spreadsheet was copied first: ${r.backup}` : '');
    } catch (e) {
      toast('err', 'Couldn’t delete the snapshot', String(e && e.message || e), 0);
    } finally {
      H.deleting = false; render();
    }
  }

  function pageHistory() {
    const { snaps } = S.v, H = S.hist;
    const page = h('div', { class: 'page hist-page' });
    const groups = [
      ['Stats', [...STATS.map(([k, label]) => [k, label, '', false]), ['max_hp', 'Max HP', '', false], ['max_mp', 'Max MP', '', false]]],
      ['Gear bonuses', COMBAT.map(([k, label, unit]) => [k, label, unit, true])],
    ];
    const flat = groups.flatMap(([, list]) => list);
    H.keys = H.keys.filter((k) => flat.some(([f]) => f === k));
    // every ticked stat keeps its colour while it stays ticked: the lowest free one when it is ticked
    Object.keys(H.slot).forEach((k) => { if (!H.keys.includes(k)) delete H.slot[k]; });
    H.keys.forEach((k) => { if (H.slot[k] === undefined) { const used = new Set(Object.values(H.slot)); let i = 0; while (used.has(i)) i++; H.slot[k] = i; } });
    const gearLabel = digi() ? ' (gear, digitalized)' : ' (gear)';
    const series = flat.filter(([k]) => H.keys.includes(k)).map(([k, label, unit, gear]) => {
      const field = gear && digi() ? k + '_digi' : k;
      return { key: k, field, label, unit, full: label + (gear ? gearLabel : ''), slot: H.slot[k], style: histStyle(H.slot[k]),
        pts: snaps.map((r) => ({ t: date(r.taken_at).getTime(), v: num(r[field]), at: r.taken_at })).filter((p) => p.v !== null && !Number.isNaN(p.t)) };
    }).sort((a, b) => a.slot - b.slot);
    const multi = series.length > 1;
    const latest = snaps[snaps.length - 1];

    page.append(pageHead('One point per snapshot, so gear swaps show up as steps.',
      host.native ? h('button', { type: 'button', class: 'btn', onclick: () => host.call('reveal', { what: 'csv' }) }, 'Show spreadsheet') : null));

    // the list: one tickbox per stat, as tall as the graph (the card is two columns; the list fills the second and scrolls)
    const list = h('div', { class: 'hist-pick-in' });
    list.append(h('div', { class: 'hist-pick-head' }, h('span', { text: H.keys.length ? `${H.keys.length} on the graph` : 'Nothing on the graph' }),
      H.keys.length ? h('button', { type: 'button', class: 'btn btn-ghost btn-sm', onclick: () => { H.keys = []; H.top = 0; store.set('hist', ''); render(); } }, 'Clear') : null));
    groups.forEach(([group, items]) => {
      list.append(h('h3', { class: 'hist-group', text: group }));
      items.forEach(([k, label, unit, gear]) => {
        const on = H.keys.includes(k), v = latest ? num(latest[gear && digi() ? k + '_digi' : k]) : null;
        list.append(h('label', { class: 'hist-opt' + (on ? ' on' : '') },
          h('input', { type: 'checkbox', 'data-key': k, checked: on, style: on ? `accent-color:${histStyle(H.slot[k]).colour}` : null,
            onchange: (ev) => {
              H.keys = ev.target.checked ? [...H.keys, k] : H.keys.filter((x) => x !== k);
              H.top = list.scrollTop; H.focus = k; store.set('hist', H.keys.join(',')); render();
            } }),
          h('span', { class: 'hist-label' }, h('span', { class: 'hist-name', text: label }), on ? histSwatch(H.slot[k]) : null),
          h('span', { class: 'hist-val num', text: v === null ? '—' : fmt(v, unit) })));
      });
    });
    const chart = h('div', { class: 'chart' });
    page.append(sect(multi ? 'Selected stats' : series.length ? series[0].full : 'History', null, null,
      h('div', { class: 'hist-row' }, h('section', { class: 'card' }, chart), h('section', { class: 'card hist-pick', role: 'group', 'aria-label': 'Stats on the graph' }, list))));
    requestAnimationFrame(() => {
      drawHistory(chart, series, snaps.map((r) => ({ t: date(r.taken_at).getTime(), at: r.taken_at })).filter((p) => !Number.isNaN(p.t)));
      list.scrollTop = H.top || 0;
      if (H.focus) { const el = list.querySelector(`input[data-key="${CSS.escape(H.focus)}"]`); if (el) el.focus({ preventScroll: true }); H.focus = null; }
    });
    // two kinds of stat ticked: say which axis is which
    if (series.some((q) => q.unit === '%') && series.some((q) => q.unit !== '%')) page.append(h('p', { class: 'note', text: 'Stats counted in percent (chance, damage dealt, speed...) use the right axis; the others (stats, HP, critical...) use the left one. Each axis fits the lines on it.' }));

    // the table: every snapshot, newest first; one column per stat on the graph
    const desc = snaps.slice().reverse();
    const head = (text, cls) => h('th', { scope: 'col', class: cls, text });
    const th = [head('Snapshot'), head('Stats read'), head('Level', 'num'), head('Max HP', 'num')];
    series.forEach((q) => { th.push(h('th', { scope: 'col', class: 'num' }, multi ? [histSwatch(q.slot), ' '] : null, q.label)); if (!multi) th.push(head('Change', 'num')); });
    th.push(h('th', { scope: 'col', class: 'hist-act' }, h('span', { class: 'sr-only', text: 'Delete' })));
    page.append(h('div', { class: 'tbl-wrap' }, h('div', { class: 'tbl-scroll' }, h('table', { class: 'tbl hist-tbl' },
      h('thead', null, h('tr', null, th)),
      h('tbody', null, desc.map((r, i) => {
        const prev = desc[i + 1], tds = [
          h('td', null, when(r.taken_at), i === 0 ? h('span', { class: 'tag', style: 'margin-left:8px', text: 'Latest' }) : null),
          h('td', { class: 'dim', text: isTrue(r.stats_live) ? 'Live' : `Saved, from ${sameDay(r.stats_from, r.taken_at) ? clock(r.stats_from) : when(r.stats_from)}` }),
          h('td', { class: 'num', text: fmt(r.level) }), h('td', { class: 'num', text: fmt(r.max_hp) })];
        series.forEach((q) => {
          const a = prev ? num(prev[q.field]) : null, b = num(r[q.field]);
          const d = a !== null && b !== null ? round2(b - a) : null;
          const good = d !== null && (lowIsGood(q.label) ? d < 0 : d > 0);
          const delta = d ? h('span', { class: 'delta ' + (good ? 'good' : 'bad'), text: (d > 0 ? '▲ ' : '▼ ') + fmt(d, q.unit, true) }) : h('span', { class: 'zero', text: d === 0 ? 'same' : '—' });
          if (multi) tds.push(h('td', { class: 'num' }, h('span', { class: 'emph', text: fmt(b, q.unit) }), h('span', { class: 'hist-delta' }, delta)));
          else tds.push(h('td', { class: 'num emph', text: fmt(b, q.unit) }), h('td', { class: 'num' }, delta));
        });
        // the latest can't be deleted (every page shows it); an earlier one asks first, in its own row
        const ask = H.asking === r.taken_at;
        tds.push(h('td', { class: 'hist-act' }, i === 0 ? null : ask
          ? h('span', { class: 'hist-ask' }, h('span', { class: 'dim', text: 'Delete this snapshot?' }),
            h('button', { type: 'button', class: 'btn btn-sm', 'data-tip': 'The spreadsheet is copied first', disabled: H.deleting ? '' : null, onclick: () => deleteHistorySnapshot(r) }, H.deleting ? 'Deleting…' : 'Delete'),
            h('button', { type: 'button', class: 'btn btn-ghost btn-sm', disabled: H.deleting ? '' : null, onclick: () => { H.asking = null; render(); } }, 'Keep'))
          : h('button', { type: 'button', class: 'btn btn-ghost btn-square btn-sm', 'aria-label': `Delete the snapshot of ${when(r.taken_at)}`, 'data-tip': 'Delete this snapshot. The spreadsheet is copied first.',
            onclick: () => { H.asking = r.taken_at; render(); } }, icon('trash'))));
        return h('tr', { class: ask ? 'hist-asking' : null }, tds);
      }))))));
    page.append(h('p', { class: 'note', text: `${snaps.length} snapshot${snaps.length === 1 ? '' : 's'}. Gear bonus columns started with the second snapshot, so the first one has no value for them.` }));
    return page;
  }

  // ---------- render ----------
  // ---------- Tools: DCM Locator ----------
  // dcm-sources.json is made by tools\dcmexport from the game data; dcm-notes.json is written by hand (what players
  // found on IMAGINE itself). Both sit beside this file.
  const DCM_KINDS = [['all', 'All'], ['shop', 'Shop'], ['drop', 'Enemy drop'], ['chest', 'Chest or loot'], ['event', 'Event or NPC'], ['barter', 'NPC exchange'], ['present', 'Present only'], ['none', 'No known source']];
  const pct = (v) => `${nf.format(n0(v))}%`;
  function loadDcm() {
    if (S.dcm.state !== 'idle') return;
    S.dcm.state = 'loading';
    const get = (p) => fetch(p, { cache: 'no-store' }).then((r) => (r.ok ? r.json() : null)).catch(() => null);
    Promise.all([get('dcm-sources.json'), get('dcm-notes.json')]).then(([d, n]) => {
      S.dcm.data = d; S.dcm.notes = (n && n.notes) || {};
      S.dcm.state = d ? 'ready' : 'failed';
      if (d) d.items.forEach((it) => { it.lines = dcmLines(it); it.kinds = new Set(it.lines.map((l) => l.group)); it.hay = [it.demon, ...it.lines.map((l) => [l.text, l.sub || '', ...(l.tags || [])].join(' '))].join(' ').toLowerCase(); });
      if (S.page === 'dcm' || S.page === 'rebirth') render();   // the Rebirth Planner asks it which demons a DCM gives
    });
  }
  /** One DCM's sources as display lines, best first: { group, kind, text, sub, value, tags }. */
  function dcmLines(it) {
    const lines = [], by = (k) => it.sources.filter((s) => s.kind === k);
    const zones = (list) => { const z = [...new Set(list.map((s) => s.zone).filter(Boolean))]; return z.length > 6 ? z.slice(0, 6).join(', ') + ` and ${z.length - 6} more` : z.join(', '); };
    const grouped = (list, key) => { const m = new Map(); list.forEach((s) => { const k = key(s); if (!m.has(k)) m.set(k, []); m.get(k).push(s); }); return [...m.values()]; };
    (S.dcm.notes[it.id] || []).forEach((t) => lines.push({ group: 'note', kind: 'IMAGINE note', text: String(t), rank: 0 }));
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
    // NPC exchanges (the client's own barter lists): one line per NPC and price, naming every zone that NPC trades it in.
    // After events so a drop, chest or shop stays the best line; before random boxes and presents.
    grouped(by('barter'), (s) => s.npc + '|' + s.cost + '|' + (s.npc ? '' : s.zone)).forEach((g) => { const z = zones(g);
      lines.push({ group: 'barter', kind: 'Exchange', text: g[0].npc ? `Talk to ${g[0].npc}` + (z ? ' in ' + z : '') : z ? `An NPC in ${z} (the data does not say who)` : 'An exchange NPC the data does not place',
        sub: g[0].cost ? 'for ' + g[0].cost : 'The data does not say what it costs', rank: 4.5 }); });
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
    page.append(h('div', { class: 'notice' }, icon('info'), h('span', null, h('b', { text: 'Read from game data, not checked on IMAGINE. ' }),
      'The DCM list, zone names, shop stock and NPC exchanges are the IMAGINE client\'s. Drops, chests, presents, events and which NPC offers an exchange come from the public server data, so IMAGINE may have changed them. Lines marked “IMAGINE note” are confirmed by players.')));
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
            : h('p', { class: 'note', text: 'Nothing in the data gives this DCM. It is most likely an IMAGINE addition whose source only the live server knows.' }),
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

  // ---------- Tools: Quest Database (and the Quest Tracker below, which uses the same data) ----------
  // quest-guide.json is made by tools\questexport: the game's quest tables for the text and needs, the server's event
  // files for the steps (who to talk to, where, what to pick, what you get). It sits beside this file.
  const QUEST_TYPES = [['available', 'Available'], ['Normal', 'Normal'], ['Repeatable', 'Repeatable'], ['Event', 'Event'], ['I-Time', 'I-Time'], ['Episode', 'Episode'], ['Inactive', 'Inactive']];
  function loadQuests() {
    if (S.quest.state !== 'idle') return;
    S.quest.state = 'loading';
    // the people's pictures (npc-pictures.json beside this file); the names show without them until it loads
    fetch('npc-pictures.json').then((r) => (r.ok ? r.json() : null)).catch(() => null).then((f) => { S.quest.faces = f; if (f && S.quest.data && (S.page === 'quests' || S.page === 'tracker')) render(); });
    const json = (f) => fetch(f, { cache: 'no-store' }).then((r) => (r.ok ? r.json() : null)).catch(() => null);
    // quest-tl.json: BABEL's English for the Japanese the quest data still has (answers, event names, item and valuable
    // names inside a line); without it the guide shows the game's Japanese
    Promise.all([json('quest-guide.json'), json('quest-tl.json')]).then(([d, tl]) => {
      if (d && tl) S.quest.tl = questTranslate(d, tl);
      S.quest.data = d; S.quest.state = d ? 'ready' : 'failed';
      if (d) d.quests.forEach((q) => {
        const words = [q.title, q.client, q.zone, ...(q.seasonal || [])];
        q.steps.forEach((s) => {
          words.push(s.objective || '', ...(s.goals || []), ...(s.optional || []).flatMap((x) => [x.who, x.where]));
          (s.ways || []).forEach((w) => words.push(w.do, w.who || '', w.where || '', ...(w.talks || []).flatMap((t) => [t.who || '', t.where || '']), ...(w.get || []), ...(w.lose || []), ...(w.needs || [])));
        });
        q.hay = words.join(' ').toLowerCase();
      });
      if (S.page === 'quests' || S.page === 'tracker') render();
    });
  }
  /**
   * Puts BABEL's English into the quest data in place (quest-tl.json: `text` = whole strings, `names` = item and valuable
   * names inside a line, both Japanese -> English) and returns English -> the game's Japanese for every string it changed.
   * Object keys are translated too (`answerFx` is keyed by the answer). The page greys those strings (`questTlMark`).
   */
  function questTranslate(d, tl) {
    const text = tl.text || {}, names = Object.keys(tl.names || {}).sort((a, b) => b.length - a.length), back = new Map();
    const one = (s) => {
      if (typeof s !== 'string' || !/[぀-ヿ㐀-鿿]/.test(s)) return s;
      let t = text[s];
      if (t === undefined) { t = s; for (const n of names) if (t.includes(n)) t = t.split(n).join(tl.names[n]); }
      if (t !== s) back.set(t, s);
      return t;
    };
    const walk = (o) => {
      if (Array.isArray(o)) { for (let i = 0; i < o.length; i++) o[i] = typeof o[i] === 'string' ? one(o[i]) : walk(o[i]); return o; }
      if (!o || typeof o !== 'object') return o;
      for (const k of Object.keys(o)) {
        const v = typeof o[k] === 'string' ? one(o[k]) : walk(o[k]), k2 = one(k);
        if (k2 !== k) delete o[k];
        o[k2] = v;
      }
      return o;
    };
    walk(d);
    return back;
  }
  /** Greys the quest lines that are BABEL's translation (S.quest.tl), with the game's Japanese in the tooltip: any element
   *  whose whole text is such a line. Runs on what the page adds, so every way a line is drawn is covered. */
  function questTlMark(root) {
    const tl = S.quest.tl;
    if (!tl || !tl.size || root.nodeType !== 1) return;
    const GAME_TEXT = '.q-pick, .q-said, .q-asks, .q-goals > li, .q-item';
    for (const el of [...(root.matches(GAME_TEXT) ? [root] : []), ...root.querySelectorAll(GAME_TEXT)]) {
      if (el.childNodes.length !== 1 || el.firstChild.nodeType !== 3 || el.classList.contains('tl-name')) continue;
      const ja = tl.get(el.textContent);
      if (ja === undefined) continue;
      el.classList.add('tl-name');
      if (!el.hasAttribute('data-tip')) el.setAttribute('data-tip', `BABEL's translation: the game has this only in Japanese. In game: ${ja}`);
    }
  }
  new MutationObserver((muts) => { if (S.quest && S.quest.tl && (S.page === 'quests' || S.page === 'tracker')) muts.forEach((m) => m.addedNodes.forEach(questTlMark)); }).observe(document.body, { childList: true, subtree: true });
  /**
   * The ways of doing a step, as groups: ways that ask the same and give the same differ only in who you talk to
   * (three NPCs who all take the report), so they become one instruction naming all of them.
   */
  function questWays(ways, q) {
    const groups = new Map();
    ways.forEach((w0) => {
      const w = Object.assign({}, w0, { needs: (w0.needs || []).filter((n) => !q.needs.includes(n)) });
      const key = JSON.stringify([w.kind === 'npc' && w.who ? 'talk' : w.do + w.where + (w.grid || ''), w.talks, w.pick, w.also, w.others, w.needs, w.fights, w.get, w.lose, w.effects]);
      if (!groups.has(key)) groups.set(key, []);
      groups.get(key).push(w);
    });
    return [...groups.values()];
  }
  /**
   * The answers to give, one line per question the person asks: the answer to pick, other answers that work as well
   * ("or"), and the answers that do not move the quest on ("not"). Null when there is nothing to choose.
   */
  /** The answer to question i of a way, with the others that work as well, and the ones that do not move the quest on (and
   *  what those do to your character when they change your alignment). */
  function questAnswer(x, i, p, skip) {
    const also = (x.also && x.also[i]) || [], not = (x.others && x.others[i]) || [], fx = x.answerFx || {};
    // answers that take the quest another way: only those that change your alignment are worth a warning (the others are
    // shown as their own way, or are menus and repeats); in a tree its own branches are left out
    const other = ((x.elsewhere && x.elsewhere[i]) || []).filter((t) => fx[t] && !(skip && skip.has(t)));
    const said = (t) => ['“', h('span', { class: 'q-said', text: t }), '”', fx[t] ? h('span', { class: 'q-fx', text: ` (${fx[t].map((f) => f[0].toLowerCase() + f.slice(1)).join('; ')})` }) : null];
    // a flat list: h() flattens one level only
    return [...[p, ...also].flatMap((t, j) => [j ? h('span', { class: 'q-arrow', text: 'or' }) : null, h('span', { class: 'q-pick', text: t })]),
      not.length ? h('span', { class: 'q-not' }, 'not ', not.flatMap((t, j) => [j ? ' or ' : '', ...said(t)])) : null,
      other.length ? h('span', { class: 'q-not' }, 'other answers: ', other.flatMap((t, j) => [j ? ', ' : '', ...said(t)])) : null];
  }
  function questAnswers(x) {
    if (!x.pick || !x.pick.length) return null;
    const lines = x.pick.map((p, i) => h('div', { class: 'q-answer' }, i ? h('span', { class: 'q-arrow', text: 'then' }) : null, ...questAnswer(x, i, p)));
    return h('div', { class: 'q-line' }, h('span', { class: 'q-key', text: 'Choose' }), h('div', { class: 'q-answers' }, lines,
      x.oldText ? h('div', { class: 'q-hint', text: 'Worded as before the game update of 19 November; the game may word these answers differently now (often in Japanese).' }) : null));
  }
  /** A person's name, with their picture before it when there is one (npc-pictures.json: the NPC's hNPCData id -> the
   *  picture: their own model's, or the Demon Database's for one drawn as a demon). */
  const questWho = (x) => {
    const src = x.npc != null && S.quest.faces ? S.quest.faces[x.npc] : null;
    return src ? h('span', { class: 'q-who' }, h('img', { class: 'q-face', src, alt: '', onerror: (e) => e.target.remove() }), h('b', { text: x.who })) : h('b', { text: x.who });
  };
  const questAt = (x) => (x.where ? (x.kind === 'zone' ? ` to ${x.where}` : ` in ${x.where}`) : '') + (x.grid ? ` at (${x.grid})` : '');
  const questRow = (label, list, cls) => (list && list.length ? h('div', { class: 'q-line' + (cls ? ' ' + cls : '') }, h('span', { class: 'q-key', text: label }), h('span', null, list.flatMap((t, i) => [i ? ', ' : '', h('span', { class: 'q-item', text: t })]))) : null);
  /** What you need, fight, get, hand over and what changes for a way. What changes is one line each: an alignment change
   *  can depend on your alignment ("If your alignment is …"). */
  const questRows = (w) => [questRow('You must', w.needs), questRow('You fight', w.fights), questRow('You receive', w.get, 'q-get'), questRow('You hand over', w.lose),
    w.effects && w.effects.length ? h('div', { class: 'q-line q-after' }, h('span', { class: 'q-key', text: 'Afterwards' }), h('div', { class: 'q-lines' }, w.effects.map((e) => h('div', { text: e })))) : null];
  /**
   * One person whose answers lead to different outcomes (Uriel in Act 13: four ways, each moving your alignment
   * somewhere else), as one tree of answers: the answers all ways share first, then at each question every answer
   * that leads on, each with what it leads to.
   */
  function questTree(groups, alt) {
    const ways = groups.map((g) => g[0]), w = ways[0];
    const root = { kids: new Map(), leaves: [] };
    ways.forEach((x) => {
      let n = root;
      x.pick.forEach((p, i) => {
        if (!n.kids.has(p)) n.kids.set(p, { kids: new Map(), leaves: [], way: x, depth: i });
        n = n.kids.get(p);
      });
      n.leaves.push(x);
    });
    // a run of single answers: "A" then "B" then ...; `sib` = the answers this tree already shows at that question
    const run = (p, k, first, sib) => {
      const line = [first ? null : h('span', { class: 'q-arrow', text: 'then' }), ...questAnswer(k.way, k.depth, p, sib)];
      let c = k;
      while (c.kids.size === 1 && !c.leaves.length) {
        const [p2, k2] = [...c.kids][0];
        line.push(h('span', { class: 'q-arrow', text: 'then' }), ...questAnswer(k2.way, k2.depth, p2, new Set(c.kids.keys())));
        c = k2;
      }
      return { line, end: c };
    };
    const outcome = (x) => {
      const rows = questRows(x).filter(Boolean);
      return h('div', { class: 'q-outcome' }, rows.length ? rows : h('div', { class: 'q-hint', text: 'Nothing else changes.' }));
    };
    const branch = (n) => h('ul', { class: 'q-tree' }, [...n.kids].map(([p, k]) => {
      const { line, end } = run(p, k, true, new Set(n.kids.keys()));
      return h('li', null, h('div', { class: 'q-answer' }, ...line), end.leaves.map(outcome), end.kids.size ? branch(end) : null);
    }));
    let start = root, lead = null;
    if (root.kids.size === 1 && !root.leaves.length) {
      const [p, k] = [...root.kids][0];
      const r = run(p, k, true, new Set(root.kids.keys()));
      if (r.end.kids.size) { lead = r.line; start = r.end; }
    }
    return h('div', { class: 'q-way' },
      h('div', { class: 'q-do' }, alt ? h('span', { class: 'q-or', text: 'or' }) : null,
        h('span', null, ['Talk to ', questWho(w), questAt(w), '. Your answers decide what happens:'])),
      w.dungeon ? h('div', { class: 'q-hint', text: w.dungeon }) : null,
      w.hint ? h('div', { class: 'q-hint', text: w.hint }) : null,
      lead ? h('div', { class: 'q-line' }, h('span', { class: 'q-key', text: 'Choose' }), h('div', { class: 'q-answer' }, ...lead)) : null,
      h('div', { class: 'q-line' }, h('span', { class: 'q-key', text: lead ? 'Then' : 'Choose' }), branch(start)));
  }
  /** One instruction: what to do and where as a sentence, then what to choose, what you need, and what changes hands. */
  /** What has to happen first (getting there, the person or object appearing): "first …", "then …" lines. */
  function questBefore(list) {
    const at = questAt;
    return (list || []).map((b, i) => h('li', null,
      h('div', { class: 'q-do' }, h('span', { class: 'q-or', text: i ? 'then' : 'first' }), h('span', null,
        b.kind === 'npc' && b.who ? ['Talk to ', questWho(b), at(b), '.'] : [h('b', { text: b.do || (b.kind === 'spot' ? 'Walk to the spot' : 'Examine the quest object') }), at(b), '.'], hereTag(b))),
      b.asks ? h('div', { class: 'q-line' }, h('span', { class: 'q-key', text: 'It asks' }), h('span', { class: 'q-asks', text: b.asks })) : null,
      questAnswers(b), questRow('You fight', b.fights), b.result ? h('div', { class: 'q-hint', text: b.result }) : null));
  }
  /** A talk of a multi-talk step: a person, or a quest object or spot the client gives no name. */
  const questTalkWho = (t) => (t.who ? questWho(t) : h('b', { text: t.do || 'Use the quest object' }));
  function questWay(ws, alt) {
    const w = ws[0];
    const at = questAt;
    let sentence;
    // what has to happen first (getting there, the person appearing): "First ... Then ..." before the step's own talk
    const before = questBefore(w.before);
    if (w.talks) sentence = [h('b', { text: w.do }), ':'];   // the people follow as a list
    else if (w.who) {
      const seen = new Set(), people = ws.filter((x) => { const k = x.who + at(x); return !seen.has(k) && seen.add(k); });
      sentence = ['Talk to ', people.map((x, i) => [i ? (i === people.length - 1 ? ' or ' : ', ') : '', questWho(x), at(x)]), '.'];
    } else sentence = [h('b', { text: w.do }), w.kind === 'event' ? '' : at(w), '.'];
    return h('div', { class: 'q-way' },
      before.length ? h('ol', { class: 'q-before' }, before) : null,
      h('div', { class: 'q-do' }, alt ? h('span', { class: 'q-or', text: 'or' }) : null, before.length ? h('span', { class: 'q-or', text: 'then' }) : null, h('span', null, sentence), hereTag(w)),
      w.talks ? h('ol', { class: 'q-talks' }, w.talks.map((t) => {
        // done once the game holds the hidden flag this talk sets (live)
        const did = S.page === 'tracker' && t.flag && S.quest.live && S.quest.live.flags.has(t.flag);
        const tb = did ? [] : questBefore(t.before);
        return h('li', { class: did ? 'q-talk-done' : null },
          // the walkthrough's own way comes after all the others ("then the last one")
          h('div', null, t.last ? [h('span', { class: 'q-or', text: 'last' }), ' '] : null, questTalkWho(t), at(t),
            t.times ? h('span', { class: 'tag', text: `${t.times} times in all` }) : null,
            did ? h('span', { class: 'tag q-done-tag', text: 'Done' }) : null, did ? null : hereTag(t)),
          !did ? questRow('You fight', t.fights) : null,
          tb.length ? h('ol', { class: 'q-before' }, tb) : null,
          !did && t.dungeon ? h('div', { class: 'q-hint', text: t.dungeon }) : null,
          !did && t.asks ? h('div', { class: 'q-line' }, h('span', { class: 'q-key', text: 'It asks' }), h('span', { class: 'q-asks', text: t.asks })) : null,
          questAnswers(t));
      })) : null,
      // parts counted on the quest's own counter (no hidden flag): the server keeps that counter, the game client never
      // gets it (comp_hack sends only the step and the kill counters), so the Tracker cannot tick them off
      S.page === 'tracker' && w.talks && w.talks.some((t) => !t.flag && !t.last)
        ? h('div', { class: 'q-hint', text: 'The game doesn\'t tell the client which of these you have done, so they are not ticked off here.' }) : null,
      w.dungeon ? h('div', { class: 'q-hint', text: w.dungeon }) : null,
      w.hint ? h('div', { class: 'q-hint', text: w.hint }) : null,
      w.times ? h('div', { class: 'q-hint', text: `Do this ${w.times}: the game counts each time.` }) : null,
      w.random ? h('div', { class: 'q-hint', text: 'The game picks this one at random.' }) : null,
      w.asks ? h('div', { class: 'q-line' }, h('span', { class: 'q-key', text: 'It asks' }), h('span', { class: 'q-asks', text: w.asks })) : null,
      questAnswers(w),
      questRows(w),
      w.finishes ? h('div', { class: 'q-hint', text: 'This finishes the quest.' }) : null);
  }
  /** A step's ways as blocks: one person's ways whose answers differ become one tree, the rest stay one block each. */
  /** Several ways the game picks one of at random from the same talk (Let's Trade!: the Demon Buster asks for one of five
   *  items): one block, the talk once, then what each pick gives and takes. */
  function questRandom(groups, alt) {
    const w = groups[0][0];
    return h('div', { class: 'q-way' },
      h('div', { class: 'q-do' }, alt ? h('span', { class: 'q-or', text: 'or' }) : null,
        h('span', null, w.who ? ['Talk to ', questWho(w), questAt(w), '.'] : [h('b', { text: w.do }), questAt(w), '.']), hereTag(w)),
      w.dungeon ? h('div', { class: 'q-hint', text: w.dungeon }) : null,
      questAnswers(w),
      h('div', { class: 'q-line' }, h('span', { class: 'q-key', text: 'At random' }),
        h('div', { class: 'q-lines' }, h('div', { class: 'q-hint', text: `The game picks one of these ${groups.length}:` }),
          h('ol', { class: 'q-talks' }, groups.map((g) => h('li', null, questRows(g[0]).filter(Boolean).length ? questRows(g[0]) : h('span', { class: 'q-hint', text: 'Nothing changes hands.' })))))));
  }
  function questBlocks(s, q) {
    const blocks = [], byPerson = new Map();
    // random picks from one talk become one block (before the per-person grouping below)
    const rnd = new Map(), rest = [];
    questWays(s.ways || [], q).forEach((g) => {
      const w = g[0], k = w.random && !w.talks && !w.finishes ? [w.kind, w.who || w.do, w.where, JSON.stringify(w.pick || [])].join('|') : null;
      if (k) { if (!rnd.has(k)) rnd.set(k, []); rnd.get(k).push(g); } else rest.push(g);
    });
    const randomBlocks = [...rnd.values()].filter((gs) => gs.length > 1);
    [...rnd.values()].filter((gs) => gs.length === 1).forEach((gs) => rest.push(gs[0]));
    rest.forEach((g) => {
      const w = g[0], k = w.who && w.pick && w.pick.length && !w.talks ? [w.kind, w.who, w.where, w.grid || ''].join('|') : null;
      if (k && byPerson.has(k)) byPerson.get(k).push(g);
      else { const b = [g]; if (k) byPerson.set(k, b); blocks.push(b); }
    });
    const out = [];
    randomBlocks.forEach((gs) => out.push(questRandom(gs, out.length > 0)));
    blocks.forEach((b) => {
      if (b.length > 1 && new Set(b.map((g) => JSON.stringify(g[0].pick))).size === b.length) out.push(questTree(b, out.length > 0));
      else b.forEach((g) => out.push(questWay(g, out.length > 0)));
    });
    // people the game shows only during this step whom the ways above do not need (Act 6's "???" in Shibuya)
    if (s.optional && s.optional.length) out.push(h('div', { class: 'q-way q-optional' },
      h('div', { class: 'q-line' }, h('span', { class: 'q-key', text: s.optional.every((x) => x.optional) ? 'Optional' : 'Also here' }),
        h('div', { class: 'q-lines' }, s.optional.map((x) => h('div', null,
          h('div', null, 'Talk to ', questWho(x), questAt(x), '.', hereTag(x)),
          x.says && x.says.length ? h('div', { class: 'q-line' }, h('span', { class: 'q-key', text: 'Says' }), h('div', { class: 'q-lines' }, x.says.map((l) => h('div', { class: 'q-asks', text: l })))) : null,
          x.note ? h('div', { class: 'q-hint', text: `Shown only during this step. ${x.note}` }) : null))))));
    return out;
  }
  // ---- live progress: the character's quests as the game has them (page command questProgress), read every few
  // seconds while the Quest Tracker is open. In progress: the quest's step (0-based, the `phase` of the guide's steps) and
  // its counters; completed: only once the game has the list (Quest Completion Bonuses opened); flags: valuables held,
  // the hidden quest flags among them (a talk of a multi-talk step is done once its flag is held). ----
  const questLive = (q) => (S.quest.live ? S.quest.live.active.get(q.id) || null : null);
  const questDone = (q) => !!(S.quest.live && S.quest.live.completed && S.quest.live.completed.has(q.id));
  // the zone the character is in (live), and whether a place of the guide is it
  // (only the Quest Tracker marks places and talks with what the game says; the Quest Database stays a plain guide)
  const questHere = (x) => S.page === 'tracker' && !!(x && x.zoneId && S.quest.live && S.quest.live.zone && x.zoneId === S.quest.live.zone);
  const hereTag = (x) => (questHere(x) ? h('span', { class: 'tag q-here-tag', text: 'In your zone' }) : null);
  /** Every place a step sends you: its ways, what comes before them, and the talks of a multi-talk step. */
  const stepPlaces = (s) => [...(s.ways || []).flatMap((w) => [w, ...(w.before || []), ...(w.talks || []).flatMap((t) => [t, ...(t.before || [])])]), ...(s.optional || [])];
  /** A quest in progress whose current step has something to do in the zone you are in. */
  const questHereNow = (q) => {
    const a = questLive(q);
    return !!a && q.steps.some((s) => s.phase === a.step && stepPlaces(s).some(questHere));
  };
  function questLiveTick() {
    const D = S.quest;
    if (D.liveBusy || S.page !== 'tracker' || document.hidden) return;
    D.liveBusy = true;
    Promise.resolve(host.call('questProgress')).catch(() => null).then((r) => {
      D.liveBusy = false;
      const raw = JSON.stringify(r && r.active ? r : null);
      if (raw === D.liveRaw) return;
      D.liveRaw = raw;
      D.live = r && r.active ? { active: new Map(r.active.map((x) => [x.id, x])), completed: r.completed ? new Set(r.completed) : null, flags: new Set(r.flags || []), zone: r.zone || null } : null;
      if (S.page === 'tracker' && D.onLive) D.onLive();
    });
  }
  setInterval(questLiveTick, 5000);
  // ---- the quest panel (panel.html, a small window the app keeps above the game) ----
  // It lists every quest in progress; the ones hidden from it are in localStorage babel-panel-hidden, which the panel
  // reads and writes too (same profile).
  const panelHiddenIds = () => { try { return JSON.parse(store.get('panel-hidden') || '[]'); } catch (e) { return []; } };
  const panelHidden = (q) => panelHiddenIds().includes(q.id);
  function panelHide(q, hide) {
    const ids = panelHiddenIds().filter((id) => id !== q.id);
    if (hide) ids.push(q.id);
    store.set('panel-hidden', JSON.stringify(ids));
  }
  const panelOpen = () => Promise.resolve(host.call('panel', { open: true })).catch(() => null);
  function questDetail(q) {
    const box = h('div', { class: 'q-detail' });
    if (q.needs.length) box.append(h('div', { class: 'q-needs' }, h('span', { class: 'q-key', text: 'Before you can start' }), h('span', null, q.needs.flatMap((t, i) => [i ? ' · ' : '', h('span', { class: 'q-item', text: t })]))));
    if (!q.walkthrough) box.append(h('p', { class: 'note', text: 'The server\'s event files do not cover this quest, so only the game\'s own objective text is shown.' }));
    // a quest started only from a seasonal event's files (the public server has those switched off)
    if (q.seasonal && q.seasonal.length) box.append(h('div', { class: 'q-needs' }, h('span', { class: 'q-key', text: 'Seasonal event' }),
      h('span', null, ...q.seasonal.flatMap((t, i) => [i ? ' · ' : '', h('span', { class: 'q-item', text: t })]), `. Its steps come from that event's files, which the public server keeps switched off, so IMAGINE may not run it.`)));
    const ol = h('ol', { class: 'q-steps' });
    q.steps.forEach((s) => ol.append(h('li', null,
      /^Step /.test(s.label) ? null : h('div', { class: 'q-step-title', text: s.label }),   // the number is already in the circle
      s.objective ? h('p', { class: 'q-objective', text: s.objective }) : null,
      s.goals && s.goals.length ? h('ul', { class: 'q-goals' }, s.goals.map((g) => h('li', { text: g }))) : null,
      questBlocks(s, q))));
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
    page.append(h('div', { class: 'notice' }, icon('info'), h('span', null, h('b', { text: 'Read from game data, not checked on IMAGINE. ' }),
      'Quest names, objectives and kill or collect counts are the game\'s own. The steps (who to talk to, what to pick, rewards) come from the public server\'s event files, so IMAGINE may have changed them. Valuables are shown with the name the game data has, which is Japanese.')));
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
    page.append(h('p', { class: 'note', text: `${quests.length} quests, ${quests.filter((x) => x.walkthrough).length} with steps from the event files. Data made on ${D.data.generatedAt}. Numbers in brackets are the map grid position. Alignment runs from −10,000 (Law) to +10,000 (Chaos): −5,000 or lower is Law, +5,000 or higher is Chaos, Neutral in between.` }));
    drawChips(); draw();
    return page;
  }
  // ---------- Tools: Quest Tracker ----------
  // The quests the character has in progress, read from the game every few seconds while this page is open (page command
  // questProgress, see questLiveTick), each with the step it is at and what to do next, from the same quest-guide.json as
  // the Quest Database. Hiding a quest here hides it in the quest panel too (babel-panel-hidden, same list); which cards
  // are open is kept in tracker-open (id -> open; otherwise open when its step has something to do in your zone).
  function trackerOpenMap() { try { return JSON.parse(store.get('tracker-open') || '{}') || {}; } catch (e) { return {}; } }
  /** One line saying what comes next, for a closed card. */
  function trackerPeek(s) {
    const w = s && (s.ways || [])[0];
    if (!w) return s && s.objective ? s.objective : 'The guide has no instructions for this step.';
    const at = questAt;
    if (w.talks) {
      const left = w.talks.filter((t) => !(t.flag && S.quest.live && S.quest.live.flags.has(t.flag)));
      return `${w.do}: ${left.map((t) => (t.who || t.do) + (t.who || !t.where ? '' : ` in ${t.where}`)).join(', ')}`;
    }
    const first = (w.before || []).find((b) => !(S.quest.live && S.quest.live.zone && S.quest.live.zone === w.zoneId && b.zoneId !== w.zoneId));
    if (first) return `First: ${first.kind === 'npc' && first.who ? `talk to ${first.who}` : first.kind === 'npc' ? 'use the quest object' : first.kind === 'spot' ? 'walk to the spot' : 'examine the quest object'}${at(first)}`;
    return w.who ? `Talk to ${w.who}${at(w)}` : `${w.do}${at(w)}`;
  }
  /** A kill goal of the step you are on, with the game's counter as a bar. */
  function trackerGoal(s, i, g, a) {
    const c = s.goalCount && s.goalCount[i];
    if (!c || c[2] !== 'kill') return h('div', { class: 'qt-goal' }, h('span', { text: g }));
    const have = Math.min(n0(a.counters[c[0]]), c[1]), met = have >= c[1];
    return h('div', { class: 'qt-goal' + (met ? ' met' : '') },
      h('span', { text: g }), h('span', { class: 'qt-count', text: `${have}/${c[1]}${met ? ' ✓' : ''}` }),
      h('span', { class: 'qt-bar', 'aria-hidden': 'true' }, h('span', { style: `width:${Math.round((have / c[1]) * 100)}%` })));
  }
  function pageTracker() {
    const D = S.quest, page = h('div', { class: 'page page-tracker' });
    loadQuests();
    page.append(pageHead('Your quests in progress, read from the game, with the step you are on and what to do next.'));
    if (D.state !== 'ready') {
      page.append(h('p', { class: 'note', text: D.state === 'failed' ? 'The quest list could not be loaded (quest-guide.json is missing beside the page).' : 'Loading the quests…' }));
      return page;
    }
    const quests = D.data.quests, byId = new Map(quests.map((q) => [q.id, q]));
    // zone id -> its name, from the places the guide names
    if (!D.zoneNames) {
      D.zoneNames = new Map();
      quests.forEach((q) => q.steps.forEach((s) => stepPlaces(s).forEach((x) => { if (x.zoneId && x.where && !D.zoneNames.has(x.zoneId)) D.zoneNames.set(x.zoneId, x.where); })));
    }
    const status = h('div', { class: 'qt-status' }), list = h('div', { class: 'qt-list' });
    let showHidden = false;
    const drawStatus = () => {
      const L = D.live;
      const facts = !L ? [] : [
        h('span', { class: 'qt-fact' }, h('b', { text: String(L.active.size) }), ` quest${L.active.size === 1 ? '' : 's'} in progress`),
        L.completed ? h('span', { class: 'qt-fact' }, h('b', { text: String(quests.filter(questDone).length) }), ' finished') : null,
        L.zone && D.zoneNames.has(L.zone) ? h('span', { class: 'qt-fact' }, 'you are in ', h('b', { text: D.zoneNames.get(L.zone) })) : null];
      status.replaceChildren(
        h('span', { class: 'qt-dot-live' + (L ? ' on' : '') }),
        h('div', { class: 'qt-status-text' },
          h('b', { text: L ? 'Following the game' : 'Not following the game' }),
          L ? h('div', { class: 'qt-facts' }, facts) : h('div', { class: 'note', text: 'With IMAGINE running and this character logged in, your quests in progress show here, with the step you are on.' }),
          L && !L.completed ? h('div', { class: 'note', text: 'To count the quests you have finished, open Quest Completion Bonuses in game once (Menu > Quest > View Quest Bonuses).' }) : null),
        h('button', { type: 'button', class: 'btn btn-sm btn-primary', onclick: panelOpen }, icon('flag'), 'Open the panel over the game'));
    };
    const card = (q, a, isHidden) => {
      const s = q.steps.find((x) => x.phase === a.step);
      const steps = q.steps.filter((x) => x.phase >= 0), idx = steps.findIndex((x) => x.phase === a.step);
      const here = questHereNow(q), map = trackerOpenMap();
      const open = !isHidden && (q.id in map ? map[q.id] : here);
      const toggle = () => { if (isHidden) return; const m = trackerOpenMap(); m[q.id] = !open; store.set('tracker-open', JSON.stringify(m)); draw(); };
      const hide = (ev) => { ev.stopPropagation(); panelHide(q, !isHidden); draw(); };
      return h('section', { class: 'qt-card' + (here ? ' here' : '') + (open ? ' open' : '') + (isHidden ? ' hidden' : '') },
        h('div', { class: 'qt-head', role: isHidden ? null : 'button', tabindex: isHidden ? null : '0', 'aria-expanded': isHidden ? null : String(open), onclick: toggle,
          onkeydown: (e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); toggle(); } } },
          isHidden ? null : h('span', { class: 'chev' }, icon('chevron')),
          h('div', { class: 'qt-title' }, h('b', { text: q.title }), h('span', { class: 'qt-sub', text: [q.type, q.client].filter(Boolean).join(' · ') })),
          here ? h('span', { class: 'tag q-here-tag', text: 'here' }) : null,
          h('span', { class: 'qt-steps', title: idx >= 0 ? `Step ${idx + 1} of ${steps.length}` : 'In progress' },
            steps.map((x, j) => h('span', { class: 'qt-pip' + (j < idx ? ' done' : j === idx ? ' now' : '') })),
            h('span', { class: 'qt-stepno', text: idx >= 0 ? `${idx + 1}/${steps.length}` : '…' })),
          // the icon is what a click does: the crossed-out eye hides, the eye shows again
          h('button', { type: 'button', class: 'icon-btn qt-hide', title: isHidden ? 'Show this quest again' : 'Hide this quest (here and in the panel)', 'aria-label': isHidden ? 'Show this quest again' : 'Hide this quest', onclick: hide }, icon(isHidden ? 'eye' : 'eye-off'))),
        isHidden ? null : open
          ? h('div', { class: 'qt-body' },
            s && s.objective ? h('p', { class: 'qt-objective', text: s.objective }) : null,
            s && s.goals && s.goals.length ? h('div', { class: 'qt-goals' }, s.goals.map((g, i) => trackerGoal(s, i, g, a))) : null,
            s ? h('div', { class: 'qt-next' }, questBlocks(s, q)) : h('p', { class: 'note', text: `The game has this quest at its step ${a.step + 1}, which the guide has no step for.` }),
            s && !q.walkthrough ? h('p', { class: 'note', text: 'The server\'s event files do not cover this quest: only the game\'s own objective text is known.' }) : null,
            h('div', { class: 'qt-foot' }, h('button', { type: 'button', class: 'link-btn', onclick: () => openInQuestDb(q.title) }, 'Open in Quest Database')))
          : h('div', { class: 'qt-peek', text: trackerPeek(s), onclick: toggle }));
    };
    const draw = () => {
      const L = D.live;
      if (!L) { list.replaceChildren(h('div', { class: 'qt-empty' }, icon('flag'), h('p', { text: 'No quests to show yet: BABEL reads them from the game while you play.' }))); return; }
      const hid = new Set(panelHiddenIds());
      const mine = [...L.active.values()].filter((a) => byId.has(a.id)).map((a) => ({ q: byId.get(a.id), a, here: questHereNow(byId.get(a.id)) }))
        .sort((x, y) => (y.here - x.here) || x.q.title.localeCompare(y.q.title));
      const hiddenQ = mine.filter((x) => hid.has(x.q.id)), shownQ = mine.filter((x) => !hid.has(x.q.id));
      const out = [];
      if (hiddenQ.length) {
        const toggle = () => { showHidden = !showHidden; draw(); };
        out.push(h('button', { type: 'button', class: 'qt-hidden-head', 'aria-expanded': String(showHidden), onclick: toggle },
          h('span', { class: 'chev' + (showHidden ? ' open' : '') }, icon('chevron')), icon('eye-off'), `Hidden (${hiddenQ.length})`));
        if (showHidden) out.push(h('div', { class: 'qt-hidden' }, hiddenQ.map((x) => card(x.q, x.a, true))));
      }
      out.push(...shownQ.map((x) => card(x.q, x.a, false)));
      if (!shownQ.length) out.push(h('p', { class: 'note', text: mine.length ? 'Every quest in progress is hidden: open “Hidden” above to show one again.' : 'No quest in progress.' }));
      list.replaceChildren(...out);
    };
    page.append(status, list);
    page.append(h('p', { class: 'note', text: 'Read from the game every few seconds while this page is open (only read, never changed). The steps are the Quest Database\'s: from the public server\'s event files, so IMAGINE may have changed some.' }));
    // the game's progress changed: redraw in place
    D.onLive = () => { drawStatus(); draw(); };
    drawStatus(); draw();
    questLiveTick();
    return page;
  }
  // ---------- Tools: Key Items ----------
  // The game's key items (valuables) a player can see, from key-items.json beside this file (made from the game's
  // valuables table, its English text and the public server's event files: reference\key-items.tsv). Which ones the
  // character holds comes from the game (page command questProgress, its `flags`), read every few seconds while this page
  // is open, and kept per character (babel-keys-held-<name>) so the page still says it with the game closed.
  const KEY_CATS = [['all', 'All'], ['modes', 'Modes and access'], ['licenses', 'Licenses'], ['comp', 'COMP and system'], ['shops', 'Shops and people'],
    ['expertise', 'Expertise cap'], ['story', 'Story'], ['quest', 'Quest items'], ['records', 'Records'], ['corridor', 'Electronic Corridor'], ['seasonal', 'Seasonal'], ['other', 'Use not known']];
  const KEY_HELD = [['all', 'All'], ['have', 'You have'], ['missing', 'Missing']];
  function loadKeys() {
    const D = S.keys;
    if (D.state !== 'idle') return;
    D.state = 'loading';
    fetch('key-items.json', { cache: 'no-store' }).then((r) => (r.ok ? r.json() : null)).catch(() => null).then((d) => {
      D.data = d; D.state = d ? 'ready' : 'failed';
      if (d) d.items.forEach((k) => { k.hay = [k.en, k.jp, k.get, k.unlocks, ...(k.quests || [])].join(' ').toLowerCase(); });
      if (S.page === 'keys') render();
    });
  }
  const keysHeldKey = (name) => 'keys-held-' + name;
  /** What the character holds: { ids: Set, at: ISO time, live: read just now } or null when never read. */
  function keysHeld() {
    const name = simActive();
    if (!name) return null;
    const D = S.keys;
    if (D.live && D.live.name === name) return D.live;
    try {
      const v = JSON.parse(store.get(keysHeldKey(name)) || 'null');
      return v && Array.isArray(v.ids) ? { name, ids: new Set(v.ids), at: v.at, live: false } : null;
    } catch (e) { return null; }
  }
  function keysTick() {
    const D = S.keys, name = simActive();
    if (D.busy || S.page !== 'keys' || document.hidden || !name) return;
    D.busy = true;
    Promise.resolve(host.call('questProgress')).catch(() => null).then((r) => {
      D.busy = false;
      if (!r || !Array.isArray(r.flags) || simActive() !== name) {
        if (D.live && D.live.name === name) { D.live = null; if (S.page === 'keys') render(); }   // the game closed: fall back to the saved list
        return;
      }
      const ids = [...r.flags].sort((a, b) => a - b), raw = ids.join(',');
      const changed = !D.live || D.live.name !== name || D.live.raw !== raw;
      const at = new Date().toISOString();
      D.live = { name, ids: new Set(ids), at, live: true, raw };
      store.set(keysHeldKey(name), JSON.stringify({ at, ids }));
      if (changed && S.page === 'keys') render();
    });
  }
  setInterval(keysTick, 5000);
  /** The first sentence of a text (the table shows it; the open row has all of it), with "…" when there is more. */
  function keyFirst(t) {
    const m = /^(.+?[.!?])(\s+\S[\s\S]*)?$/.exec(t.trim());
    return m && m[2] ? m[1] + ' …' : t;
  }
  function keyDetail(k) {
    const box = h('div', { class: 'q-detail key-detail' });
    const line = (key, kids) => h('div', { class: 'q-needs' }, h('span', { class: 'q-key', text: key }), h('span', null, kids));
    box.append(line('How to get it', k.get || 'Not known.'));
    box.append(line('What it does', k.unlocks || 'Not known.'));
    if (k.quests && k.quests.length) box.append(h('div', { class: 'q-needs' }, h('span', { class: 'q-key', text: 'Quests' }),
      h('ul', { class: 'gdb-list' }, k.quests.map((t) => h('li', null, questLine(`Quest “${t}”`))))));
    const notes = [];
    if (k.newmoon) notes.push('NewMoon’s own: the public server files don’t have it, so this comes from the game’s own description.');
    if (k.seasonal) notes.push('Only from a seasonal event, which may not be running.');
    if (!k.sure) notes.push('Worked out from the game files, not checked in game.');
    if (k.enSrc === 'translated') notes.push(`The game names it only in Japanese (${k.jp}); the English name is BABEL’s translation.`);
    if (notes.length) box.append(h('p', { class: 'note', text: notes.join(' ') }));
    return box;
  }
  function pageKeys() {
    const D = S.keys, page = h('div', { class: 'page page-dcm' });
    loadKeys(); keysTick();
    page.append(pageHead('The game’s key items: which ones you have, what each one opens and how to get the ones you’re missing. Click one to open it.'));
    if (D.state !== 'ready') {
      page.append(h('p', { class: 'note', text: D.state === 'failed' ? 'The key item list could not be loaded (key-items.json is missing beside the page).' : 'Loading the key items…' }));
      return page;
    }
    const held = keysHeld(), items = D.data.items;
    const has = (k) => !!held && held.ids.has(k.id);
    page.append(h('div', { class: 'notice' }, icon('info'), h('span', null,
      !simActive() ? 'Pick a character at the top left to see which key items it has.'
        : !held ? `Start IMAGINE and log in as ${simActive()} to see which key items you have; the page reads them from the game while it is open.`
          : held.live ? `Read from the game just now: ${simActive()} has ${fmt(items.filter(has).length)} of these ${fmt(items.length)} key items.`
            : `As last read from the game (${new Date(held.at).toLocaleString()}): ${simActive()} had ${fmt(items.filter(has).length)} of these ${fmt(items.length)} key items. Log in to read them again.`,
      ' How to get them and what they do come from the game files and the public server’s event files, so NewMoon may have changed some.')));
    const inCat = (k, c) => c === 'all' || k.cat === c;
    const inHeld = (k, c) => c === 'all' || !held || (c === 'have' ? has(k) : !has(k));
    const catChips = h('div', { class: 'chips', role: 'group', 'aria-label': 'Filter by kind of key item' });
    const heldChips = h('div', { class: 'chips', role: 'group', 'aria-label': 'Filter by whether you have it' });
    const body = h('tbody'), pager = h('nav', { class: 'pager', 'aria-label': 'Pages' }), wrap = h('div', { class: 'tbl-wrap' });
    const drawChips = () => {
      catChips.replaceChildren(...KEY_CATS.filter(([id]) => id === 'all' || items.some((k) => k.cat === id)).map(([id, label]) =>
        h('button', { type: 'button', class: 'chip-btn', 'aria-pressed': String(D.cat === id), onclick: () => { D.cat = id; D.pg = 0; drawChips(); draw(); } },
          label, h('span', { class: 'count', text: fmt(items.filter((k) => inCat(k, id) && inHeld(k, D.held)).length) }))));
      heldChips.replaceChildren(...(held ? KEY_HELD : []).map(([id, label]) =>
        h('button', { type: 'button', class: 'chip-btn', 'aria-pressed': String(D.held === id), onclick: () => { D.held = id; D.pg = 0; drawChips(); draw(); } },
          label, h('span', { class: 'count', text: fmt(items.filter((k) => inCat(k, D.cat) && inHeld(k, id)).length) }))));
    };
    const draw = () => {
      body.replaceChildren();
      const q = D.q.trim().toLowerCase();
      const all = items.filter((k) => inCat(k, D.cat) && inHeld(k, D.held) && (!q || k.hay.includes(q)));
      const pg = pageOf(D, all);
      pg.rows.forEach((k) => {
        const open = D.open.has(k.id);
        const toggle = () => { if (D.open.has(k.id)) D.open.delete(k.id); else D.open.add(k.id); draw(); };
        const state = !held ? h('span', { class: 'zero', text: '—' })
          : has(k) ? h('span', { class: 'tag key-have' }, icon('check'), 'Have') : h('span', { class: 'tag tag-off', text: 'Missing' });
        body.append(h('tr', { class: 'row-click' + (open ? ' open' : '') + (held && !has(k) ? ' key-missing' : ''), onclick: (ev) => { if (!ev.target.closest('button')) toggle(); } },
          h('td', null, h('button', { type: 'button', class: 'row-btn', 'aria-expanded': String(open), onclick: toggle },
            h('span', { class: 'chev' }, icon('chevron')),
            // a translated name is greyed like the other pages' (the game names it only in Japanese, shown under it)
            h('span', null, k.enSrc === 'translated' ? h('span', { class: 'key-en tl-name', 'data-tip': TL_TIP(k.jp, 'key item'), text: k.en }) : h('span', { class: 'key-en', text: k.en }),
              k.enSrc === 'translated' ? h('span', { class: 'key-tr', tabindex: 0, 'data-tip': 'The game names it only in Japanese; this is a translation', text: 'translated' }) : null,
              k.jp && k.jp.normalize('NFKC') !== k.en.normalize('NFKC') ? h('span', { class: 'key-jp', lang: 'ja', text: k.jp }) : null))),
          h('td', { class: 'key-does' }, k.unlocks && k.unlocks !== 'Not known.' ? keyFirst(k.unlocks) : h('span', { class: 'zero', text: 'Not known' })),
          h('td', null, h('span', { class: 'tag', text: (KEY_CATS.find(([id]) => id === k.cat) || [, k.cat])[1] }),
            k.newmoon ? h('span', { class: 'tag tag-nm', text: 'NewMoon' }) : null),
          h('td', null, state)));
        if (open) body.append(h('tr', { class: 'detail-row' }, h('td', { colspan: 4 }, keyDetail(k))));
      });
      if (!all.length) body.append(h('tr', null, h('td', { colspan: 4, class: 'empty-row', text: q ? `Nothing matches “${D.q.trim()}”.` : 'No key item in this group.' })));
      drawPager(pager, D, all.length, pg, draw, () => wrap, 'key items');
    };
    page.append(h('div', { class: 'toolbar' }, searchBox(D.q, 'Search a key item, quest, person or place', (v) => { D.q = v; D.pg = 0; drawChips(); draw(); }), heldChips));
    page.append(h('div', { class: 'toolbar' }, catChips));
    wrap.append(h('table', { class: 'tbl key-tbl' },
      h('thead', null, h('tr', null, h('th', { scope: 'col', text: 'Key item' }), h('th', { scope: 'col', text: 'What it does' }), h('th', { scope: 'col', text: 'Kind' }), h('th', { scope: 'col', text: 'You' }))), body));
    page.append(wrap, pager);
    page.append(h('p', { class: 'note', text: `${fmt(items.length)} key items the game shows (its hidden progress flags are left out). English names come from the game’s own English text where it has one; the rest are translations, marked. Data made on ${D.data.generatedAt}.` }));
    drawChips(); draw();
    return page;
  }

  // ---------- Tools: Dungeon Guides ----------
  // dungeon-guides.json is made in the knowledge base (newmoon-kb\progression\guides\build.mjs, from the event-script
  // research kept there): every Dungeon Database dungeon where something done inside the run (or a roll) changes the
  // fight, the route or the reward, and for each outcome the steps to get it (what to do, where, what it needs, what
  // breaks it). The rest are listed as linear. Read from the public server's scripts, not NewMoon's; not tested in game.
  // The tags are picked by hand per dungeon in the knowledge base (research\tags.json), with what each one means:
  const GUIDE_TAGS = [['all', 'Any', ''],
    ['hidden', 'Hidden boss', 'A boss you only get by doing something on purpose (random rare spawns don’t count)'],
    ['summon', 'Summon a demon', 'A specific demon has to be summoned at some step'],
    ['minigame', 'Minigame', 'An event run scored by points, waves, a race or a hunt']];
  const guideTagName = (t) => (GUIDE_TAGS.find(([id]) => id === t) || [, t])[1];
  const guideKey = (g) => g.name + '|' + g.from;
  function loadGuides() {
    if (S.guides.state !== 'idle') return;
    S.guides.state = 'loading';
    fetch('dungeon-guides.json', { cache: 'no-store' }).then((r) => (r.ok ? r.json() : null)).catch(() => null).then((d) => {
      S.guides.data = d; S.guides.state = d ? 'ready' : 'failed';
      if (d) d.guides.forEach((g) => {
        g.key = guideKey(g);
        g.hay = [g.name, g.from, ...g.tags.map(guideTagName), ...g.outcomes.flatMap((o) => [o.result, o.fails, ...o.steps.flatMap((s) => [s.do, s.where, s.needs])])].join(' ').toLowerCase();
      });
      if (S.page === 'guides') render();
    });
  }
  /** One dungeon's guide: what to bring (Diaspora), then one row per outcome; a row opens its numbered steps. A guide
   *  with a single outcome shows it open. */
  function guideDetail(g, redraw) {
    const D = S.guides, box = h('div', { class: 'q-detail guide-detail' });
    if (g.prep) {
      const line = (k, v) => (v && v.length ? h('div', { class: 'q-line' }, h('span', { class: 'q-key', text: k }), h('span', null, Array.isArray(v) ? v.join(' · ') : v)) : null);
      box.append(h('div', { class: 'guide-prep' }, line('Before you go', g.prep.facts), line('You need', g.prep.needs), line('Recipes', g.prep.recipes), line('Expect', g.prep.expect)));
    }
    const single = g.outcomes.length === 1;
    if (!single) box.append(h('span', { class: 'q-key', text: `${g.outcomes.length} ways this dungeon can go · click one for its steps` }));
    const outs = h('div', { class: 'guide-outs' });
    g.outcomes.forEach((o, i) => {
      const k = g.key + '#' + i, open = single || D.outOpen.has(k);
      const toggle = () => { if (D.outOpen.has(k)) D.outOpen.delete(k); else D.outOpen.add(k); redraw(); };
      const head = h('div', { class: 'guide-res' }, h('span', { class: 'guide-res-text', text: o.result }),
        o.inferred ? h('span', { class: 'tag tag-off', tabindex: 0, 'data-tip': 'Worked out from how the script fits together, not stated by it: check it in game first', text: 'inferred' }) : null,
        h('span', { class: 'src-count', text: o.steps.length === 1 ? '1 step' : `${o.steps.length} steps` }));
      if (!open) { outs.append(h('section', { class: 'guide-out' }, h('button', { type: 'button', class: 'row-btn guide-out-btn', 'aria-expanded': 'false', onclick: toggle }, h('span', { class: 'chev' }, icon('chevron')), head))); return; }
      outs.append(h('section', { class: 'guide-out' + (single ? '' : ' open') },
        single ? head : h('button', { type: 'button', class: 'row-btn guide-out-btn', 'aria-expanded': 'true', onclick: toggle }, h('span', { class: 'chev' }, icon('chevron')), head),
        h('ol', { class: 'q-steps guide-steps' }, o.steps.map((s) => h('li', null,
          h('span', { class: 'guide-do', text: s.do }),
          s.where ? h('div', { class: 'q-line' }, h('span', { class: 'q-key', text: 'Where' }), h('span', { text: s.where })) : null,
          s.needs ? h('div', { class: 'q-line' }, h('span', { class: 'q-key', text: 'Needs' }), h('span', { text: s.needs })) : null))),
        o.fails ? h('div', { class: 'q-line guide-fails' }, h('span', { class: 'q-key', text: 'Breaks if' }), h('span', { text: o.fails })) : null));
    });
    box.append(outs);
    box.append(h('div', null, h('button', { type: 'button', class: 'btn btn-sm', onclick: () => { S.dun.q = g.name; S.dun.kind = 'all'; go('dungeons'); } }, icon('gate'), 'Its enemies in the Dungeon Database')));
    return box;
  }
  function pageGuides() {
    const D = S.guides, page = h('div', { class: 'page page-dcm' });
    loadGuides();
    page.append(pageHead('The dungeons that aren’t just “go in and kill everything”: hidden and True bosses, demons to summon, objects to click, choices, rolls and timers. Click a dungeon for each outcome and the steps to get it.'));
    if (D.state !== 'ready') {
      page.append(h('p', { class: 'note', text: D.state === 'failed' ? 'The guides could not be loaded (dungeon-guides.json is missing beside the page).' : 'Loading the guides…' }));
      return page;
    }
    page.append(h('div', { class: 'notice' }, icon('info'), h('span', null, h('b', { text: 'Read from the server’s event scripts, not checked on IMAGINE. ' }),
      'The steps come from the public server data, so IMAGINE may have changed some. Steps marked “inferred” were worked out from how the script fits together; try those first before planning a run around them.')));
    const list = D.data.guides;
    const inTag = (g, t) => t === 'all' || g.tags.includes(t);
    const tagChips = h('div', { class: 'chips', role: 'group', 'aria-label': 'Filter by what makes it special' });
    const body = h('tbody');
    const drawChips = () => {
      tagChips.replaceChildren(...GUIDE_TAGS.map(([id, label, tip]) => h('button', { type: 'button', class: 'chip-btn', 'aria-pressed': String(D.tag === id), 'data-tip': tip || null, onclick: () => { D.tag = id; drawChips(); draw(); } },
        label, h('span', { class: 'count', text: fmt(list.filter((g) => inTag(g, id)).length) }))));
    };
    const draw = () => {
      body.replaceChildren();
      const q = D.q.trim().toLowerCase();
      const rows = list.filter((g) => inTag(g, D.tag) && (!q || g.hay.includes(q)));
      rows.forEach((g) => {
        const open = D.open.has(g.key);
        const toggle = () => { if (D.open.has(g.key)) D.open.delete(g.key); else D.open.add(g.key); draw(); };
        const steps = g.outcomes.reduce((a, o) => a + o.steps.length, 0);
        body.append(h('tr', { class: 'row-click' + (open ? ' open' : ''), onclick: (ev) => { if (!ev.target.closest('button')) toggle(); } },
          h('td', null, h('button', { type: 'button', class: 'row-btn', 'aria-expanded': String(open), onclick: toggle },
            h('span', { class: 'chev' }, icon('chevron')),
            h('span', null, g.name, g.from && g.from !== g.name ? h('span', { class: 'src-count', text: `entered from ${g.from}` }) : null,
              g.modes.length ? h('span', { class: 'src-count', text: `modes: ${g.modes.join(', ')}` }) : null))),
          h('td', null, h('span', { class: 'guide-tags' }, g.tags.map((t) => h('span', { class: 'tag guide-tag guide-tag-' + t, tabindex: 0, 'data-tip': (GUIDE_TAGS.find(([id]) => id === t) || [])[2] || null, text: guideTagName(t) })))),
          h('td', { class: 'num' }, String(g.outcomes.length), h('span', { class: 'src-count', text: steps === 1 ? '1 step' : `${fmt(steps)} steps` }))));
        if (open) body.append(h('tr', { class: 'detail-row' }, h('td', { colspan: 3 }, guideDetail(g, draw))));
      });
      if (!rows.length) body.append(h('tr', null, h('td', { colspan: 3, class: 'empty-row', text: q ? `Nothing matches “${D.q.trim()}”.` : 'No dungeon in this group.' })));
    };
    page.append(h('div', { class: 'toolbar' }, searchBox(D.q, 'Search a dungeon, boss, demon, item or mechanic', (v) => { D.q = v; draw(); }), tagChips));
    page.append(h('div', { class: 'tbl-wrap' }, h('table', { class: 'tbl guide-tbl' },
      h('thead', null, h('tr', null, h('th', { scope: 'col', text: 'Dungeon' }), h('th', { scope: 'col', text: 'Tags' }), h('th', { scope: 'col', class: 'num', text: 'Outcomes' }))), body)));
    const lin = D.data.linear || [];
    page.append(h('details', { class: 'guide-linear' }, h('summary', null, `${fmt(lin.length)} linear dungeons: get in, kill everything, done`),
      h('p', { class: 'note', text: lin.map((x) => x.name).join(' · ') })));
    page.append(h('p', { class: 'note', text: `${fmt(list.length)} dungeons with a guide, ${fmt(list.reduce((a, g) => a + g.outcomes.length, 0))} outcomes. Entry modes (WILDCAT, BEARCAT, Time trial, Hard) are listed on each dungeon but don't make it special on their own. Hover a tag for what it means. Data made on ${D.data.generatedAt}.` }));
    drawChips(); draw();
    return page;
  }

  // ---------- Pages for the long lists (Item Database, Skill Database) ----------
  // Pages of S.perPage rows (one saved choice for every list, babel-rows-per-page): 17,000 table rows at once is too
  // slow, and a page number can be come back to. pageOf keeps st.pg in range and gives this page's rows; drawPager fills
  // the pager under a table: "a–b of N", the per-page picker (when the list is longer than the smallest page),
  // Previous / Next, the first and last page and the two around this one, gaps as "…". A page change scrolls the table
  // back into view. Every filter or search change on a page sets st.pg = 0 itself.
  const PAGE_SIZES = [100, 200, 500];
  S.perPage = PAGE_SIZES.includes(Number(store.get('rows-per-page') || store.get('gdb-per'))) ? Number(store.get('rows-per-page') || store.get('gdb-per')) : 100;
  function pageOf(st, rows) {
    const per = S.perPage, pages = Math.max(1, Math.ceil(rows.length / per));
    st.pg = Math.min(Math.max(0, st.pg | 0), pages - 1);
    return { per, pages, rows: rows.slice(st.pg * per, (st.pg + 1) * per) };
  }
  function drawPager(pager, st, total, { per, pages }, draw, anchor, what) {
    const goPage = (p) => { st.pg = p; draw(); const a = anchor && anchor(); if (a) a.scrollIntoView({ block: 'start' }); };
    const nums = [...new Set([0, pages - 1, st.pg - 2, st.pg - 1, st.pg, st.pg + 1, st.pg + 2].filter((p) => p >= 0 && p < pages))].sort((a, b) => a - b);
    const kids = [h('span', { class: 'note', text: total ? `${fmt(st.pg * per + 1)}–${fmt(Math.min((st.pg + 1) * per, total))} of ${fmt(total)}${what ? ' ' + what : ''}` : '' })];
    if (total > PAGE_SIZES[0]) kids.push(h('label', { class: 'gdb-check pager-per' }, 'Per page',
      h('select', { class: 'input ddb-select', 'aria-label': 'Rows per page', onchange: (ev) => { S.perPage = Number(ev.target.value); store.set('rows-per-page', String(S.perPage)); st.pg = 0; draw(); } },
        PAGE_SIZES.map((n) => h('option', { value: String(n), selected: n === per ? true : null, text: String(n) })))));
    if (pages > 1) {
      kids.push(h('button', { type: 'button', class: 'btn btn-sm', disabled: st.pg === 0 ? true : null, onclick: () => goPage(st.pg - 1) }, 'Previous'));
      nums.forEach((p, i) => {
        if (i && p !== nums[i - 1] + 1) kids.push(h('span', { class: 'pager-gap', text: '…' }));
        kids.push(h('button', { type: 'button', class: 'btn btn-sm pager-num', 'aria-current': p === st.pg ? 'page' : null, 'aria-label': `Page ${p + 1}`, onclick: () => goPage(p) }, String(p + 1)));
      });
      kids.push(h('button', { type: 'button', class: 'btn btn-sm', disabled: st.pg >= pages - 1 ? true : null, onclick: () => goPage(st.pg + 1) }, 'Next'));
    }
    pager.replaceChildren(...kids);
  }
  // ---------- Tools: Item Database ----------
  // gear-db.json (the file keeps its old name) is made by "imagine-reader geardb": every item the IMAGINE client names.
  // What gear gives is read from the client's current tables; other items carry the game's own description; where an
  // item comes from is traced through the server data, with the community spreadsheets' notes for gear. It sits beside
  // this file. Item pictures are in gear-db-icons\ beside it (the game's own art, made locally with geardb --client, not in git).
  const GDB_SLOTS = ['Weapon', 'Head', 'Face', 'Neck', 'Top', 'Arms', 'Bottom', 'Feet', 'Ring', 'Earring', 'Extra', 'Back', 'Talisman', 'COMP', 'Bullets'];
  // the kinds of item (it.kind, from the builder), in the order of the chips, with the chip's label
  const GDB_KINDS = [['Gear', 'Gear'], ['Consumable', 'Consumables'], ['Material', 'Materials'], ['Voucher', 'Vouchers & boxes'], ['Enchant', 'Tarot & soul crystals'],
    ['Demon equipment', 'Demon equipment'], ['Appearance', 'Appearance'], ['Key item', 'Key items'], ['Other', 'Other']];
  const gdbKindLabel = (k) => (GDB_KINDS.find((x) => x[0] === k) || [k, k])[1];
  // the source filter: how a "Where to get it" line's kind is grouped, and the choices offered (smtwiki.ru splits its item
  // tables by where things come from: shops, drops, events; the same idea, from the traced sources)
  const GDB_SRC_OF = { Buy: 'Shop', Exchange: 'Exchange', 'Enemy drop': 'Drop', Chest: 'Chest', Plasma: 'Chest', Open: 'Box', 'Random box': 'Box', 'Event or NPC': 'Event', 'Event reward': 'Event', Crystallize: 'Crystallize', Craft: 'Craft', 'Demon present': 'Present', 'Drop table': 'Table', 'Destiny box': 'Table', 'Demon quest': 'Table' };
  const GDB_SRC = [['Shop', 'Bought in a shop'], ['Exchange', 'NPC exchange'], ['Drop', 'Enemy drop'], ['Chest', 'Chest or plasma'], ['Box', 'Ticket or random box'], ['Event', 'Event or NPC'], ['Crystallize', 'Crystallized from a demon'], ['Craft', 'Crafted'], ['Quest', 'Quest reward'], ['Present', 'Demon present'], ['Table', 'Drop table only'], ['Notes', 'Community notes only'], ['None', 'No known source']];
  // the columns, each sortable by a click on its header (D.sort, D.dir)
  const GDB_COLS = [['name', 'Item'], ['type', 'Type'], ['gives', 'Gives'], ['where', 'Where to get it']];
  /** Loads gear-db.json once; the promise gives the data (null when it can't be read). Other pages use it too (the
   *  Character Planner's pictures and "Where to get it"). */
  function loadGearDb() {
    if (S.gdb.wait) return S.gdb.wait;
    if (S.gdb.state !== 'idle') return (S.gdb.wait = Promise.resolve(S.gdb.data || null));
    S.gdb.state = 'loading';
    return (S.gdb.wait = fetch('gear-db.json', { cache: 'no-store' }).then((r) => (r.ok ? r.json() : null)).catch(() => null).then((d) => {
      S.gdb.data = d; S.gdb.state = d ? 'ready' : 'failed';
      if (d) {
        d.items.forEach((it, i) => {
          // key items have their own id space; sheet-only rows have no id
          it.key = it.keyItem ? 'key' + it.id : it.id || 'sheet' + i;
          it.kind = it.kind || 'Gear';
          if (it.kind === 'Enchant' && it.class === 'Consumable') delete it.class;   // the client files every crystal as a consumable: no finer type to show
          // sheet-only rows carry the sheet's own sex column ("M/F", "M", "F", "N/A"): the game's words for the filter
          it.gender = { 'M/F': 'Any', M: 'Male', F: 'Female', 'N/A': 'Any' }[it.gender] || it.gender || 'Any';
          // the game's own effect text packs several lines into one, with a written-out line break between them
          const lines = (list) => (list || []).flatMap((x) => String(x).split(/\s*\\n\s*/)).map((x) => x.trim()).filter(Boolean);
          it.stats = lines(it.stats); it.effects = lines(it.effects); it.appearance = lines(it.appearance);
          it.tarot = lines(it.tarot); it.soul = lines(it.soul); it.force = lines(it.force); it.desc = it.desc || []; it.gives = it.gives || []; it.how = it.how || []; it.quests = it.quests || [];
          (it.sets || []).forEach((s) => { s.effects = lines(s.effects); });
          it.all = [...(it.stats || []), ...(it.effects || []), ...(it.appearance || []), ...(it.sets || []).flatMap((s) => s.effects)];
          it.get = it.get || []; it.sources = it.sources || [];
          const getWords = (list) => (list || []).flatMap((g) => [g.k, g.t, g.s || '']);
          it.known = it.get.length > 0 || it.sources.length > 0 || it.how.length > 0 || it.quests.length > 0;
          // the ways to get it, as the source filter groups them (every line counts, not only the first)
          it.src = new Set(it.get.map((g) => GDB_SRC_OF[g.k] || '').filter(Boolean));
          if (it.quests.length || it.how.some((w) => /^Quest/.test(w))) it.src.add('Quest');
          if (it.how.some((w) => /^Exchange/.test(w))) it.src.add('Exchange');
          if (it.sources.some((s) => s.ticket)) it.src.add('Box');
          if (it.sources.length) it.src.add('Notes');
          if (!it.src.size) it.src.add('None');
          it.hay = [it.name, it.ja || '', it.kind, it.class || '', it.slot || '', ...it.all, ...it.tarot, ...it.soul, ...it.force, ...it.desc, ...it.gives, ...it.how, ...it.quests,
            ...(it.sets || []).flatMap((s) => s.with), ...getWords(it.get),
            ...it.sources.flatMap((s) => [s.tab, s.how, s.ticket || '', ...getWords(s.ticketGet)])].join(' ').toLowerCase();
        });
        d.items.sort((a, b) => a.name.localeCompare(b.name));
        // the classes (the game's own item classes) in each kind, for the second row of chips
        d.classes = {};
        d.items.forEach((it) => { if (it.class) (d.classes[it.kind] = d.classes[it.kind] || new Set()).add(it.class); });
      }
      // the Rebirth Planner's equipment and Demon Force steps say where each part and item comes from; the Dungeon Database's
      // drops show the item's entry (a tooltip made before this arrived is cached on its element: redraw)
      if (S.page === 'geardb' || S.page === 'rebirth' || S.page === 'dungeons') render();
      return d;
    }));
  }
  /** Where to get an item in the game, as the Item Database shows it: the list lines (shops, drops, exchanges, quests,
   *  the ticket or box it comes in and where that comes from) and the community notes. */
  function gdbWhere(it) {
    // g.d: how deep the line sits under the one before it ("Open X", then where X comes from)
    const getLine = (g, extra) => h('li', { style: (g.d || 0) + (extra || 0) ? `padding-left:${22 * ((g.d || 0) + (extra || 0))}px` : null },
      g.k ? h('span', { class: 'gdb-kind', text: g.k }) : null, g.k ? h('b', { text: g.t }) : h('span', { class: 'src-count', style: 'margin-left:0', text: g.t }),
      g.s ? h('span', { class: 'src-count', text: g.s }) : null, g.v ? h('span', { class: 'gdb-rate', text: g.v }) : null);
    const where = [];
    it.get.forEach((g) => where.push(getLine(g)));
    // a fusion plugin: the Fusion Planner's ways to get it (a quest, an exchange); several are alternatives
    if (it.how.length > 1) where.push(h('li', { class: 'any-one', text: `Any one of these ${it.how.length} ways gives it:` }));
    it.how.forEach((w) => where.push(h('li', null, questLine(w))));
    // quests that hand it over as a reward (the Quest Database's data), each with a link to the quest
    it.quests.forEach((t) => where.push(h('li', { class: 'gdb-quest' }, h('span', { class: 'gdb-kind', text: 'Quest' }), questLine(`Quest “${t}”`))));
    if (it.questsMore) where.push(h('li', { class: 'gdb-with', text: `and ${fmt(it.questsMore)} more quests` }));
    const tickets = new Map();
    it.sources.forEach((s) => { if (s.ticket && !tickets.has(s.ticket)) tickets.set(s.ticket, s.ticketGet || []); });
    tickets.forEach((lines, ticket) => {
      where.push(h('li', null, h('span', { class: 'gdb-kind', text: 'Comes in' }), h('b', { text: ticket }),
        lines.length ? null : h('span', { class: 'src-count', text: 'the game data does not say where this ticket comes from' })));
      lines.forEach((g) => where.push(getLine(g, 1)));
    });
    // What players wrote down about it (the place is the name of the sheet's page, usually the dungeon).
    const notes = [...new Set(it.sources.map((s) => [s.tab, s.how].filter(Boolean).join(' · ')))];
    return { where, notes };
  }
  /**
   * An item's Item Database entry as a tooltip (the Dungeon Database's drops): `pic` and `sub` head it (the page's own
   * picture and what it says about the drop), then the kind and facts, the description, what it gives and where to get
   * it, each list cut short. `null` while gear-db.json is not in yet (it is asked for here) or when it lacks the item:
   * the caller keeps its own tooltip then.
   */
  function itemCardTip(id, pic, sub) {
    if (S.gdb.state === 'idle') loadGearDb();
    const d = S.gdb.data;
    const it = d && id ? (S.gdb.byId || (S.gdb.byId = new Map(d.items.filter((x) => x.id && !x.keyItem).map((x) => [x.id, x])))).get(id) : null;
    if (!it) return null;
    const gear = it.kind === 'Gear';
    const facts = [gear ? it.slot : gdbKindLabel(it.kind), it.class && it.class !== it.slot && it.class !== gdbKindLabel(it.kind) && it.class !== it.kind ? it.class : '',
      it.level ? `Level ${it.level}+` : '', it.gender && it.gender !== 'Any' ? it.gender + ' only' : '', it.alignment ? it.alignment + ' only' : ''].filter(Boolean);
    const MAX = 8;
    const sec = (label, list, cls) => (list && list.length ? [h('div', { class: 'tt-sec', text: label }),
      ...list.slice(0, MAX).map((x) => h('div', { class: 'tt-line' + (cls ? ' ' + cls : ''), text: x })),
      list.length > MAX ? h('div', { class: 'tt-line', text: `and ${fmt(list.length - MAX)} more` }) : null] : []);
    const where = it.get.map((g) => [g.k, g.t, g.s, g.v].filter(Boolean).join(' · '));
    return h('div', null, tipHead(pic, it.name, [facts.join(' · '), sub].filter(Boolean).join(' · ')),
      it.ja ? h('p', { class: 'tt-line tl-name', text: TL_TIP(it.ja, 'item') }) : null,
      ...(it.desc.length ? [h('div', { class: 'tt-sec', text: it.descJa ? 'Description (translated)' : 'Description' }), ...tlTipLines(it.desc.join('\n'), it.descJa)] : []),
      ...sec('Tarot Fusion', it.tarot), ...sec('Soul Fusion', it.soul), ...sec('Demon Force', it.force),
      ...sec('Can give', it.gives.concat(it.givesMore ? [`and ${fmt(it.givesMore)} more`] : [])),
      ...(it.sets || []).flatMap((x) => sec('Set bonus', x.effects)),
      ...sec('Appearance Bonus', it.appearance), ...sec('Basic Features', it.stats), ...sec('Characteristics', it.effects),
      ...sec('Where to get it', where.concat(it.how)));
  }
  function gearDbDetail(it) {
    const box = h('div', { class: 'q-detail' });
    const block = (label, list) => (list && list.length ? h('div', { class: 'gdb-block' }, h('span', { class: 'q-key', text: label }), h('ul', { class: 'gdb-list' }, list.map((x) => h('li', { text: x })))) : null);
    // what kind of item it is: the slot for gear, the game's own class for the rest (never both when they say the same)
    const gear = it.kind === 'Gear';
    const facts = [gear ? it.slot : gdbKindLabel(it.kind), it.class && it.class !== it.slot && it.class !== gdbKindLabel(it.kind) && it.class !== it.kind ? it.class : '',
      it.level ? `Level ${it.level}+` : '', it.gender && it.gender !== 'Any' ? it.gender + ' only' : '', it.alignment ? it.alignment + ' only' : '', it.weapon || '', it.affinity ? it.affinity + ' affinity' : '',
      it.stack ? `Stacks to ${fmt(it.stack)}` : '', it.sell ? `Sells for ${fmt(it.sell)} Macca` : ''].filter(Boolean);
    box.append(h('div', { class: 'detail-meta' }, facts.map((x) => h('span', { class: 'tag', text: x }))));
    // `ja`: the game names it only in Japanese and the name is BABEL's translation (item-names-en.tsv, or the Key Items page's)
    if (it.ja) box.append(h('p', { class: 'note' }, h('span', { class: 'tl-name', text: it.name }), ` is BABEL's translation. The game has no English name for this item: it calls it ${it.ja}.`));
    if (it.sheetOnly) box.append(h('p', { class: 'note', text: 'This row is from the spreadsheet only: its name matched no item in the game data, so the effects below are the sheet\'s own text.' }));
    // the game's own description (its tooltip), then what a tarot or soul crystal gives when fused into a piece
    const descBlock = block(it.descJa ? 'Description (translated)' : 'Description', it.desc);
    if (descBlock && it.descJa) { descBlock.classList.add('tl-name'); descBlock.setAttribute('data-tip', TL_DESC_TIP(it.descJa)); }
    if (descBlock) box.append(descBlock);
    // a Demon Force material: the effect it puts in a demon's force slot (the Demon Planner's Demon Force step)
    [block('Tarot Fusion', it.tarot), block('Soul Fusion', it.soul), block('Demon Force', it.force)].filter(Boolean).forEach((b) => box.append(b));
    // a voucher, ticket or random box: the items it can give (the server's box tables)
    if (it.gives.length) {
      const b = block('Can give', it.gives);
      if (it.givesMore) b.lastChild.append(h('li', { class: 'gdb-with', text: `and ${fmt(it.givesMore)} more` }));
      box.append(b);
    }
    // The sections are named and ordered as in a worn piece's tooltip on the Gear page: set bonus, then TIP_ORDER.
    // s.any: the bonus needs one of the listed pieces; otherwise it needs all of them
    (it.sets || []).forEach((s) => {
      const b = block('Set bonus', s.effects); if (!b) return;
      b.lastChild.prepend(h('li', { class: 'gdb-with', text: s.any && s.with.length > 1 ? `Pieces: this and any one of ${s.with.join(', ')}` : `Pieces: this, ${s.with.join(', ')}` }));
      box.append(b); });
    // sheet-only rows: "effects" is the sheet's slot text, not the game's characteristics
    [block('Appearance Bonus', it.appearance), block('Basic Features', it.stats), block(it.sheetOnly ? 'Effects' : 'Characteristics', it.effects),
      it.mods ? block('Modification Slot', [`${it.mods} slot${it.mods === 1 ? '' : 's'}`]) : null].filter(Boolean).forEach((b) => box.append(b));
    if (gear && !it.all.length && !it.sheetOnly) box.append(h('p', { class: 'note', text: 'This piece gives no stats or effects in the game data.' }));
    // Where to get it, in the game: what the game data says about the piece itself, then about the ticket it comes in.
    const { where, notes } = gdbWhere(it);
    if (where.length) box.append(h('div', { class: 'gdb-block' }, h('span', { class: 'q-key', text: 'Where to get it' }), h('ul', { class: 'gdb-list gdb-get' }, where)));
    if (notes.length) box.append(h('div', { class: 'gdb-block' }, h('span', { class: 'q-key', text: 'Community notes' }), h('ul', { class: 'gdb-list' }, notes.map((n) => h('li', { text: n })))));
    if (!where.length && !notes.length) box.append(h('p', { class: 'note', text: gear ? 'Neither the game data nor the community notes say where this piece comes from.' : 'The game data does not say where this item comes from.' }));
    if (it.id) box.append(h('p', { class: 'note', text: it.keyItem ? `Key item ${it.id}` : `Item ${it.id}` }));
    return box;
  }
  function pageGearDb() {
    const D = S.gdb, page = h('div', { class: 'page page-dcm' });
    loadGearDb();
    page.append(pageHead('Every item in the game: what it gives or does, and where it comes from. Click a row for the details.'));
    if (D.state !== 'ready') {
      page.append(h('p', { class: 'note', text: D.state === 'failed' ? 'The item list could not be loaded (gear-db.json is missing beside the page).' : 'Loading the item list…' }));
      return page;
    }
    page.append(h('div', { class: 'notice' }, icon('info'), h('span', null, h('b', { text: 'Two sources. ' }),
      'Names, descriptions, stats, effects and sets are read from the IMAGINE client\'s current item tables. "Where to get it" and what a box can give are traced through the public server data (drops, chests, shops, events, exchanges), which IMAGINE may have changed. "Community notes" are what the community wrote down for IMAGINE, and can be imprecise.')));
    const items = D.data.items;
    // two rows of chips: the kind of item, then within it the slot (gear) or the game's own class (everything else)
    const sub = (it) => (it.kind === 'Gear' ? it.slot : it.class) || '';
    // the finer filters (the folded chip groups): who can use it, where it comes from, weapon range and affinity, level.
    // `f` is the set of picks to test against: the page's (D), or one with a single group changed for a chip's count
    const fine = (it, f = D) => (!f.gender || (it.gender || 'Any') === f.gender)
      && (!f.align || (f.align === 'Any' ? !it.alignment : it.alignment === f.align))
      && (!f.src.length || f.src.some((s) => it.src.has(s)))
      && (!f.range || it.weapon === f.range) && (!f.aff.length || f.aff.includes(it.affinity))
      && (!f.maxLv || (it.level || 0) <= Number(f.maxLv));
    const pass = (it, kind, cls, f) => (!D.known || it.known) && fine(it, f) && (kind === 'all' || it.kind === kind) && (cls === 'all' || sub(it) === cls);
    const fineCount = () => (D.gender ? 1 : 0) + (D.align ? 1 : 0) + D.src.length + (D.range ? 1 : 0) + D.aff.length + (D.maxLv ? 1 : 0);
    // the last column's text: the game's own answer first, then a plugin's way, a quest, the ticket, then the place the community notes file it under
    const whereText = (it) => {
      const ticket = (it.sources.find((s) => s.ticket) || {}).ticket;
      return it.get.length ? (it.get[0].k === 'Open' ? `Open ${it.get[0].t}` : `${it.get[0].k}: ${it.get[0].t}`) : it.how.length ? it.how[0] : it.quests.length ? `Quest: ${it.quests[0]}` : ticket ? `Open ${ticket}`
        : it.sources.length ? [...new Set(it.sources.map((s) => s.tab))].slice(0, 2).join(', ') : '';
    };
    const typeText = (it) => D.kind === 'all' && it.kind !== 'Gear' ? [gdbKindLabel(it.kind), sub(it) !== gdbKindLabel(it.kind) && sub(it) !== it.kind ? sub(it) : ''].filter(Boolean).join(' · ') : sub(it) || gdbKindLabel(it.kind);
    const kindChips = h('div', { class: 'chips', role: 'group', 'aria-label': 'Filter by kind of item' });
    const classChips = h('div', { class: 'chips gdb-sub-chips', role: 'group', 'aria-label': 'Filter by type' });
    const body = h('tbody'), pager = h('nav', { class: 'pager', 'aria-label': 'Pages' });
    const chip = (pressed, label, n, onclick) => h('button', { type: 'button', class: 'chip-btn', 'aria-pressed': String(pressed), onclick }, label, h('span', { class: 'count', text: n }));
    const drawChips = () => {
      const kinds = GDB_KINDS.filter(([k]) => items.some((it) => it.kind === k));
      kindChips.replaceChildren(...[['all', 'All']].concat(kinds).map(([id, label]) =>
        chip(D.kind === id, label, items.filter((it) => pass(it, id, 'all')).length, () => { D.kind = id; D.cls = 'all'; D.pg = 0; drawChips(); drawFilters(); draw(); })));
      // the second row: slots for gear (the game's order), the classes the kind has otherwise; only when there is a choice
      const subs = D.kind === 'Gear' ? GDB_SLOTS : D.kind === 'all' ? [] : [...(D.data.classes[D.kind] || [])].sort((a, b) => a.localeCompare(b));
      const shown = subs.filter((s) => items.some((it) => it.kind === D.kind && sub(it) === s));
      classChips.replaceChildren(...(shown.length > 1 ? [['all', 'All']].concat(shown.map((s) => [s, s])).map(([id, label]) =>
        chip(D.cls === id, label, items.filter((it) => pass(it, D.kind, id)).length, () => { D.cls = id; D.pg = 0; drawChips(); drawFilters(); draw(); })) : []));
      classChips.hidden = shown.length <= 1;
    };
    // the Gives column: what a piece gives, what a crystal fuses in, what a box can give, else the game's description
    const givesText = (it) => {
      const list = it.all.length ? it.all : it.tarot.length || it.soul.length ? [...it.tarot.map((x) => `Tarot: ${x}`), ...it.soul.map((x) => `Soul: ${x}`)]
        : it.force.length ? it.force.map((x) => `Demon Force: ${x}`) : it.gives.length ? [`Can give ${it.gives.slice(0, 2).join(', ')}` + (it.gives.length + (it.givesMore || 0) > 2 ? ` and ${fmt(it.gives.length + (it.givesMore || 0) - 2)} more` : '')] : it.desc;
      // a description is the game's whole tooltip: the start of it here, all of it in the opened row
      if (list === it.desc) { const d = it.desc.join(' '); return d.length > 110 ? d.slice(0, 108).replace(/\s+\S*$/, '') + '…' : d; }
      return list.slice(0, 3).join(' · ') + (list.length > 3 ? ` · and ${list.length - 3} more` : '');
    };
    const draw = () => {
      body.replaceChildren();
      const words = D.q.trim().toLowerCase().split(/\s+/).filter(Boolean);
      const rows = items.filter((it) => pass(it, D.kind, D.cls) && words.every((w) => it.hay.includes(w)));
      // the sorted column (a click on its header): the list is by name already; Gives sorts by how much an item gives
      // (most first when descending); Type and Where by their text, with the empty ones last
      const gives = (it) => it.all.length + it.tarot.length + it.soul.length + it.gives.length;
      const text = (it) => D.sort === 'type' ? typeText(it) : whereText(it);
      if (D.sort === 'name' && D.dir === -1) rows.reverse();
      else if (D.sort === 'gives') rows.sort((a, b) => D.dir * (gives(a) - gives(b)) || a.name.localeCompare(b.name));
      else if (D.sort !== 'name') rows.sort((a, b) => { const ta = text(a), tb = text(b); return (!ta !== !tb) ? (ta ? -1 : 1) : D.dir * ta.localeCompare(tb) || a.name.localeCompare(b.name); });
      const pg = pageOf(D, rows);
      pg.rows.forEach((it) => {
        const open = D.open.has(it.key);
        const toggle = () => { if (D.open.has(it.key)) D.open.delete(it.key); else D.open.add(it.key); draw(); };
        const gives = givesText(it), src = whereText(it), type = typeText(it);
        body.append(h('tr', { class: 'row-click' + (open ? ' open' : ''), onclick: (ev) => { if (!ev.target.closest('button')) toggle(); } },
          h('td', null, h('button', { type: 'button', class: 'row-btn', 'aria-expanded': String(open), onclick: toggle },
            h('span', { class: 'chev' }, icon('chevron')), ddbImg(it.icon, 28, 'dcm-icon gdb-icon'), h('span', null, tlName(it.name, it.ja, 'item'), it.sheetOnly ? h('span', { class: 'src-count', text: 'sheet only' }) : null,
              it.ja ? h('span', { class: 'key-jp', lang: 'ja', text: it.ja }) : null))),
          h('td', null, type || h('span', { class: 'zero', text: '—' })),
          // the start of the description when nothing else is known; greyed when that description is BABEL's translation
          h('td', { class: (gives ? 'gdb-gives' : 'zero') + (it.descJa && !it.all.length && !it.tarot.length && !it.soul.length && !it.gives.length ? ' tl-name' : ''), text: gives || '—' }),
          h('td', { class: src ? '' : 'zero', text: src || 'Not known' })));
        if (open) body.append(h('tr', { class: 'detail-row' }, h('td', { colspan: 4 }, gearDbDetail(it))));
      });
      if (!rows.length) body.append(h('tr', null, h('td', { colspan: 4, class: 'empty-row', text: words.length ? `Nothing matches “${D.q.trim()}”.` : 'No items in this group.' })));
      drawPager(pager, D, rows.length, pg, draw, () => page.querySelector('.tbl-wrap'));
    };
    const known = h('label', { class: 'gdb-check' }, h('input', { type: 'checkbox', checked: D.known ? true : null, onchange: (ev) => { D.known = ev.target.checked; D.pg = 0; drawChips(); draw(); } }), 'Only items with a known source');
    page.append(h('div', { class: 'toolbar' }, searchBox(D.q, 'Search a name, effect, description or place (e.g. limit break celu)', (v) => { D.q = v; D.pg = 0; draw(); }), known));
    page.append(kindChips, classChips);
    // The finer filters: labelled chip groups behind a "Filters" button (folded until opened or something is picked), each
    // chip with the count it would give with the other picks as they are. Who, Alignment and Range take one pick; Source
    // and Affinity take several (any of them). The level limit is a number box: typed, not picked.
    const fbtn = h('button', { type: 'button', class: 'btn btn-sm gdb-fbtn', 'aria-expanded': 'false' });
    const filters = h('div', { class: 'gdb-filters' });
    const drawFilters = () => {
      const n = fineCount();
      fbtn.replaceChildren(icon('sliders'), n ? `Filters · ${n}` : 'Filters');
      fbtn.setAttribute('aria-expanded', String(D.fopen));
      filters.hidden = !D.fopen;
      if (!D.fopen) return;
      const affs = [...new Set(items.filter((it) => it.affinity).map((it) => it.affinity))].sort((a, b) => a.localeCompare(b));
      const weapons = D.kind === 'Gear' && (D.cls === 'all' || D.cls === 'Weapon') || D.kind === 'all';
      const count = (key, val) => items.filter((it) => pass(it, D.kind, D.cls, { ...D, [key]: val })).length;
      const redraw = () => { D.pg = 0; drawChips(); drawFilters(); draw(); };
      // one group: its label, an "Any" chip that clears it, and a chip per choice; `multi` groups keep an array of picks
      const group = (label, key, options, multi) => {
        const picked = (v) => (multi ? D[key].includes(v) : D[key] === v);
        const pick = (v) => { if (multi) D[key] = picked(v) ? D[key].filter((x) => x !== v) : D[key].concat(v); else D[key] = picked(v) ? '' : v; redraw(); };
        const none = multi ? !D[key].length : !D[key];
        return h('div', { class: 'gdb-fgroup', role: 'group', 'aria-label': label }, h('span', { class: 'gdb-flabel', text: label }),
          h('div', { class: 'chips' }, chip(none, 'Any', count(key, multi ? [] : ''), () => { D[key] = multi ? [] : ''; redraw(); }),
            options.map(([v, t]) => chip(picked(v), t, count(key, multi ? (picked(v) ? D[key] : D[key].concat(v)) : v), () => pick(v)))));
      };
      filters.replaceChildren(...[
        group('Who', 'gender', [['Any', 'Both sexes'], ['Male', 'Male only'], ['Female', 'Female only']]),
        group('Alignment', 'align', [['Any', 'No restriction'], ['Law', 'Law only'], ['Neutral', 'Neutral only'], ['Chaos', 'Chaos only']]),
        group('Source', 'src', GDB_SRC, true),
        weapons ? group('Range', 'range', [['Close range', 'Close range'], ['Long range', 'Long range']]) : null,
        weapons ? group('Affinity', 'aff', affs.map((a) => [a, a]), true) : null,
        h('div', { class: 'gdb-fgroup' }, h('span', { class: 'gdb-flabel', text: 'Level' }),
          h('label', { class: 'gdb-check' }, 'Usable up to level', h('input', { type: 'number', class: 'input gdb-lv', min: 1, max: 999, placeholder: 'any', value: D.maxLv || '', 'aria-label': 'Usable up to this level',
            oninput: (ev) => { D.maxLv = ev.target.value.replace(/\D/g, ''); D.pg = 0; drawChips(); draw(); } })),
          n ? h('button', { type: 'button', class: 'btn btn-sm', onclick: () => { Object.assign(D, { gender: '', align: '', src: [], range: '', aff: [], maxLv: '' }); redraw(); } }, 'Clear filters') : null)].filter(Boolean));
    };
    fbtn.onclick = () => { D.fopen = !D.fopen; drawFilters(); };
    page.append(h('div', { class: 'toolbar' }, fbtn), filters);
    // the headers sort: a click sorts by that column, a second click turns it round
    const head = h('tr');
    const drawHead = () => head.replaceChildren(...GDB_COLS.map(([id, label]) => { const on = D.sort === id;
      return h('th', { scope: 'col', 'aria-sort': on ? (D.dir === 1 ? 'ascending' : 'descending') : null },
        h('button', { type: 'button', class: 'th-btn', 'data-tip': `Sort by ${label.toLowerCase()}${on ? ' the other way' : ''}.`, onclick: () => { if (on) D.dir = -D.dir; else { D.sort = id; D.dir = id === 'gives' ? -1 : 1; } D.pg = 0; drawHead(); draw(); } },   // Gives: the most first on the first click
          h('span', { text: label }), on ? h('span', { class: 'sort-mark', text: D.dir === 1 ? '▲' : '▼' }) : null)); }));
    page.append(h('div', { class: 'tbl-wrap' }, h('table', { class: 'tbl' }, h('thead', null, head), body)));
    page.append(pager);
    const real = items.filter((x) => x.id && !x.keyItem), keys = items.filter((x) => x.keyItem).length;
    page.append(h('p', { class: 'note', text: `${fmt(real.length)} items in the game data (${fmt(real.filter((x) => x.kind === 'Gear').length)} pieces of gear) and ${fmt(keys)} key items; ${fmt(real.filter((x) => x.known).length)} items with a known way to get them. Data made on ${D.data.generatedAt}. Community notes are from SnowSheet's gear reference.` }));
    if (fineCount()) D.fopen = true;   // picks kept from last time stay visible
    drawChips(); drawFilters(); drawHead(); draw();
    return page;
  }
  // ---------- Tools: Dungeon Database ----------
  // dungeons.json is made by "imagine-reader dungeons" (src\Build\Dungeons.cs): the server's zone, instance and drop
  // files for where each enemy is, the IMAGINE client's demon table for what it is. Only enemies that can't be
  // contracted are in it. Enemy pictures sit in dungeon-icons\ beside it (made locally from the client, not in git).
  const DUN_KINDS = [['all', 'All'], ['Dungeon', 'Dungeons'], ['Field', 'Fields'], ['other', 'Other places']];
  const STAT_NAMES = ['STR', 'MAG', 'VIT', 'INT', 'SPD', 'LUCK'], COMBAT_NAMES = ['CLS-R', 'LNG-R', 'Spell', 'Support', 'P.Def', 'M.Def'];
  function loadDungeons() {
    if (S.dun.state !== 'idle') return;
    S.dun.state = 'loading';
    fetch('dungeons.json', { cache: 'no-store' }).then((r) => (r.ok ? r.json() : null)).catch(() => null).then((d) => {
      S.dun.data = d; S.dun.state = d ? 'ready' : 'failed';
      if (d) {
        const sk = (id) => (d.skills[id] ? d.skills[id][0] : '');
        Object.values(d.mobs).forEach((m) => { m.hay = [m.name, m.ja || '', m.race || '', ...m.skills.map(sk), ...m.enemySkills.map(sk), ...m.traits.map(sk)].join(' ').toLowerCase(); });
        // a drop points at d.items[id] = [name, the game's description, picture]
        const item = (g) => { const [name, desc, icon, ja, dja] = d.items[g.i] || ['', '', null]; return Object.assign(g, { item: name, desc, icon, ja, dja }); };   // ja: BABEL translated the name
        (d.everywhere || []).forEach(item);   // the game-wide drops every enemy that inherits drops can give
        d.dungeons.forEach((x, i) => {
          x.key = i;
          (x.shared || []).forEach(item); (x.bossChest || []).forEach(item); (x.chests || []).forEach(item); (x.rewards || []).forEach(item);
          x.mobs.forEach((e) => { e.mob = d.mobs[e.m]; (e.drops || []).forEach(item); e.hay = [e.mob.hay, e.where || '', ...(e.drops || []).map((g) => g.item)].join(' ').toLowerCase(); });
          // chests and scripted rewards too: "god's majesty" finds End of Days
          x.hay = [x.name, x.from || '', ...[x.shared, x.bossChest, x.chests, x.rewards].flatMap((l) => (l || []).map((g) => g.item))].join(' ').toLowerCase();
          x.bosses = x.mobs.filter((e) => e.boss).length;
        });
      }
      if (S.page === 'dungeons') render();
    });
  }
  const dunIcon = (m) => h('span', { class: 'dcm-icon', 'aria-hidden': 'true' }, m.icon ? h('img', { src: m.icon, alt: '', width: 28, height: 28, loading: 'lazy', onerror: (ev) => ev.target.remove() }) : null);
  /** One enemy's card: what kind of enemy it is, its numbers, its affinities, what it can do and what it drops. */
  function dunMobDetail(e) {
    const m = e.mob, D = S.dun.data, box = h('div', { class: 'dun-mob' });
    box.append(h('div', { class: 'detail-meta' },
      m.race ? h('span', { class: 'tag', text: m.race }) : null, h('span', { class: 'tag', text: `Level ${m.level}` }),
      e.boss ? h('span', { class: 'tag tag-boss', text: 'Boss' }) : null,
      h('span', { class: 'tag', 'data-tip': e.talk ? 'You can talk to it, but it never joins you (no contract)' : 'Talking to it always fails', text: e.talk ? 'Talks, never joins' : 'Can\'t be talked to' }),
      e.xp ? h('span', { class: 'tag', text: `${fmt(e.xp)} XP` }) : null));
    const cell = (k, v, tip) => h('div', { class: 'dun-stat', 'data-tip': tip || null }, h('span', { class: 'dun-stat-k', text: k }), h('span', { class: 'dun-stat-v', text: fmt(v) }));
    box.append(h('div', { class: 'gdb-block' }, h('span', { class: 'q-key', text: 'Stats' }),
      h('div', { class: 'dun-stats' },
        cell('Max HP', m.hp, 'With the extra HP the game gives this enemy'), cell('Max MP', m.mp),
        m.stats.map((v, i) => cell(STAT_NAMES[i], v)), m.combat.map((v, i) => cell(COMBAT_NAMES[i], v)))));
    // affinities: weak (takes more), resists (takes less), and the chance to null, reflect or absorb a hit
    const groups = [['Weak to', m.aff.filter((a) => a.res < 0), (a) => `${a.el} ${Math.abs(a.res)}%`, 'dun-weak'],
      ['Resists', m.aff.filter((a) => a.res > 0), (a) => `${a.el} ${a.res}%`, 'dun-res'],
      ...['Null', 'Reflect', 'Absorb'].map((k) => [k === 'Null' ? 'Nulls' : k === 'Reflect' ? 'Reflects' : 'Absorbs', m.aff.filter((a) => a.kind === k), (a) => (a.pct >= 100 ? a.el : `${a.el} ${a.pct}%`), 'dun-nra'])]
      .filter((g) => g[1].length);
    box.append(h('div', { class: 'gdb-block' }, h('span', { class: 'q-key', text: 'Affinities' }),
      groups.length ? h('div', { class: 'dun-aff' }, groups.map(([label, list, txt, cls]) => h('div', { class: 'dun-aff-row' },
        h('span', { class: 'dun-aff-k', text: label }), h('span', { class: 'chips' }, list.map((a) => h('span', { class: 'tag ' + cls, text: txt(a) }))))))
        : h('span', { class: 'zero', text: 'No weakness or resistance' })));
    // skills as the game's icons; the name and description are in the tooltip. Many enemy-only skills have no icon in
    // the game at all (icon id 0): those show their name in a small tag, with the same tooltip, after the icons.
    const skillPic = (icon) => h('img', { class: 'dun-skill-pic', src: icon, alt: '', width: 32, height: 32, loading: 'lazy' });
    const skills = (label, ids, kind, tip) => {
      const list = ids.filter((id) => D.skills[id]).sort((a, b) => !D.skills[a][2] - !D.skills[b][2]);
      return list.length ? h('div', { class: 'gdb-block' }, h('span', { class: 'q-key', text: label, 'data-tip': tip || null }),
        h('div', { class: 'dun-skill-row' }, list.map((id) => { const [n, d, icon, ja, dja] = D.skills[id];   // ja / dja: the game's Japanese name / description when BABEL translated it
          const el = icon ? h('span', { class: 'dun-skill', tabindex: 0, role: 'img', 'aria-label': n }, skillPic(icon))
            : h('span', { class: 'tag dun-skill-name' + (ja ? ' tl-name' : ''), tabindex: 0, text: n });
          return withTip(el, () => h('div', null, tipHead(icon ? skillPic(icon) : null, n, kind), ja ? h('p', { class: 'tt-line tl-name', text: TL_TIP(ja, 'skill') }) : null, ...tlTipLines(d, dja))); }))) : null;
    };
    [skills('Skills', m.skills, 'Skill'), skills('Enemy-only skills', m.enemySkills, 'Enemy-only skill', 'Skills only this enemy version uses'),
      skills('Traits', m.traits, 'Trait', 'Its innate passive features')].forEach((b) => b && box.append(b));
    if (e.drops) box.append(h('div', { class: 'gdb-block' }, h('span', { class: 'q-key', text: 'Drops' }),
      h('ul', { class: 'gdb-list dun-drop-list' }, e.drops.map((g) => withTip(h('li', { tabindex: 0 }, h('span', { class: 'gdb-kind dun-rate', text: `${nf.format(g.rate)}%` }),
        dunItemPic(g, 20), tlName(g.item, g.ja, 'item'), g.max ? h('span', { class: 'src-count', text: `up to ${fmt(g.max)}` }) : null),
        () => itemCardTip(g.i, dunItemPic(g, 32), `Here: ${nf.format(g.rate)}% per kill${g.max ? ` · up to ${fmt(g.max)}` : ''}`) || h('div', null, tipHead(dunItemPic(g, 32), g.item, `${nf.format(g.rate)}% per kill${g.max ? ` · up to ${fmt(g.max)}` : ''}`), g.ja ? h('p', { class: 'tt-line tl-name', text: TL_TIP(g.ja, 'item') }) : null, ...tlTipLines(g.desc, g.dja)))))));
    return box;
  }
  /** A cell of the enemy list with one tag per affinity (weaknesses, or null / reflect / absorb), so nobody has to open the enemy. */
  const dunAffCell = (list, tag, none) => h('td', { class: 'dun-aff-cell' }, list.length ? h('span', { class: 'chips' }, list.map(tag)) : h('span', { class: 'zero', text: none }));
  function dunDetail(x, q, redraw) {
    const D = S.dun, box = h('div', { class: 'dun-detail' });
    const only = q && !x.hay.includes(q) ? x.mobs.filter((e) => e.hay.includes(q)) : x.mobs;   // a search for an enemy shows that enemy
    const body = h('tbody');
    only.forEach((e) => {
      const k = x.key + '|' + e.m + (e.boss ? '|b' : ''), open = D.mob.has(k);
      const toggle = () => { if (D.mob.has(k)) D.mob.delete(k); else D.mob.add(k); redraw(); };
      body.append(h('tr', { class: 'row-click' + (open ? ' open' : ''), onclick: (ev) => { if (!ev.target.closest('button')) toggle(); } },
        h('td', null, h('button', { type: 'button', class: 'row-btn', 'aria-expanded': String(open), onclick: toggle },
          h('span', { class: 'chev' }, icon('chevron')), dunIcon(e.mob),
          h('span', null, tlName(e.mob.name, e.mob.ja, 'demon'), e.boss ? h('span', { class: 'tag tag-boss', style: 'margin-left:8px', text: 'Boss' }) : null,
            h('span', { class: 'src-count', text: [e.mob.race, e.where].filter(Boolean).join(' · ') })))),
        h('td', { class: 'num', text: e.mob.level }),
        h('td', { class: 'num', text: fmt(e.mob.hp) }),
        dunAffCell(e.mob.aff.filter((a) => a.res < 0), (a) => h('span', { class: 'tag dun-weak', 'data-tip': `Weak to ${a.el}: takes ${Math.abs(a.res)}% more damage`, text: `${a.el} ${Math.abs(a.res)}%` }), 'No weakness'),
        dunAffCell(e.mob.aff.filter((a) => a.kind), (a) => h('span', { class: 'tag dun-nra', 'data-tip': `${a.kind === 'Null' ? 'Nulls' : a.kind === 'Reflect' ? 'Reflects' : 'Absorbs'} ${a.el}${a.pct >= 100 ? '' : `, ${a.pct}% of the time`}`,
          text: `${a.kind} ${a.el}${a.pct >= 100 ? '' : ` ${a.pct}%`}` }), 'None')));
      if (open) body.append(h('tr', { class: 'detail-row' }, h('td', { colspan: 5 }, dunMobDetail(e))));
    });
    const loot = dunLoot(x);
    const strip = (list, tip) => h('div', { class: 'dun-loot' }, list.map((l) => withTip(h('span', { class: 'dun-loot-item', tabindex: 0, role: 'img', 'aria-label': `${l.item}, up to ${nf.format(l.best)}%` },
      dunItemPic(l, 32), h('span', { class: 'dun-loot-rate', text: `${nf.format(l.best)}%` })), () => tip(l))));
    // the chests its events open: the boss chest at the end of a stage, and any others (a stage's events decide which)
    const chestList = (list, who) => (list || []).map((g) => ({ i: g.i, item: g.item, ja: g.ja, dja: g.dja, desc: g.desc, icon: g.icon, best: g.rate, max: g.max || 0, from: [{ who, rate: g.rate }] }));
    // and what its events give by script (x.rewards: End of Days' reward rolls), each with its chance per roll
    const boss = chestList(x.bossChest, 'boss'), other = chestList(x.chests, 'chest'), rewards = chestList(x.rewards, 'reward');
    // One short warning for all of it: the rates are the vanilla server data's, and a place whose enemies drop nothing
    // there (Dark Babel: IMAGINE sets those on its own server) says so. The game-wide drops any enemy anywhere can give
    // (d.everywhere) are left off the page: the user asked to ignore them.
    if (loot.length || boss.length || other.length || rewards.length) box.append(h('div', { class: 'notice notice-warn dun-warn' }, icon('alert'), h('span', null,
      h('b', { text: 'Vanilla rates, not IMAGINE\'s. ' }),
      loot.length ? '' : 'No enemy drops here in the server data (IMAGINE keeps its own). ',
      boss.length || other.length || rewards.length ? 'Each chest or reward item rolls on its own.' : '')));
    else box.append(h('div', { class: 'notice notice-warn dun-warn' }, icon('alert'), h('span', null, h('b', { text: 'No drops here in the server data. ' }), 'IMAGINE keeps its own.')));
    if (loot.length) box.append(h('div', { class: 'dun-loot-block' }, h('span', { class: 'q-key', text: `Drops here · ${loot.length === 1 ? '1 item' : `${loot.length} items`}` }), strip(loot, dunLootTip)));
    if (boss.length) box.append(h('div', { class: 'dun-loot-block' }, h('span', { class: 'q-key', 'data-tip': 'What the boss chest at the end of a stage can hold, over every stage of this place', text: `End chest · ${boss.length === 1 ? '1 item' : `${boss.length} items`}` }), strip(boss, dunLootTip)));
    if (other.length) box.append(h('div', { class: 'dun-loot-block' }, h('span', { class: 'q-key', 'data-tip': 'Other chests the events here open', text: `Other chests · ${other.length === 1 ? '1 item' : `${other.length} items`}` }), strip(other, dunLootTip)));
    if (rewards.length) box.append(h('div', { class: 'dun-loot-block' }, h('span', { class: 'q-key', 'data-tip': 'Given by the events here by script, not from a chest or an enemy (a reward roll, such as at the end of a run): the chance of each roll', text: `Rewards · ${rewards.length === 1 ? '1 item' : `${rewards.length} items`}` }), strip(rewards, dunLootTip)));
    box.append(h('table', { class: 'tbl dun-mobs' },
      h('thead', null, h('tr', null, h('th', { scope: 'col', text: 'Enemy' }), h('th', { scope: 'col', class: 'num', text: 'Level' }), h('th', { scope: 'col', class: 'num', text: 'Max HP' }),
        h('th', { scope: 'col', text: 'Weak to' }), h('th', { scope: 'col', text: 'Null, reflect, absorb' }))), body));
    return box;
  }
  /** An item's picture from the client (dungeon-icons\), or an empty box when it has none. */
  const dunItemPic = (g, size) => (g.icon ? h('img', { class: 'dun-item-pic', src: g.icon, alt: '', width: size, height: size, loading: 'lazy' })
    : h('span', { class: 'dun-item-pic', style: `width:${size}px;height:${size}px` }));
  /**
   * Everything that drops in a place, each item once: who drops it and at what rate (each enemy's own drops, and the
   * place's own drop sets that every enemy there shares), best chance first.
   */
  function dunLoot(x) {
    if (x.loot) return x.loot;
    const by = new Map();
    const add = (g, who) => {
      if (!by.has(g.i)) by.set(g.i, { i: g.i, item: g.item, ja: g.ja, dja: g.dja, desc: g.desc, icon: g.icon, best: 0, max: 0, from: [] });
      const l = by.get(g.i); l.best = Math.max(l.best, g.rate); l.max = Math.max(l.max, g.max || 0); l.from.push({ who, rate: g.rate });
    };
    (x.shared || []).forEach((g) => add(g, null));
    x.mobs.forEach((e) => (e.drops || []).forEach((g) => add(g, e)));
    return (x.loot = [...by.values()].sort((a, b) => b.best - a.best || a.item.localeCompare(b.item)));
  }
  function dunLootTip(l) {
    const rows = l.from.slice().sort((a, b) => b.rate - a.rate);
    {
      const reward = rows[0] && rows[0].who === 'reward', inChest = rows[0] && (rows[0].who === 'boss' || rows[0].who === 'chest');
      const card = itemCardTip(l.i, dunItemPic(l, 32), `Here: up to ${nf.format(l.best)}% ${reward ? 'per roll' : inChest ? 'per chest' : 'per kill'}${l.max > 1 ? ` · up to ${fmt(l.max)} at once` : ''}`);
      if (card) return card;
    }
    const shown = rows.slice(0, 10), more = rows.length - shown.length;
    const inChest = rows[0] && (rows[0].who === 'boss' || rows[0].who === 'chest'), reward = rows[0] && rows[0].who === 'reward';
    return h('div', null, tipHead(dunItemPic(l, 32), l.item, `Up to ${nf.format(l.best)}% ${reward ? 'per roll' : inChest ? 'per chest' : 'per kill'}${l.max > 1 ? ` · up to ${fmt(l.max)} at once` : ''}`),
      l.ja ? h('p', { class: 'tt-line tl-name', text: TL_TIP(l.ja, 'item') }) : null,
      ...tlTipLines(l.desc, l.dja),
      h('div', { class: 'tt-sec', text: rows[0] && rows[0].who === 'all' ? 'Dropped by' : reward ? 'Given by' : rows[0] && (rows[0].who === 'boss' || rows[0].who === 'chest') ? 'Found in' : 'Dropped here by' }),
      shown.map((r) => h('div', { class: 'tt-line dun-tt-row' }, h('span', { text: r.who === 'all' ? 'Any enemy, in any place' : r.who === 'boss' ? 'The boss chest at the end of a stage' : r.who === 'chest' ? 'A chest the events here open' : r.who === 'reward' ? 'A reward roll of the events here' : r.who ?`${r.who.mob.name}${r.who.boss ? ' (boss)' : ''} · level ${r.who.mob.level}` : 'Every enemy here' }), h('b', { text: `${nf.format(r.rate)}%` }))),
      more > 0 ? h('div', { class: 'tt-line', text: `and ${more} more` }) : null);
  }
  /**
   * A place's enemies at a glance: small pictures beside its name, bosses first, then the strongest; each picture once
   * (enemy versions of one demon share it). Hovering one names it; "+N" says how many more there are.
   */
  const DUN_FACES = 8;
  function dunFaces(x) {
    if (!x.faces) {
      const seen = new Set();
      x.faces = [...x.mobs].sort((a, b) => !!b.boss - !!a.boss || b.mob.level - a.mob.level)
        .filter((e) => e.mob.icon && !seen.has(e.mob.icon) && seen.add(e.mob.icon));
    }
    if (!x.faces.length) return null;
    const more = x.faces.length - DUN_FACES;
    return h('span', { class: 'dun-faces', 'aria-hidden': 'true' },
      x.faces.slice(0, DUN_FACES).map((e) => h('img', { class: 'dun-face' + (e.boss ? ' boss' : ''), src: e.mob.icon, alt: '', width: 22, height: 22, loading: 'lazy',
        'data-tip': `${e.mob.name}${e.boss ? ' (boss)' : ''} · level ${e.mob.level}`, onerror: (ev) => ev.target.remove() })),
      more > 0 ? h('span', { class: 'dun-faces-more', 'data-tip': `${more} more enemy ${more === 1 ? 'picture' : 'pictures'}; open the place for all its enemies`, text: `+${more}` }) : null);
  }
  function pageDungeons() {
    const D = S.dun, page = h('div', { class: 'page page-dcm' });
    loadDungeons();
    loadGearDb();   // the drops' tooltips are the Item Database's entries (itemCardTip)
    page.append(pageHead('Every dungeon and the enemies in it that can\'t be contracted. Click a dungeon for its enemies, and an enemy for its stats, affinities, skills and traits.'));
    if (D.state !== 'ready') {
      page.append(h('p', { class: 'note', text: D.state === 'failed' ? 'The dungeon list could not be loaded (dungeons.json is missing beside the page).' : 'Loading the dungeons…' }));
      return page;
    }
    page.append(h('div', { class: 'notice' }, icon('info'), h('span', null, h('b', { text: 'Read from game data, not checked on IMAGINE. ' }),
      'Names, stats, skills and traits are the IMAGINE client\'s. Which enemies are where, their levels, whether they talk and their drops come from the public server data, so IMAGINE may have changed them. Stats are the enemy\'s own at that level, before its passive skills and traits.')));
    const list = D.data.dungeons;
    const inKind = (x, k) => (k === 'all' ? true : k === 'other' ? x.kind !== 'Dungeon' && x.kind !== 'Field' : x.kind === k);
    const chips = h('div', { class: 'chips', role: 'group', 'aria-label': 'Filter by kind of place' });
    const body = h('tbody');
    const drawChips = () => chips.replaceChildren(...DUN_KINDS.map(([id, label]) =>
      h('button', { type: 'button', class: 'chip-btn', 'aria-pressed': String(D.kind === id), onclick: () => { D.kind = id; drawChips(); draw(); } },
        label, h('span', { class: 'count', text: list.filter((x) => inKind(x, id)).length }))));
    // Grouped: under the place you enter from (a town, a lobby: "Dark Babel", "Celu Tower" for its Bronze, Silver,
    // Gold...). Places the data gives no entrance for close the list, by kind.
    const NO_DOOR = { Dungeon: 'Other dungeons (entrance not in the data)', Field: 'Fields', other: 'Other places' };
    const groupOf = (x) => x.from || NO_DOOR[x.kind === 'Dungeon' || x.kind === 'Field' ? x.kind : 'other'];
    const isDoorless = (g) => Object.values(NO_DOOR).includes(g);
    const draw = () => {
      body.replaceChildren();
      const q = D.q.trim().toLowerCase();
      const rows = list.filter((x) => inKind(x, D.kind) && (!q || x.hay.includes(q) || x.mobs.some((e) => e.hay.includes(q))));
      const placeRow = (x) => {
        const open = D.open.has(x.key);
        const toggle = () => { if (D.open.has(x.key)) D.open.delete(x.key); else D.open.add(x.key); draw(); };
        const hits = q && !x.hay.includes(q) ? x.mobs.filter((e) => e.hay.includes(q)).length : 0;
        const door = !D.group && x.from && x.from !== x.name;   // in a group its entrance is the heading
        body.append(h('tr', { class: 'row-click' + (open ? ' open' : '') + (D.group ? ' dun-in-group' : ''), onclick: (ev) => { if (!ev.target.closest('button')) toggle(); } },
          h('td', null, h('button', { type: 'button', class: 'row-btn', 'aria-expanded': String(open), onclick: toggle },
            h('span', { class: 'chev' }, icon('chevron')),
            h('span', null, x.name, door ? h('span', { class: 'src-count', text: `entered from ${x.from}` }) : null, hits ? h('span', { class: 'src-count', text: hits === 1 ? '1 matching enemy' : `${hits} matching enemies` }) : null)),
            dunFaces(x)),
          h('td', null, h('span', { class: 'tag', text: x.kind })),
          h('td', { class: 'num', text: x.levels }),
          h('td', { class: 'num' }, String(x.mobs.length), x.bosses ? h('span', { class: 'src-count', text: x.bosses === 1 ? '1 boss' : `${x.bosses} bosses` }) : null)));
        if (open) body.append(h('tr', { class: 'detail-row' }, h('td', { colspan: 4 }, dunDetail(x, q, draw))));
      };
      if (!D.group) rows.forEach(placeRow);
      else {
        const groups = new Map();
        rows.forEach((x) => { const g = groupOf(x); if (!groups.has(g)) groups.set(g, []); groups.get(g).push(x); });
        [...groups.keys()].sort((a, b) => isDoorless(a) - isDoorless(b) || (isDoorless(a) ? Object.values(NO_DOOR).indexOf(a) - Object.values(NO_DOOR).indexOf(b) : !/^\w/.test(a) - !/^\w/.test(b) || a.localeCompare(b)))
          .forEach((g) => {
            const places = groups.get(g), shut = !q && !D.shown.has(g);   // a search opens every group it finds something in
            const fold = () => { if (D.shown.has(g)) D.shown.delete(g); else D.shown.add(g); draw(); };
            const lv = places.flatMap((x) => x.levels.split('–').map(Number)), lo = Math.min(...lv), hi = Math.max(...lv);
            body.append(h('tr', { class: 'dun-group-row' + (shut ? '' : ' open'), onclick: (ev) => { if (!ev.target.closest('button')) fold(); } },
              h('td', { colspan: 4 }, h('button', { type: 'button', class: 'row-btn', 'aria-expanded': String(!shut), onclick: fold },
                h('span', { class: 'chev' }, icon('chevron')),
                h('span', null, isDoorless(g) ? g : h('span', null, h('span', { class: 'dun-group-pre', text: 'Entered from ' }), g),
                  h('span', { class: 'src-count', text: `${places.length === 1 ? '1 place' : `${places.length} places`} · level ${lo === hi ? lo : `${lo}–${hi}`}` }))))));
            if (!shut) places.forEach(placeRow);
          });
      }
      if (!rows.length) body.append(h('tr', null, h('td', { colspan: 4, class: 'empty-row', text: q ? `Nothing matches “${D.q.trim()}”.` : 'No place in this group.' })));
    };
    const grouping = h('label', { class: 'gdb-check', 'data-tip': 'List each dungeon under the town or area you enter it from' },
      h('input', { type: 'checkbox', checked: D.group ? true : null, onchange: (ev) => { D.group = ev.target.checked; store.set('dun-group', D.group ? '1' : '0'); draw(); } }), 'Group by entrance');
    page.append(h('div', { class: 'toolbar' }, searchBox(D.q, 'Search a dungeon, enemy, skill, drop or entrance', (v) => { D.q = v; draw(); }), chips, grouping));
    page.append(h('div', { class: 'tbl-wrap' }, h('table', { class: 'tbl' },
      h('thead', null, h('tr', null, h('th', { scope: 'col', text: 'Place' }), h('th', { scope: 'col', text: 'Kind' }), h('th', { scope: 'col', class: 'num', text: 'Levels' }), h('th', { scope: 'col', class: 'num', text: 'Enemies' }))), body)));
    page.append(h('p', { class: 'note', text: `${list.length} places, ${Object.keys(D.data.mobs).length} enemy kinds that can't be contracted. Data made on ${D.data.generatedAt}. An enemy can't be contracted when talking to it always fails, or when it talks but never joins. Drop rates are the chance per kill.` }));
    drawChips(); draw();
    return page;
  }

  // ---------- Tools: Demon Database ----------
  // demon-db.json is made by "imagine-reader demondb" (src\Build\DemonDb.cs): the compendium's demons with their stats,
  // affinities, features, skills, tarot and soul from the IMAGINE client, and where each can be contracted from the
  // server's zone files. Enemy-only copies (dungeon and boss versions) are folded into their demon, not listed. Pictures
  // are in demon-db-icons\ beside this file.
  const DDB_KINDS = [['contract', 'Can be contracted'], ['other', 'Not contracted anywhere'], ['all', 'All demons']];
  function loadDemonDb() {
    if (S.ddb.state !== 'idle') return;
    S.ddb.state = 'loading';
    fetch('demon-db.json', { cache: 'no-store' }).then((r) => (r.ok ? r.json() : null)).catch(() => null).then((d) => {
      S.ddb.data = d; S.ddb.state = d ? 'ready' : 'failed';
      if (d) d.demons.forEach((x) => {
        const sk = (id) => d.skills[id] || { name: 'A skill the game data does not name', desc: '' };
        x.hay = [x.name, x.race, x.align, ...x.traits.concat(x.mt ? x.mt.traits : []).map((t) => sk(t).name), ...x.skills.map(([id]) => sk(id).name),
          ...[x.tarot, x.soul].filter(Boolean).flatMap((e) => [e.name, ...e.lines]), ...x.contract.map((c) => c.zone), x.variantOf || '', ...(x.get || []).map((g) => g.text)].join(' ').toLowerCase();
      });
      if (S.page === 'ddb' || S.page === 'skills' || S.page === 'rank' || S.page === 'rebirth' || S.page === 'fusion') render();   // the Fusion Planner's tree: where demons can be contracted
    });
    loadDcm();   // a demon's DCM, when there is one, links to the DCM Locator
  }
  /** Stats at a level, as the server works them out (CharacterManager::GetDemonBaseStats, then CalculateDependentStats for a demon). */
  function ddbStats(x, level) {
    const s = x.stats.slice(), g = (S.ddb.data.growth || {})[x.growth];
    if (g) {
      const boost = Math.floor((level + 3) / 4), stage = Math.floor((boost - 1) / 5);
      const add = (k, n) => { if (g[k] && n > 0) for (let i = 0; i < 6; i++) s[i] += g[k][i] * n; };
      // the server's table: A for every boost up to stage 1, then A x6, B x5, C x5, D for the rest
      if (stage <= 1) add(0, boost);
      else if (stage === 2) { add(0, 6); add(1, boost - 6); }
      else if (stage === 3) { add(0, 6); add(1, 5); add(2, boost - 11); }
      else { add(0, 6); add(1, 5); add(2, 5); add(3, boost - 16); }
    }
    const [str, mag, vit, int, spd] = s, hp = s[6], mp = s[7], L = level;
    const out = s.slice();
    out[6] = hp + Math.ceil(hp * 0.03 * L) + Math.ceil(str * 0.3) + Math.ceil((hp * 0.01 + 0.5) * vit);
    out[7] = mp + Math.ceil(mp * 0.03 * L) + Math.ceil(mag * 0.3) + Math.ceil((mp * 0.01 + 0.5) * int);
    const dep = (base, stat, k) => base + Math.round(stat * k + 0.5 + L * 0.1);
    out[8] = dep(s[8], str, 0.5); out[9] = dep(s[9], spd, 0.5); out[10] = dep(s[10], mag, 0.5); out[11] = dep(s[11], int, 0.5);
    out[12] = dep(s[12], vit, 0.1); out[13] = dep(s[13], int, 0.1);
    return out;
  }
  const ddbImg = (src, size, cls) => { const box = h('span', { class: cls || 'dcm-icon', 'aria-hidden': 'true' });
    if (src) box.append(h('img', { src, alt: '', width: size, height: size, loading: 'lazy', onerror: (ev) => ev.target.remove() })); return box; };
  const ddbFace = (x) => ddbImg(x.icon || (S.ddb.data && S.ddb.data.noIcon), 28);
  const ddbAlign = (x) => h('span', { class: 'tag tag-align ' + alignClass(x.align), text: x.align });
  const ddbLv = (c) => (c.min === c.max ? `Lv ${c.min}` : `Lv ${c.min}–${c.max}`);
  const ddbMoon = (mask) => 'Moon: ' + [...Array(16).keys()].filter((i) => mask & (1 << i)).map((i) => (i === 0 ? 'new' : i === 8 ? 'full' : i < 8 ? `${i}/8 waxing` : `${16 - i}/8 waning`)).join(', ');
  /** The Fusion Planner's demons for a Demon Database entry: its own id, and the ids the database merged into it (same
   *  name, race and base level, such as Asura Cerberus 550 and 1302). Mitama-fused forms are the database's `mt`. */
  const ddbFusionIds = (E, x) => E.demons.filter((f) => !f.mitamaFused && (f.id === x.id || (f.name === x.name && f.raceName === x.race && f.lvl === x.level)));
  /** "Where to get it": every way the game data gives, first in a demon's details. Fusion comes from the Fusion
   *  Planner's engine (fusion-data.json): how many recipes make it, a link to them, and each special fusion with the
   *  plugin it needs and where to get that plugin. */
  function ddbWhere(x, block) {
    const where = h('ul', { class: 'gdb-list gdb-get' });
    // the kind on the left, everything about it in one column beside it (notes and plugin boxes stay out of the kind's column)
    const row = (kind, ...kids) => where.append(h('li', { class: 'ddb-get-row' }, h('span', { class: 'gdb-kind', text: kind }), h('div', { class: 'ddb-get-body' }, ...kids)));
    x.contract.forEach((c) => row('Contract', h('b', { text: c.zone }), h('span', { class: 'gdb-rate', text: ddbLv(c) }),
      c.moon ? h('span', { class: 'tag', style: 'margin-left:8px', text: ddbMoon(c.moon) }) : null, c.season ? h('span', { class: 'tag', style: 'margin-left:8px', text: c.season + ' event' }) : null));
    const dcm = S.dcm.data && S.dcm.data.items.find((it) => it.devilId === x.id);
    if (dcm) row('DCM', h('b', { text: dcm.lines && dcm.lines[0] ? `${dcm.lines[0].kind}: ${dcm.lines[0].text}` : 'No source in the game data' }),
      h('button', { type: 'button', class: 'btn btn-sm', style: 'margin-left:10px', onclick: () => { S.dcm.q = x.name; S.dcm.kind = 'all'; S.dcm.open = new Set([dcm.id]); go('dcm'); } }, 'Open in DCM Locator'));

    // Fusion, from the planner's engine. Until it is loaded and has worked out every fusion once (about a second), say so.
    loadFusion();
    const U = S.fus, E = U.state === 'ready' ? U.eng : null;
    let specials = null;
    if (U.state === 'loading' || (E && !E.built())) {
      row('Fusion', h('span', { class: 'note' }, h('span', { class: 'spin ink', style: 'margin-right:8px;vertical-align:-2px' }), 'Working out which fusions make it…'));
      if (E) setTimeout(() => { if (!E.built()) { E.build(); if (S.page === 'ddb') render(); } }, 30);
    } else if (E) {
      const mine = ddbFusionIds(E, x), ids = new Set(mine.map((f) => f.id));
      specials = E.F.specials.filter((sp) => ids.has(sp.result));
      const R = mine.map((f) => E.recipes(f)), two = R.reduce((n, r) => n + r.two.length, 0), three = R.reduce((n, r) => n + r.three.length, 0);
      const target = mine.find((f) => E.canMake(f));
      if (target) {
        const cheap = R.flatMap((r) => r.two)[0] || R.flatMap((r) => r.three)[0];
        const ways = [two ? `${fmt(two)} two-demon ${two === 1 ? 'recipe' : 'recipes'}` : '', three ? `${fmt(three)} ${three === 1 ? 'tri-fusion' : 'tri-fusions'}` : '',
          specials.length ? `${specials.length} special ${specials.length === 1 ? 'fusion' : 'fusions'}` : ''].filter(Boolean);
        row('Fusion', h('span', null, h('b', { text: 'Can be fused' }), h('span', { class: 'src-count', text: ways.join(' · ') }),
          cheap ? h('div', { class: 'note', text: `Cheapest at base levels: ${cheap.map((d) => `${d.label} (Lv ${d.lvl})`).join(' + ')}` }) : null),
          h('button', { type: 'button', class: 'btn btn-sm', style: 'margin-left:10px', onclick: () => {
            fusOpenTree(target); go('fusion'); } }, 'Open in Fusion Planner'));
      }
      // each special fusion: its demons and the plugin it needs; then each plugin once, with the ways to get it
      const plugName = (sp) => { const p = E.F.plugins[String(sp.plugin)]; return p && p.name ? `Needs the key item “${p.name}”` : 'Needs a fusion plugin'; };
      specials.forEach((sp) => row('Special fusion', h('span', null,
        h('b', { text: sp.src.map(([id, variant]) => { const d = E.byId.get(id); return (d ? d.name : 'A demon the data does not name') + (variant ? ' (or a variant of it)' : ''); }).join(' + ') }),
        h('div', { class: 'note', text: sp.plugin ? plugName(sp) : 'No plugin needed.' }))));
      [...new Map(specials.filter((sp) => sp.plugin).map((sp) => [sp.plugin, sp])).values()].forEach((sp) => row('Plugin', pluginLine(E, sp)));
    } else if (x.fusion) row('Fusion', h('span', null, `Normal two-demon fusion: a ${x.race} result picked by level`));   // the fusion data is missing

    // a variant: an exchange item, a special fusion or a gift (demon-db.json "get"), each with what the game data says about
    // it. Special fusions are left out when the planner's list above has them (it also says where to get the plugin).
    (x.get || []).filter((g) => !(specials && specials.length && g.kind === 'Special fusion')).forEach((g) => row(g.kind, h('span', null, h('b', { text: g.text }),
      g.note ? h('div', { class: 'note', text: g.note }) : null,
      g.from ? (g.from.length ? [g.from.length > 1 ? h('div', { class: 'any-one', text: `The item comes from any one of these ${g.from.length}:` }) : null,
        h('ul', { class: 'gdb-list gdb-sub-list' }, g.from.map((f) => h('li', null, questLine(f))))] : h('div', { class: 'note', text: 'The data does not say where this item comes from.' })) : null)));
    // every line is a separate way to get the demon (a "+" inside one is fused together): say that any one is enough
    const n = where.childNodes.length;
    return block('Where to get it', n ? h('div', null, n > 1 ? h('p', { class: 'any-one', text: 'Each line is a separate way to get it: any one is enough.' }) : null, where)
      : h('span', { class: 'zero', text: 'The game data names no place to contract it, no fusion and no DCM.' }));
  }
  function ddbDetail(x) {
    const D = S.ddb, d = D.data, box = h('div', { class: 'q-detail ddb-detail' });
    const sk = (id) => d.skills[id] || { name: 'A skill the game data does not name', desc: '' };
    const block = (label, ...kids) => h('div', { class: 'gdb-block' }, h('span', { class: 'q-key', text: label }), h('div', null, ...kids));
    box.append(h('div', { class: 'detail-meta' }, h('span', { class: 'tag', text: x.race }), h('span', { class: 'tag', text: `Base Lv ${x.level}` }), ddbAlign(x),
      x.gender && !/^(None|Na)$/i.test(x.gender) ? h('span', { class: 'tag', text: x.gender }) : null,
      x.fusion ? h('span', { class: 'tag', 'data-tip': 'The game\'s fusion picks it by level within its race.', text: 'Normal fusion' }) : null,
      x.mitama ? h('span', { class: 'tag', text: 'Mitama' }) : null,
      x.variant ? h('span', { class: 'tag', 'data-tip': 'A version of a demon with its own name, level, stats and skills. The compendium counts it with the demon it is a version of, not on its own.', text: x.variantOf ? `Variant of ${x.variantOf}` : 'Variant' }) : null,
      x.counted || x.variant ? null : h('span', { class: 'tag tag-off', 'data-tip': 'The compendium lists it but does not count it toward its totals.', text: 'Not counted in the compendium' })));
    box.append(ddbWhere(x, block));

    // Normal or Mitama-fused: Mitama fusion turns it into another demon type with its own features, stats and
    // affinities (x.mt; skills only when they differ). The view switches everything below that the Mitama form changes.
    const mv = !!(x.mt && D.mtView.has(x.id));
    const v = mv ? Object.assign({}, x, x.mt, { skills: x.mt.skills || x.skills }) : x;
    if (x.mt) box.append(h('div', { class: 'chips ddb-form', role: 'group', 'aria-label': 'Which form' }, [['', 'Normal'], ['m', 'Mitama-fused']].map(([k, label]) =>
      h('button', { type: 'button', class: 'chip-btn', 'aria-pressed': String(mv === !!k), onclick: () => { if (k) D.mtView.add(x.id); else D.mtView.delete(x.id); box.replaceWith(ddbDetail(x)); } }, label))));

    // Stats, at any level from its base level (where it starts when contracted or fused) to 99
    const lvKey = x.id + (mv ? 'm' : '');
    const lv = Math.max(v.level, Math.min(99, D.level[lvKey] || v.level));
    const grid = h('div', { class: 'ddb-stats' });
    const drawStats = (L) => { const s = ddbStats(v, L);
      grid.replaceChildren(...d.statNames.map((n, i) => h('div', { class: 'ddb-stat' }, h('span', { class: 'q-key', text: n }), h('b', { text: fmt(s[i]) })))); };
    const num = h('input', { class: 'input ddb-lvnum', type: 'number', min: v.level, max: 99, value: lv, 'aria-label': 'Level' });
    const range = h('input', { type: 'range', class: 'ddb-range', min: v.level, max: 99, value: lv, 'aria-label': 'Level' });
    const setLv = (L) => { L = Math.max(v.level, Math.min(99, Math.round(Number(L) || v.level))); D.level[lvKey] = L; num.value = L; range.value = L; drawStats(L); };
    num.oninput = () => { if (num.value !== '') setLv(num.value); }; num.onchange = () => setLv(num.value); range.oninput = () => setLv(range.value);
    drawStats(lv);
    box.append(block('Stats', h('div', { class: 'ddb-lv' }, h('span', { class: 'q-key', text: 'At level' }), num, range,
      h('span', { class: 'src-count', text: mv ? 'The Mitama-fused demon\'s own base stats; no Demon Force, rebirth or Mitama runes.' : 'No Demon Force, rebirth or Mitama: the stats of a fresh demon of this level.' })), grid));

    // Affinities: damage taken is multiplied by (100% − the value); natural null / reflect / drain chances beside it
    const NRA = { 1: 'Null', 2: 'Reflect', 3: 'Drain' };
    const aff = h('div', { class: 'ddb-aff' }, d.elements.map((e, i) => {
      const r = v.res[i], n = v.nra[i], kind = n > 0 ? NRA[n % 10] : null;
      const cls = r < 0 ? ' weak' : r >= 100 ? ' null' : r > 0 ? ' strong' : '';
      return h('div', { class: 'ddb-aff-cell' + cls, 'data-tip': `${e}: ${r < 0 ? `takes ${-r}% more damage` : r > 0 ? `takes ${Math.min(r, 100)}% less damage` : 'normal damage'}${kind ? `; ${Math.floor(n / 10)}% chance to ${kind.toLowerCase()} it` : ''}` },
        h('span', { class: 'q-key', text: e }), h('b', { text: r ? `${r > 0 ? '' : '−'}${Math.abs(r)}%` : '—' }), kind ? h('span', { class: 'ddb-nra', text: `${kind} ${Math.floor(n / 10)}%` }) : null);
    }));
    box.append(block('Affinities', aff, h('p', { class: 'note', style: 'margin-top:6px', text: 'Resistance in %: the damage it takes is cut by that much. A negative number is a weakness.' })));

    const skillRow = (id, tag, inline) => { const s = sk(id);
      return h('li', { class: 'ddb-skill', 'data-tip': inline ? null : s.desc || null }, ddbImg(s.icon, 24, 'skill-ico'),
        h('span', null, sklName(s, 'b'), tag ? h('span', { class: 'src-count', text: tag }) : null, inline && s.desc ? h('small', { class: 'ddb-desc' + (s.descJa ? ' tl-name' : ''), 'data-tip': s.descJa ? TL_DESC_TIP(s.descJa) : null, text: s.desc }) : null)); };
    // Features. The Mitama-fused form keeps most and gains or swaps one: those are "Mitama features", shown in both views.
    const gained = x.mt ? x.mt.traits.filter((t) => !x.traits.includes(t)) : [], lost = x.mt ? x.traits.filter((t) => !x.mt.traits.includes(t)) : [];
    box.append(block('Features', v.traits.length ? h('ul', { class: 'gdb-list' }, v.traits.map((t) => skillRow(t, mv && gained.includes(t) ? 'Mitama feature' : '', true))) : h('span', { class: 'zero', text: 'None' })));
    if (x.mt && !mv) box.append(block('Mitama features', gained.length
      ? h('div', null, h('ul', { class: 'gdb-list' }, gained.map((t) => skillRow(t, '', true))),
        h('p', { class: 'note', style: 'margin-top:4px', text: lost.length ? `Mitama-fused, it gets ${gained.length === 1 ? 'this' : 'these'} in place of ${lost.map((t) => sk(t).name).join(', ')}.` : `Mitama-fused, it gets ${gained.length === 1 ? 'this' : 'these'} on top of its own features.` }))
      : h('span', { class: 'zero', text: 'Mitama-fused, it keeps the same features.' })));
    box.append(block('Skills', v.skills.length ? h('ul', { class: 'gdb-list ddb-skills' }, v.skills.map(([id, at]) => skillRow(id, at ? `learns at Lv ${at}` : 'knows it'))) : h('span', { class: 'zero', text: 'None in the game data' })));

    // Tarot and soul: what its crystal gives when fused into gear
    const ench = (label, e) => block(label, !e ? h('span', { class: 'zero', text: 'None in the game data' }) : h('div', null,
      h('div', null, h('b', { text: e.name }), h('span', { class: 'src-count', text: [e.slots.join(', '), e.level ? `equip level ${e.level}` : ''].filter(Boolean).join(' · ') })),
      h('ul', { class: 'gdb-list', style: 'margin-top:3px' }, e.lines.map((l) => h('li', { class: /^\s/.test(l) ? 'gdb-sub' : null, text: l.trim() })))));
    box.append(ench('Tarot', x.tarot), ench('Soul', x.soul));
    const crystal = (x.tarot || x.soul || {}).crystal;
    if (crystal) box.append(h('p', { class: 'note', text: `Both come from ${crystal}.` }));

    if (x.enemy.length) box.append(block('Enemy only', h('div', { class: 'note', style: 'margin-bottom:4px', text: 'It appears here too, but cannot be talked into a contract (dungeon and boss versions).' }),
      h('ul', { class: 'gdb-list ddb-enemy' }, x.enemy.map((c) => h('li', null, c.zone, h('span', { class: 'src-count', text: ddbLv(c) }))))));
    return box;
  }
  function pageDemonDb() {
    const D = S.ddb, page = h('div', { class: 'page page-dcm' });
    loadDemonDb();
    page.append(pageHead('Every demon you can own: stats, affinities, features, skills, tarot and soul, and where to contract it. Click a demon for the details.'));
    if (D.state !== 'ready') {
      page.append(h('p', { class: 'note', text: D.state === 'failed' ? 'The demon list could not be loaded (demon-db.json is missing beside the page).' : 'Loading the demons…' }));
      return page;
    }
    page.append(h('div', { class: 'notice' }, icon('info'), h('span', null, h('b', { text: 'Read from game data, not checked on IMAGINE. ' }),
      'Demons, stats, affinities, skills, tarot and soul are the IMAGINE client\'s. Where each spawns comes from the public server data, so IMAGINE may have changed it. Variants (Lucky, Inexperienced, Open-minded and the like) are listed on their own, with how to get them. Dungeon and boss copies of a demon are not: they show under “Enemy only”.')));
    const all = D.data.demons;
    const inKind = (x, k) => (k === 'all' ? true : k === 'contract' ? x.contract.length > 0 : !x.contract.length);
    // the Race and Alignment dropdowns, and "Show variants" (a demon a link opened is shown even when it is a variant)
    const inPick = (x) => (!D.race || x.race === D.race) && (!D.align || x.align === D.align) && (D.variants || !x.variant);
    const races = [...new Set(all.map((x) => x.race))].sort();
    const chips = h('div', { class: 'chips', role: 'group', 'aria-label': 'Which demons' });
    const body = h('tbody'), count = h('p', { class: 'note' }), onlyBar = h('div');
    const drawChips = () => chips.replaceChildren(...DDB_KINDS.map(([id, label]) =>
      h('button', { type: 'button', class: 'chip-btn', 'aria-pressed': String(!D.only && D.kind === id), onclick: () => { D.kind = id; D.only = null; drawChips(); draw(); } },
        label, h('span', { class: 'count', text: all.filter((x) => inKind(x, id) && inPick(x)).length }))));
    const SORTS = { level: (a, b) => a.level - b.level || a.name.localeCompare(b.name), name: (a, b) => a.name.localeCompare(b.name), race: (a, b) => a.race.localeCompare(b.race) || a.level - b.level };
    const draw = () => {
      body.replaceChildren();
      const words = D.q.trim().toLowerCase().split(/\s+/).filter(Boolean);
      // opened from a link (the Skill Database): that one demon only. A search by its name would also list every
      // demon whose skills, features or race have the word ("Power" matches 57), burying the one clicked.
      const only = D.only != null ? all.find((x) => x.id === D.only) : null;
      onlyBar.replaceChildren(...(only ? [h('div', { class: 'notice ddb-only' }, icon('info'), h('span', null, 'Showing only ', h('b', { text: only.name }), '.'),
        h('button', { type: 'button', class: 'btn btn-sm', onclick: () => { D.only = null; drawChips(); draw(); } }, 'Show all demons'))] : []));
      const rows = only ? [only] : all.filter((x) => inKind(x, D.kind) && inPick(x) && words.every((w) => x.hay.includes(w))).sort(SORTS[D.sort] || SORTS.level);
      const hidden = only || D.variants ? 0 : all.filter((x) => x.variant && inKind(x, D.kind) && words.every((w) => x.hay.includes(w))).length;
      rows.forEach((x) => {
        const open = D.open.has(x.id);
        const toggle = () => { if (D.open.has(x.id)) D.open.delete(x.id); else D.open.add(x.id); draw(); };
        const c0 = x.contract[0];
        const where = c0 ? h('span', null, c0.zone, h('span', { class: 'src-count', text: ddbLv(c0) + (x.contract.length > 1 ? ` · and ${x.contract.length - 1} more` : '') }))
          : h('span', { class: 'zero', text: x.fusion ? 'Fusion only' : x.get && x.get.length ? ({ Gift: 'Event or NPC gift' }[x.get[0].kind] || x.get[0].kind) : '—' });
        const ench = [x.tarot && x.tarot.lines[0], x.soul && x.soul.lines[0]].filter(Boolean);
        body.append(h('tr', { class: 'row-click' + (open ? ' open' : ''), onclick: (ev) => { if (!ev.target.closest('button')) toggle(); } },
          h('td', null, h('button', { type: 'button', class: 'row-btn', 'aria-expanded': String(open), onclick: toggle },
            h('span', { class: 'chev' }, icon('chevron')), ddbFace(x), h('span', null, x.name, h('span', { class: 'src-count', text: x.race })))),
          h('td', { class: 'num', text: x.level }),
          h('td', null, ddbAlign(x)),
          h('td', null, where),
          h('td', { class: ench.length ? 'gdb-gives' : 'zero', text: ench.join(' · ') || '—' })));
        if (open) body.append(h('tr', { class: 'detail-row' }, h('td', { colspan: 5 }, ddbDetail(x))));
      });
      if (!rows.length) body.append(h('tr', null, h('td', { colspan: 5, class: 'empty-row', text: words.length ? `Nothing matches “${D.q.trim()}”.` : 'No demon in this group.' })));
      count.textContent = `Showing ${rows.length} of ${all.length} demons${hidden ? ` (${hidden} ${hidden === 1 ? 'variant' : 'variants'} hidden)` : ''}. ${all.filter((x) => x.contract.length).length} can be contracted somewhere. Data made on ${D.data.generatedAt}.`;
    };
    const sel = (label, value, options, onchange) => h('label', { class: 'gdb-check' }, label,
      h('select', { class: 'input ddb-select', 'aria-label': label, onchange: (ev) => onchange(ev.target.value) }, options.map(([v, t]) => h('option', { value: v, selected: v === value ? true : null, text: t }))));
    page.append(h('div', { class: 'toolbar' }, searchBox(D.q, 'Search a demon, race, skill, feature, tarot or zone', (v) => { D.q = v; D.only = null; drawChips(); draw(); }),
      sel('Race', D.race, [['', 'All races']].concat(races.map((r) => [r, r])), (v) => { D.race = v; D.only = null; drawChips(); draw(); }),
      sel('Alignment', D.align, [['', 'All'], ['Law', 'Law'], ['Neutral', 'Neutral'], ['Chaos', 'Chaos']], (v) => { D.align = v; D.only = null; drawChips(); draw(); }),
      sel('Sort', D.sort, [['level', 'Level'], ['name', 'Name'], ['race', 'Race']], (v) => { D.sort = v; draw(); }),
      h('label', { class: 'gdb-check', 'data-tip': 'Variants are versions of a demon with their own name, level, stats and skills (Lucky Pixie, Inexperienced Cerberus, Open-minded Ishtar ...)' },
        h('input', { type: 'checkbox', checked: D.variants ? true : null, onchange: (ev) => { D.variants = ev.target.checked; D.only = null; store.set('ddb-variants', D.variants ? '1' : '0'); drawChips(); draw(); } }),
        'Show variants')));
    page.append(chips, onlyBar);
    page.append(h('div', { class: 'tbl-wrap' }, h('table', { class: 'tbl' },
      h('thead', null, h('tr', null, h('th', { scope: 'col', text: 'Demon' }), h('th', { scope: 'col', class: 'num', text: 'Lv' }), h('th', { scope: 'col', text: 'Alignment' }),
        h('th', { scope: 'col', text: 'Where to contract' }), h('th', { scope: 'col', text: 'Tarot · Soul' }))), body)));
    page.append(count);
    drawChips(); draw();
    return page;
  }
  // ---------- Tools: Demon Rank ----------
  // The Demon Database's demons side by side, sorted by any of their 12 stats or Critical. Stats come from ddbStats
  // (the server's growth and dependent-stat rules); Critical is the demon's own value from the client (CorrectTbl
  // CRITICAL), which does not grow with level. P.Def and M.Def are left out on purpose.
  const RANK_KINDS = [['contract', 'Can be contracted'], ['own', 'Any way to get it'], ['all', 'All demons']];
  /** Whether the player can get the demon some way: a contract, a normal fusion, a DCM, or another way the data lists
   *  (an exchange item, a special fusion, an event or NPC gift), the same rule as the Stat Ranking's
   *  "Any way to get it". The Rebirth Planner's demon list uses it; the DCM part needs dcm-sources.json loaded (loadDcm). */
  function ddbCanOwn(x) {
    return x.contract.length > 0 || !!x.fusion || !!(S.dcm.data && S.dcm.data.items.some((it) => it.devilId === x.id)) || !!(x.get && x.get.length);
  }
  /** The demon's face, with the game's Mitama mark on its top left corner (as Overview's summoned demon) when the Mitama form is shown. */
  const rankFace = (x, mt) => { const pic = ddbFace(x); if (mt && S.data) { pic.classList.add('rank-face'); mitamaMark(pic, true); } return pic; };
  function pageRank() {
    const D = S.rank, page = h('div', { class: 'page page-dcm page-rank' });
    loadDemonDb();
    page.append(pageHead('Every demon you can own, ranked by a stat. Click a column to sort by it, click a demon to open it in the Demon Database.'));
    if (S.ddb.state !== 'ready') {
      page.append(h('p', { class: 'note', text: S.ddb.state === 'failed' ? 'The demon list could not be loaded (demon-db.json is missing beside the page).' : 'Loading the demons…' }));
      return page;
    }
    const d = S.ddb.data, all = d.demons;
    const hasDcm = (x) => !!(S.dcm.data && S.dcm.data.items.some((it) => it.devilId === x.id));
    const inKind = (x, k) => (k === 'all' ? true : k === 'contract' ? x.contract.length > 0 : x.contract.length > 0 || x.fusion || hasDcm(x) || !!(x.get && x.get.length));
    const races = [...new Set(all.map((x) => x.race))].sort();
    const names = d.statNames.slice(0, 12).concat('Crit');
    const tips = { 'Max HP': 'Max HP', 'Max MP': 'Max MP', 'CLS-R': 'Close-range attack', 'LNG-R': 'Long-range attack', Spell: 'Spell power', Support: 'Support power',
      Crit: 'Critical: the demon\'s own value. It does not grow with level; a critical roll adds Luck to it.' };
    const body = h('tbody'), head = h('thead'), count = h('p', { class: 'note' }), chips = h('div', { class: 'chips', role: 'group', 'aria-label': 'Which demons' });
    /** One demon as a row of numbers: the form shown (Mitama-fused when asked and it has one), the level used, its 13 values. */
    const rowOf = (x) => {
      const v = D.mt && x.mt ? Object.assign({}, x, x.mt) : x;
      // Level 1 is below every demon's base level: the server's formula at level 1, to compare them on equal footing
      const L = D.at === 'base' ? v.level : D.at === '1' ? 1 : 99;
      const s = ddbStats(v, L);
      return { x, mt: v !== x, L, vals: s.slice(0, 12).concat(v.crit || 0) };
    };
    const drawChips = () => chips.replaceChildren(...RANK_KINDS.map(([id, label]) =>
      h('button', { type: 'button', class: 'chip-btn', 'aria-pressed': String(D.kind === id), onclick: () => { D.kind = id; drawChips(); draw(); } },
        label, h('span', { class: 'count', text: all.filter((x) => inKind(x, id) && (!D.race || x.race === D.race)).length }))));
    const draw = () => {
      const words = D.q.trim().toLowerCase().split(/\s+/).filter(Boolean);
      const rows = all.filter((x) => inKind(x, D.kind) && (!D.race || x.race === D.race) && words.every((w) => (x.name + ' ' + x.race).toLowerCase().includes(w))).map(rowOf);
      rows.sort((a, b) => D.dir * (b.vals[D.sort] - a.vals[D.sort]) || a.x.name.localeCompare(b.x.name));
      head.replaceChildren(h('tr', null, h('th', { scope: 'col', class: 'num', text: '#' }), h('th', { scope: 'col', text: 'Demon' }), h('th', { scope: 'col', class: 'num', text: 'Lv' }),
        names.map((n, i) => { const on = D.sort === i;
          return h('th', { scope: 'col', class: 'num' + (on ? ' rank-on' : ''), 'aria-sort': on ? (D.dir === 1 ? 'descending' : 'ascending') : null },
            h('button', { type: 'button', class: 'th-btn', 'data-tip': `${tips[n] || n}. Click to sort${on ? ' the other way' : ''}.`,
              onclick: () => { if (on) D.dir = -D.dir; else { D.sort = i; D.dir = 1; } draw(); } },
              h('span', { text: n }), on ? h('span', { class: 'sort-mark', text: D.dir === 1 ? '▼' : '▲' }) : null)); })));
      // equal values share a rank (1, 2, 2, 4)
      let rank = 0;
      body.replaceChildren(...rows.map((r, i) => {
        if (i === 0 || r.vals[D.sort] !== rows[i - 1].vals[D.sort]) rank = i + 1;
        const x = r.x;
        return h('tr', { class: 'row-click', 'data-tip': 'Open in the Demon Database', onclick: () => { S.ddb.only = x.id; S.ddb.q = ''; S.ddb.open = new Set([x.id]); if (r.mt) S.ddb.mtView.add(x.id); go('ddb'); } },
          h('td', { class: 'num rank-no', text: rank }),
          h('td', null, h('div', { class: 'rank-demon' }, rankFace(x, r.mt), h('span', null, x.name, h('span', { class: 'src-count', text: x.race })))),
          h('td', { class: 'num', text: r.L }),
          r.vals.map((v, j) => h('td', { class: 'num' + (j === D.sort ? ' emph rank-on' : ''), text: fmt(v) })));
      }));
      if (!rows.length) body.append(h('tr', null, h('td', { colspan: 16, class: 'empty-row', text: words.length ? `Nothing matches “${D.q.trim()}”.` : 'No demon in this group.' })));
      count.textContent = `Showing ${rows.length} of ${all.length} demons. ` + (D.at === 'base' ? 'Each at its base level, the level it has when contracted or fused.'
        : D.at === '1' ? 'All at level 1, below where any demon starts: the server\'s formula at level 1, to compare them on equal footing.' : 'All at level 99.')
        + ' No Demon Force, rebirth or Mitama runes: the stats of a fresh demon of that level.';
    };
    const atChips = h('div', { class: 'chips', role: 'group', 'aria-label': 'Compare at' });
    const drawAt = () => { atChips.replaceChildren(...[['base', 'Base level'], ['1', 'Level 1'], ['99', 'Level 99']].map(([k, label]) =>
      h('button', { type: 'button', class: 'chip-btn', 'aria-pressed': String(D.at === k), onclick: () => { D.at = k; drawAt(); draw(); } }, label))); };
    const mtBox = h('label', { class: 'gdb-check', 'data-tip': 'Use the Mitama-fused form\'s stats for the demons that have one' },
      h('input', { type: 'checkbox', checked: D.mt ? true : null, onchange: (ev) => { D.mt = ev.target.checked; draw(); } }), 'Mitama-fused');
    page.append(h('div', { class: 'toolbar' }, searchBox(D.q, 'Search a demon or race', (v) => { D.q = v; draw(); }),
      h('label', { class: 'gdb-check' }, 'Race', h('select', { class: 'input ddb-select', 'aria-label': 'Race', onchange: (ev) => { D.race = ev.target.value; drawChips(); draw(); } },
        [['', 'All races']].concat(races.map((r) => [r, r])).map(([v, t]) => h('option', { value: v, selected: v === D.race ? true : null, text: t })))), mtBox));
    page.append(h('div', { class: 'toolbar rank-at' }, h('span', { class: 'q-key', text: 'Compare at' }), atChips), chips);
    page.append(h('div', { class: 'tbl-wrap' }, h('table', { class: 'tbl rank-tbl' }, head, body)), count);
    drawChips(); drawAt(); draw();
    return page;
  }
  // ---------- Tools: Skill Database ----------
  // skill-db.json is made by "imagine-reader skilldb" (src\Build\SkillDb.cs): every skill of the IMAGINE client grouped
  // by its action type (the game's Help names them), with element, cost, timings, power and the expertise ranks that
  // unlock it. Which demons learn it or have it as a feature comes from demon-db.json (the Demon Database), read here.
  // Icons are in skill-db-icons\ beside this file.
  const SKL_WHO = [['get', 'Skills you can get'], ['demon', 'Learned by demons'], ['feature', 'Demon features'], ['exp', 'From expertise'], ['barter', 'From barter'], ['fusion', 'Fusion skills'], ['all', 'Every skill in the game data']];
  /** Text as searchable words: lower case, each word after one space (punctuation is a space). */
  const sklWords = (s) => ' ' + String(s || '').toLowerCase().replace(/[^\p{L}\p{N}]+/gu, ' ').trim() + ' ';
  const SKL_NO_ICON = { none: 'The game has no picture for this skill (fusion skills and most demon features have none).',
    missing: 'The game names a picture for this skill, but the client does not include it.' };
  /** A skill's picture; an empty box says why there is none. */
  /** A skill's name. `ja` (skill-db.json, demon-db.json): the game names it only in Japanese and the name is BABEL's
   *  translation (referenceskill-names-en.tsv), so it is greyed out with the game's own name in its tooltip. */
  /** A name BABEL translated (or repaired) because the game has it only in Japanese or garbled: greyed out, with the game's
   *  own name in its tooltip. `ja` is that name (skills, demons, growth types: skill-names-en.tsv, demon-names-en.tsv,
   *  growth-names-en.tsv); without it the name is the game's and shows as it is. */
  const TL_TIP = (ja, what) => `BABEL's translation: the game has no English name for this ${what}. In game it is called ${ja}`;
  const tlName = (name, ja, what, tag = 'span') => (ja ? h(tag, { class: 'tl-name', 'data-tip': TL_TIP(ja, what), text: name }) : h(tag, { text: name }));
  /** A description BABEL translated because the game's is Japanese (`descJa`: skill-db, demon-db, gear-db, dungeons):
   *  greyed out, the game's own text in the tooltip. */
  const TL_DESC_TIP = (ja) => `BABEL's translation: the game has this text only in Japanese. In game it reads:\n\n${Array.isArray(ja) ? ja.join('\n') : ja}`;
  const TL_DESC_NOTE = 'BABEL\'s translation of the game\'s Japanese text';
  const SKL_TL_TIP = (s) => TL_TIP(s.ja, 'skill');
  const sklName = (s, tag = 'span') => tlName(s ? s.name : '', s && s.ja, 'skill', tag);
  const sklIcon = (x) => { const box = ddbImg(x.icon, 24, 'skill-ico'); if (!x.icon && SKL_NO_ICON[x.noIcon]) box.setAttribute('data-tip', SKL_NO_ICON[x.noIcon]); return box; };
  function loadSkillDb() {
    loadDemonDb();   // who learns each skill
    loadSkillData();
  }
  /** skill-db.json alone (the Expertise page's icon tooltips take the game's descriptions from it). */
  function loadSkillData() {
    if (S.skl.state !== 'idle') return;
    S.skl.state = 'loading';
    fetch('skill-db.json', { cache: 'no-store' }).then((r) => (r.ok ? r.json() : null)).catch(() => null).then((d) => {
      S.skl.data = d; S.skl.state = d ? 'ready' : 'failed';
      if (d) d.skills.forEach((x) => { x.hay0 = sklWords([x.name, x.ja, x.el, x.desc, ...(x.exp || []), x.fusion ? 'fusion' : '', x.barter ? 'barter' : '', ...(x.barter || []).map((b) => b.cost)].join(' ')); });
      // tooltips made before the descriptions came in are made again on the next hover
      document.querySelectorAll('[data-rich]').forEach((el) => { delete el._richNode; });
      if (S.page === 'rebirth') render();   // the planner's Skills step waits for the list
      if (S.page === 'skills' || (d && (S.page === 'expertise' || S.page === 'sim'))) render();   // the planners' unlock lists get their pictures
    });
  }
  /** A skill of the game by id (skill-db.json), or null while that is loading or when it isn't there. */
  const sklById = (id) => {
    const D = S.skl.data;
    if (!D) return null;
    if (!S.skl.byId || S.skl.byIdFrom !== D) { S.skl.byId = new Map(D.skills.map((x) => [x.id, x])); S.skl.byIdFrom = D; }
    return S.skl.byId.get(id) || null;
  };
  /** A skill an expertise rank unlocks, found in skill-db.json by its expertise, rank and name ("Shot · Class 0 Rank 5"). */
  const sklByExp = (exp, rank, name) => {
    const D = S.skl.data;
    if (!D) return null;
    if (!S.skl.byExp || S.skl.byExpFrom !== D) {
      S.skl.byExp = new Map();
      D.skills.forEach((x) => (x.exp || []).forEach((e) => S.skl.byExp.set(`${e}|${x.name}`, x)));
      S.skl.byExpFrom = D;
    }
    return S.skl.byExp.get(`${exp} · Class ${Math.floor(rank / 10)} Rank ${rank % 10}|${name}`) || null;
  };
  /** A rank's skills in a planner's unlock list; a rank that lets you learn only one of them puts "or" between them. */
  const unlockSkills = (u, exp) => {
    const tip = `The game lets you learn only ${u.pick === 1 ? 'one' : u.pick} of these. Which one counts is the one you learned in game.`;
    const list = u.skills.map((s) => unlockSkill(s, exp, u.rank));
    if (!u.pick || u.skills.length < 2) return list;
    if (u.pick > 1) return [h('span', { class: 'sim-unlock-pick', 'data-tip': tip, text: `Learn ${u.pick} of: ` }), ...list];
    return list.flatMap((el, i) => (i ? [h('span', { class: 'sim-unlock-pick unl-or', 'data-tip': tip, text: 'or' }), el] : [el]));
  };
  /** One skill in a planner's unlock list: its picture and name, with the game-style tooltip (expSkillTip). */
  function unlockSkill(s, exp, rank) {
    const x = sklByExp(exp, rank, s.name);
    const ic = x ? sklIcon(x) : h('span', { class: 'skill-ico', 'aria-hidden': 'true' });
    const el = h('span', { class: 'unl-skill' + (s.passive ? ' unl-passive' : '') }, ic, h('span', { text: s.name }));
    return expSkillTip(el, { id: x ? x.id : null, name: s.name, active: !s.passive, expertise: exp, rank: `Class ${Math.floor(rank / 10)} Rank ${rank % 10}`,
      effects: x ? '' : (s.passive ? '' : 'An active skill: it adds no stats by itself') });
  }
  /** skill id -> the compendium demons that learn it or have it as a feature (from the Demon Database's data), made once. */
  function sklDemons() {
    const D = S.skl, dd = S.ddb.data;
    if (!dd) return null;
    if (D.byDemon && D.byDemonFrom === dd) return D.byDemon;
    const m = new Map(), add = (id, row) => { if (!m.has(id)) m.set(id, []); const l = m.get(id); if (!l.some((r) => r.d === row.d && r.how === row.how && r.mt === row.mt)) l.push(row); };
    dd.demons.forEach((x) => {
      x.traits.forEach((t) => add(t, { d: x, how: 'feature', lv: 0 }));
      x.skills.forEach(([id, at]) => add(id, { d: x, how: 'skill', lv: at }));
      if (x.mt) {
        x.mt.traits.filter((t) => !x.traits.includes(t)).forEach((t) => add(t, { d: x, how: 'feature', lv: 0, mt: true }));
        (x.mt.skills || []).filter(([id]) => !x.skills.some(([s]) => s === id)).forEach(([id, at]) => add(id, { d: x, how: 'skill', lv: at, mt: true }));
      }
    });
    m.forEach((l) => l.sort((a, b) => (a.how === b.how ? 0 : a.how === 'skill' ? -1 : 1) || a.lv - b.lv || a.d.level - b.d.level || a.d.name.localeCompare(b.d.name)));
    D.data.skills.forEach((x) => { x.hay = x.hay0 + sklWords((m.get(x.id) || []).map((r) => r.d.name).join(' ')); });
    D.byDemon = m; D.byDemonFrom = dd;
    return m;
  }
  /** "Spell, Long-range" -> "Spell 100% + Long-range 50%". The game lists a skill's stats in order (skill-db.json `stat`): with
   *  two, the first counts in full and the second at half (server: LNGR_SPELL = lngr + spell/2); the three-stat types add
   *  all three in full. One stat (or Weapon, Support) has nothing to split. The game has no four-stat type. */
  function sklStatRates(stat) {
    const p = (stat || '').split(', ').filter(Boolean);
    if (p.length < 2) return stat;
    // each "Stat 100%" is one unbreakable piece, so a narrow box wraps only at the "+" and never inside "Long-range"
    return p.flatMap((s, i) => [i ? ' + ' : '', h('span', { style: 'white-space:nowrap', text: `${s} ${p.length === 2 && i === 1 ? 50 : 100}%` })]).filter(Boolean);
  }
  function sklDetail(x, who) {
    const box = h('div', { class: 'q-detail skl-detail' });
    const block = (label, ...kids) => h('div', { class: 'gdb-block' }, h('span', { class: 'q-key', text: label }), h('div', null, ...kids));
    if (x.ja) box.append(block('Name', h('p', { class: 'ddb-desc skl-desc' }, h('span', { class: 'tl-name', text: x.name }),
      ` is BABEL's translation. The game has no English name for this skill: it calls it ${x.ja}.`)));
    if (x.desc) box.append(block(x.descJa ? 'In game (translated)' : 'In game', h('p', { class: 'ddb-desc skl-desc' + (x.descJa ? ' tl-name' : ''), 'data-tip': x.descJa ? TL_DESC_TIP(x.descJa) : null, text: x.desc })));
    const facts = [['Target', x.target], ['Reach', x.reach != null ? String(x.reach) : null, 'In the game\'s own distance units: compare skills with each other.'],
      ['Damage from', sklStatRates(x.stat), 'The stat its damage or effect is worked out from. With two stats the first counts in full and the second at half; with three, each counts in full and they are added.'], ['Needs', x.weapon],
      ['Heals', x.heal != null ? String(x.heal) : null, 'The healing modifier (a % is of the target\'s Max HP).'],
      ['MP while held', x.drain ? `${x.drain} MP` : null, 'Drained while the stance is held.']].filter((f) => f[1]);
    if (facts.length) box.append(block('Details', h('div', { class: 'skl-facts' }, facts.map(([k, v, tip]) =>
      h('div', { class: 'ddb-stat', 'data-tip': tip || null }, h('span', { class: 'q-key', text: k }), h('b', Array.isArray(v) ? null : { text: v }, ...(Array.isArray(v) ? v : [])))))));
    const get = h('ul', { class: 'gdb-list gdb-get' });
    (x.exp || []).forEach((e) => get.append(h('li', null, h('span', { class: 'gdb-kind', text: 'Expertise' }), h('b', { text: e }))));
    // `at`: who opens the exchange list, and where, traced in the server data (skill-db.json, npc_places); absent when no event there
    // opens it. `lv` / `needs`: what the exchange asks of the demon (the barter condition table)
    (x.barter || []).forEach((b) => get.append(h('li', null, h('span', { class: 'gdb-kind', text: 'Barter' }), h('b', { text: b.cost }),
      h('span', { class: 'src-count', text: (b.at ? ` · ${b.at}` : '') + (b.for === 'demon' ? ' · your summoned demon learns it (Skill: Demon)' : ' · you learn it')
        + (b.lv || b.needs ? ` · needs ${[b.lv ? `${b.for === 'demon' ? 'the demon' : 'you'} at Lv ${b.lv}` : '', b.needs || ''].filter(Boolean).join('; ')}` : '') }))));
    if (x.fusion) get.append(h('li', null, h('span', { class: 'gdb-kind', text: 'Fusion' }), h('span', null, 'A fusion skill: the game data marks it as one, it does not say which pair makes it.')));
    if (get.childNodes.length) box.append(block('How to get it', get));
    if (who && who.length) {
      const open = (d) => { S.ddb.only = d.id; S.ddb.q = ''; S.ddb.open = new Set([d.id]); go('ddb'); };
      const tag = (r) => (r.how === 'feature' ? 'feature' : r.lv ? `learns at Lv ${r.lv}` : 'knows it') + (r.mt ? ' · Mitama-fused' : '');
      box.append(block(`Demons (${new Set(who.map((r) => r.d.id)).size})`, h('ul', { class: 'gdb-list skl-demons' }, who.map((r) =>
        h('li', null, h('button', { type: 'button', class: 'skl-demon', 'data-tip': 'Open in the Demon Database', onclick: () => open(r.d) },
          ddbFace(r.d), h('span', null, h('b', { text: r.d.name }), h('span', { class: 'src-count', text: `${r.d.race} · Lv ${r.d.level} · ${tag(r)}` }))))))));
    } else if (!x.exp && !x.fusion && !x.barter) box.append(block('How to get it', h('span', { class: 'zero', text: S.ddb.data ? 'No compendium demon, expertise rank or NPC exchange has it: enemies, events or unused.' : 'Loading the demons…' })));
    return box;
  }
  function pageSkills() {
    const D = S.skl, page = h('div', { class: 'page page-dcm' });
    loadSkillDb();
    page.append(pageHead('Every skill in the game, separated by action type. Click a skill for what it does and who can learn it.'));
    if (D.state !== 'ready') {
      page.append(h('p', { class: 'note', text: D.state === 'failed' ? 'The skill database could not be loaded (skill-db.json is missing beside the page).' : 'Loading the skills…' }));
      return page;
    }
    const d = D.data, all = d.skills, byDemon = sklDemons() || new Map();
    const types = d.types, typeOf = Object.fromEntries(types.map((t) => [t.key, t]));
    const whoOf = (x) => byDemon.get(x.id) || [];
    const inWho = (x, k) => {
      const w = whoOf(x);
      if (k === 'all') return true;
      if (k === 'demon') return w.some((r) => r.how === 'skill');
      if (k === 'feature') return w.some((r) => r.how === 'feature');
      if (k === 'exp') return !!x.exp;
      if (k === 'fusion') return !!x.fusion;
      if (k === 'barter') return !!x.barter;
      return w.length > 0 || !!x.exp || !!x.fusion || !!x.barter;
    };
    const words = () => sklWords(D.q).trim().split(' ').filter(Boolean);   // each must start a word: "agi" finds Agi, not Magic
    const match = (x) => inWho(x, D.who) && (!D.el || x.el === D.el) && words().every((w) => (x.hay || x.hay0).includes(' ' + w));
    const chips = h('div', { class: 'chips', role: 'group', 'aria-label': 'Action type' });
    const helpBox = h('div'), body = h('tbody'), more = h('nav', { class: 'pager', 'aria-label': 'Pages' }), count = h('p', { class: 'note' });
    const drawChips = () => {
      const pool = all.filter(match);
      chips.replaceChildren(...[['', 'All types']].concat(types.map((t) => [t.key, t.name])).map(([k, label]) =>
        h('button', { type: 'button', class: 'chip-btn', 'aria-pressed': String(D.type === k), onclick: () => { D.type = k; D.pg = 0; drawChips(); draw(); } },
          label, h('span', { class: 'count', text: k ? pool.filter((x) => x.type === k).length : pool.length }))));
    };
    const drawHelp = () => {
      const t = typeOf[D.type];
      const p = (s) => h('p', { class: 'skl-help', text: s.replace(/\n\s*\n+/g, '\n') });
      helpBox.replaceChildren(h('div', { class: 'notice skl-notice' }, icon('info'), h('div', null,
        t ? [h('b', { text: `${t.name} · from the game's Help` }), t.help ? p(t.help) : p('The game\'s Help says nothing more about this type.'),
          t.key === 'shot' && t.also ? [h('b', { text: 'Destruction magic is a Shot skill too' }), p(t.also)] : null,
          t.key === 'support' ? [h('b', { text: 'Support and curse spells are Curative skills too' }), t.also ? p(t.also) : null, t.also2 ? p(t.also2) : null] : null]
          : [h('b', { text: 'Action types · from the game\'s Help' }), p(d.intro + ' Passive and switch skills are not used in a fight, so they have no action type of their own: they are listed after the others.')])));
    };
    const SORTS = { name: (a, b) => a.name.localeCompare(b.name) || a.id - b.id, power: (a, b) => (b.power || 0) - (a.power || 0) || a.name.localeCompare(b.name),
      stiff: (a, b) => (b.stiff || 0) - (a.stiff || 0) || a.name.localeCompare(b.name),
      cd: (a, b) => parseFloat(a.cd || 0) - parseFloat(b.cd || 0) || a.name.localeCompare(b.name) };
    const whoSummary = (x) => {
      const w = whoOf(x), n = new Set(w.filter((r) => r.how === 'skill').map((r) => r.d.id)).size, f = new Set(w.filter((r) => r.how === 'feature').map((r) => r.d.id)).size;
      const parts = [];
      if (x.exp) parts.push(x.exp.length === 1 ? x.exp[0] : `${x.exp.length} expertise ranks`);
      if (n) parts.push(n === 1 ? w.find((r) => r.how === 'skill').d.name : `${n} demons`);
      if (f) parts.push(f === 1 ? `Feature of ${w.find((r) => r.how === 'feature').d.name}` : `Feature of ${f} demons`);
      if (x.barter) parts.push('Barter');
      if (x.fusion) parts.push('Fusion skill');
      return parts.length ? h('span', { class: 'skl-who', text: parts.join(' · ') }) : h('span', { class: 'zero', text: '—' });
    };
    const COLS = 7;
    const row = (x) => {
      const open = D.open.has(x.id);
      const toggle = () => { if (D.open.has(x.id)) D.open.delete(x.id); else D.open.add(x.id); draw(); };
      const cell = (v, cls) => h('td', { class: (cls || 'num') + (v ? '' : ' zero'), text: v || '—' });
      body.append(h('tr', { class: 'row-click' + (open ? ' open' : ''), onclick: (ev) => { if (!ev.target.closest('button')) toggle(); } },
        h('td', null, h('button', { type: 'button', class: 'row-btn', 'aria-expanded': String(open), onclick: toggle },
          h('span', { class: 'chev' }, icon('chevron')), sklIcon(x), h('span', null, sklName(x), x.el ? h('span', { class: 'src-count', text: x.el }) : null, x.ja ? h('span', { class: 'key-jp', lang: 'ja', text: x.ja }) : null))),
        cell(x.cost, 'dim'), cell(x.cast), cell(x.cd), cell(x.power != null ? String(x.power) : x.heal != null ? `heals ${x.heal}` : ''), cell(x.stiff != null ? fmt(x.stiff) : ''),
        h('td', null, whoSummary(x))));
      if (open) body.append(h('tr', { class: 'detail-row' }, h('td', { colspan: COLS }, sklDetail(x, whoOf(x)))));
    };
    const draw = () => {
      body.replaceChildren(); more.replaceChildren();
      const rows = all.filter((x) => (!D.type || x.type === D.type) && match(x));
      const order = types.map((t) => t.key);
      rows.sort((a, b) => (D.type ? 0 : order.indexOf(a.type) - order.indexOf(b.type)) || SORTS[D.sort](a, b));
      // this page's rows; under "All" a group row names each action type where it starts (also at the top of a page)
      const pg = pageOf(D, rows);
      let last = null;
      for (const x of pg.rows) {
        if (!D.type && x.type !== last) {
          last = x.type;
          const n = rows.filter((r) => r.type === x.type).length;
          body.append(h('tr', { class: 'dun-group-row', onclick: () => { D.type = x.type; D.pg = 0; drawChips(); draw(); window.scrollTo(0, 0); } },
            h('td', { colspan: COLS }, typeOf[x.type].name, h('span', { class: 'src-count', text: `${n} skill${n === 1 ? '' : 's'} · show only these` }))));
        }
        row(x);
      }
      if (!rows.length) body.append(h('tr', null, h('td', { colspan: COLS, class: 'empty-row', text: D.q.trim() ? `Nothing matches “${D.q.trim()}”.` : 'No skill in this group.' })));
      drawPager(more, D, rows.length, pg, draw, () => body.closest('.tbl-wrap') || body.closest('table'), 'skills');
      drawHelp();
      const nGet = all.filter((x) => inWho(x, 'get')).length;
      count.textContent = `${fmt(rows.length)} skills listed (${all.length} in the game data, ${nGet} of them can be learned by a demon, come from expertise or an NPC exchange (barter), or are fusion skills). `
        + (S.ddb.state === 'ready' ? '' : 'Still loading the demons: who learns each skill shows once they are in. ')
        + `${fmt(all.filter((x) => x.ja).length)} the game names only in Japanese are shown greyed out, under BABEL's English translation. `
        + `${d.skipped.untranslated ? `${d.skipped.untranslated} skills with only a Japanese name and no translation yet, ` : ''}Item uses and debug skills are left out. Data made on ${d.generatedAt}.`;
    };
    const sel = (label, value, options, onchange) => h('label', { class: 'gdb-check' }, label,
      h('select', { class: 'input ddb-select', 'aria-label': label, onchange: (ev) => onchange(ev.target.value) }, options.map(([v, t]) => h('option', { value: v, selected: v === value ? true : null, text: t }))));
    const els = [...new Set(all.map((x) => x.el).filter(Boolean))].sort();
    page.append(h('div', { class: 'toolbar' }, searchBox(D.q, 'Search a skill, element, effect, expertise or demon', (v) => { D.q = v; D.pg = 0; drawChips(); draw(); }),
      sel('Who can get it', D.who, SKL_WHO, (v) => { D.who = v; D.pg = 0; drawChips(); draw(); }),
      sel('Element', D.el, [['', 'All']].concat(els.map((e) => [e, e])), (v) => { D.el = v; D.pg = 0; drawChips(); draw(); }),
      sel('Sort', D.sort, [['name', 'Name'], ['power', 'Modifier'], ['stiff', 'Stiffness'], ['cd', 'Cooldown']], (v) => { D.sort = v; draw(); })));
    page.append(chips, helpBox);
    page.append(h('div', { class: 'tbl-wrap' }, h('table', { class: 'tbl skl-tbl' },
      h('thead', null, h('tr', null, h('th', { scope: 'col', text: 'Skill' }), h('th', { scope: 'col', text: 'Cost' }), h('th', { scope: 'col', class: 'num', text: 'Cast' }),
        h('th', { scope: 'col', class: 'num', text: 'Cooldown' }), h('th', { scope: 'col', class: 'num', 'data-tip': 'The skill\'s damage modifier, as its description in game gives it. A heal shows its healing modifier.', text: 'Modifier' }),
        h('th', { scope: 'col', class: 'num', 'data-tip': 'How long a hit leaves the target stunned, as the skill\'s description in game gives it.', text: 'Stiffness' }),
        h('th', { scope: 'col', text: 'Who can get it' }))), body)));
    page.append(more, count);
    drawChips(); draw();
    return page;
  }
  // ---------- Build Planner (Tools): other gear and expertise ranks, and what they would give ----------
  // The numbers come from the reader (host "simInit" / "simulate" / "simCatalog": src\Build\Simulator.cs), so they are
  // worked out by the same code as the rest of the app. The page only keeps the build being tried, asks for its result
  // after every change and shows it against the character as last read. Nothing is saved but the build being tried
  // (in the page's own storage, so it is still there next time).
  const SIM_SUB = 'Try other gear and expertise ranks and watch your stats change. Nothing here touches the game or your saved data.';
  // A worn piece's parts, numbered as the game's fusion window numbers them.
  const SIM_PARTS = {
    whole: ['Replace the whole piece', 'Another piece, with its own basic features and characteristics. Tarot, soul fusion, fusion bonus and mods stay as they are.'],
    id: ['Slot 1 · Set bonus and appearance', 'The piece this one counts as: it decides the appearance bonus and the sets.'],
    basic: ['Slot 2 · Basic features', 'The piece whose basic features (its stats) this one carries.'],
    special: ['Slot 3 · Characteristics', 'The piece whose characteristics (its special effects) this one carries.'],
    tarot: ['Tarot fusion', 'The crystal fused in as a tarot.'],
    soul: ['Soul fusion', 'The crystal fused in as a soul.'],
  };
  // the five mod slots of a piece, parts "mod0" ... "mod4" (spec.mods[i]; 0 = none, 255 = an open slot with nothing in it)
  for (let i = 0; i < 5; i++) SIM_PARTS['mod' + i] = [`Mod slot ${i + 1}`, 'The mod in this slot. Only what the game lets you end up with in this slot is listed: a new kind of mod starts at level 1, and each level needs the one below it.'];
  const simModIdx = (part) => { const m = /^mod(\d)$/.exec(part); return m ? Number(m[1]) : -1; };
  /** What one part of a piece holds now (an item, a crystal, a mod effect). */
  const simPartVal = (s, part) => { const i = simModIdx(part); return !s ? undefined : i >= 0 ? ((s.mods || [])[i] || 0) : s[part]; };
  const simClone = (v) => JSON.parse(JSON.stringify(v));
  /** Puts these (and only these) inside el: nothing for null or false, lists are unpacked. */
  const simFill = (el, ...kids) => { el.replaceChildren(...kids.flat(3).filter((x) => x !== null && x !== undefined && x !== false)); return el; };
  const simSame = (a, b) => JSON.stringify(a) === JSON.stringify(b);
  const simKey = () => 'sim-draft';   // the unsaved build (not one per character: builds are independent of the character shown)
  const simPt = (v) => (n0(v) / 100).toLocaleString('en-US', { minimumFractionDigits: 2, maximumFractionDigits: 2 }) + 'pt';   // as the game's Expertise window writes points
  const simRank = (r) => `Class ${Math.floor(r / 10)} · Rank ${r % 10}`;
  const simPlain = (name) => String(name || '').replace(/[♂♀]/g, '').trim();

  // --- the base of a build: a saved character as last read, or none ("No base": nothing worn, no expertise, and the
  //     numbers are only what the build gives; a level and an alignment are then the build's own). The base only
  //     serves as the start and as what the build is compared with: nothing of that character is changed. ---
  const simActive = () => S.meta.character || '';
  const simLevel0 = () => n0(S.v && S.v.latest && S.v.latest.level) || 99;
  const simArgs = (who, level, align) => (who ? { base: who } : { base: '', level: level || simLevel0(), alignment: align || 'NEUTRAL' });
  const simNow = () => simArgs(S.sim.char, S.sim.level, S.sim.align);
  const simBaseKey = (who, level, align) => (who ? 'c:' + who : `none:${level || simLevel0()}:${align || 'NEUTRAL'}`);
  /** The base a saved build (or the kept unsaved one) names. One saved before bases existed: the character shown. */
  const simBaseOf = (b) => ({ who: b.base === undefined || b.base === null ? simActive() : String(b.base), level: n0(b.level) || simLevel0(), align: b.alignment || b.align || 'NEUTRAL' });
  /** What the reader says about a base (its build as read, the expertise list, its result): asked once per reading. */
  function simBase(who, level, align) {
    const M = S.sim, key = simBaseKey(who, level, align);
    if (!M.inits.has(key)) M.inits.set(key, host.call('simInit', simArgs(who, level, align))
      .then((init) => { const entry = { init, catalog: {} }; M.bases.set(key, entry); return entry; })
      .catch((e) => { M.inits.delete(key); throw e; }));
    return M.inits.get(key);
  }
  /** Makes this the base of the build in the editor. Throws when it can't be read (a character without a snapshot). */
  async function simSetBase(who, level, align) {
    const M = S.sim, entry = await simBase(who, level, align);
    Object.assign(M, { char: who, level: who ? M.level : (level || simLevel0()), align: who ? M.align : (align || 'NEUTRAL'), init: entry.init, base: entry.init.result, catalog: entry.catalog });
  }
  // --- a build's digitalize demon: one of its base's COMP demons as last seen, or none. It is the build's own: picking
  //     one changes nothing for the character. A build that never chose (a new one, an older one) takes the
  //     character's saved demon when the app is showing digitalized numbers, else none. ---
  const simDemonDefault = (init) => (digi() && init && init.meta.demon_key ? init.meta.demon_key : '');
  const simDemonOf = (b, init) => (typeof b.demon === 'string' ? b.demon : simDemonDefault(init));
  const simDemonName = (key) => (simIsPlan(key) ? (simPlanBuild(key) || {}).name || 'a deleted demon build' : String(key || '').replace(/^\d+:/, '').replace(/:\d+$/, ''));   // "type:name:level" (older: "type:name"), or a Demon Planner build's name
  /** The demons of the editor's base (what that character's demon picker lists; asked once per reading). */
  function simDemonList() {
    const M = S.sim, entry = M.bases.get(simBaseKey(M.char, M.level, M.align));
    if (!entry || !M.char) return Promise.resolve([]);
    if (!entry.demonsAsk) entry.demonsAsk = host.call('simDemons', { base: M.char })
      .then((r) => { entry.demons = (r && r.demons) || []; entry.skillIcons = (r && r.skillIcons) || null; return entry.demons; })
      .catch(() => { entry.demons = []; return entry.demons; });
    return entry.demonsAsk;
  }
  // --- or a Demon Planner build ("plan:<its id>"): the demon as that build ends (rbpRecord), worked out again from the
  //     saved build each time, so a change in the Demon Planner shows here too. The reader gets the demon with every
  //     call (plannedDemon); a build deleted since leaves the Character Planner build with no demon. ---
  const SIM_PLAN = 'plan:';
  const simIsPlan = (key) => String(key || '').startsWith(SIM_PLAN);
  const simPlanBuild = (key) => (simIsPlan(key) ? rbpBuilds().find((b) => b.id === key.slice(SIM_PLAN.length)) || null : null);
  /** Whether what the Demon Planner works a build out with is in (it asks for it; false while it loads). */
  function rbpDataReady() {
    loadRebirth();
    const busy = (st) => !st || st === 'idle' || st === 'loading';
    return !busy(S.rbp.state) && !busy(S.ddb.state) && !busy(S.fus.state) && !busy(S.dcm.state);
  }
  const rbpDataWait = () => new Promise((done) => { const t0 = Date.now(); const tick = () => (rbpDataReady() || Date.now() - t0 > 20000 ? done() : setTimeout(tick, 120)); tick(); });
  /** A simulate call's demon: the key, and for a Demon Planner build the demon as it ends. */
  async function simDemonArgs(key) {
    if (!simIsPlan(key)) return { demon: key };
    await rbpDataWait();
    const b = simPlanBuild(key), r = b ? rbpRecord(b) : null;
    return { demon: key, plannedDemon: r ? r.record : undefined };
  }
  /** The Demon Planner's saved builds as the picker lists them: each as the demon it ends as, with what digitalizing the
   *  editor's base with it gives (the reader works that out for that base, as for its COMP demons). */
  async function simPlanList() {
    const M = S.sim, entry = M.bases.get(simBaseKey(M.char, M.level, M.align));
    if (!entry || !M.char) return [];
    return demonPlanList(entry, M.char);
  }
  /** The same for any page: `entry` keeps the answer (plansSig / plansAsk / plans), `base` is whose offers they show
   *  (none: the character the app shows). */
  async function demonPlanList(entry, base) {
    await rbpDataWait();
    const rows = rbpBuilds().map((b) => { const r = rbpRecord(b); return r ? { b, r } : null; }).filter(Boolean);
    const sig = JSON.stringify(rows.map(({ b, r }) => [b.id, b.name, r.record]));
    if (entry.plansSig !== sig || !entry.plansAsk) {
      entry.plansSig = sig;
      const make = (offers) => rows.map(({ b, r }) => {
        const D = r.D, form = r.fused && D.x.mt ? D.x.mt : D.x;
        return { key: SIM_PLAN + b.id, name: b.name, demonName: D.name, level: r.record.level, race: form.race || D.x.race || '',
          mitama: r.record.mitamaType ? RBP_KINDS[r.record.mitamaType - 1] : '', icon: (D.comp && D.comp.icon) || form.icon || D.x.icon || null,
          offer: offers.get(SIM_PLAN + b.id) || null, planned: r.record.planned.some(Boolean),
          type: r.record.type, forceStackIds: r.record.forceStack, equipmentIds: r.record.equipment, reunion: r.record.reunion, preRanks: r.preRanks };
      });
      entry.plansAsk = (rows.length ? host.call('simDemons', { ...(base ? { base } : {}), planned: rows.map(({ b, r }) => ({ key: SIM_PLAN + b.id, demon: r.record })) }) : Promise.resolve({ planned: [] }))
        .then((res) => { entry.plans = make(new Map(((res && res.planned) || []).map((p) => [p.key, p.offer]))); return entry.plans; })
        .catch(() => { entry.plans = make(new Map()); return entry.plans; });
    }
    return entry.plansAsk;
  }
  /** What differs from the base: the slots and the expertises. Of the build in the editor, or of the one given against its own base. */
  function simChanges(build, init) {
    const M = S.sim, B = build || M.build, I = init || M.init, slots = [], ranks = [];
    if (!I || !B) return { slots, ranks, total: 0 };
    I.build.slots.forEach((s0, k) => { if (!simSame((B.slots || [])[k] || null, s0)) slots.push(k); });
    const p0 = I.build.points, p = B.points || {};
    new Set([...Object.keys(p0), ...Object.keys(p)]).forEach((id) => { if (n0(p0[id]) !== n0(p[id])) ranks.push(id); });
    return { slots, ranks, total: slots.length + ranks.length };
  }
  /** Keeps what is in the editor: the unsaved build in the page's storage (only what differs from the character as
   *  read), a saved build in its file, a moment after the last change. */
  function simSave() {
    const M = S.sim;
    if (M.id) { simStore(); return; }
    const c = simChanges();
    store.set(simKey(), JSON.stringify({ base: M.char, level: M.level, align: M.align, demon: M.demon,
      slots: Object.fromEntries(c.slots.map((k) => [k, M.build.slots[k]])), points: Object.fromEntries(c.ranks.map((id) => [id, n0(M.build.points[id])])),
      cap: M.cap === undefined ? null : M.cap }));   // the expertise point cap set by hand (null: the game's)
  }
  /** The unsaved build as kept: { base, level, align, slots: {slot: piece}, points: {expertise: points}, cap } (slots and points: what differs from its base). */
  function simDraft() {
    try { const v = JSON.parse(store.get(simKey()) || 'null'); return v && typeof v === 'object' ? v : {}; } catch (e) { return {}; }
  }
  const simDraftCount = () => { const d = simDraft(); return Object.keys(d.slots || {}).length + Object.keys(d.points || {}).length; };

  // --- saved builds: sim-builds.json in the character's folder, through the host (simBuilds / simSaveBuild /
  //     simDeleteBuild). A saved build is whole (15 pieces + every expertise's points + the cap), not "what differs". ---
  const simCardKey = (b) => { const at = simBaseOf(b); return JSON.stringify([simBaseKey(at.who, at.level, at.align), typeof b.demon === 'string' ? (simIsPlan(b.demon) ? `${b.demon}@${(simPlanBuild(b.demon) || {}).saved_at || ''}` : b.demon) : 'saved:' + digi(), b.slots, b.points]); };
  /** The build in the editor, as a saved build is written. */
  const simWhole = () => { const M = S.sim; return { ...simNow(), demon: M.demon, slots: M.build.slots, points: M.build.points, cap: M.cap === undefined ? null : M.cap }; };
  const simStatus = () => { const el = S.sim.statusEl; if (el) el.textContent = { saving: 'Saving…', saved: 'Saved', failed: 'Not saved' }[S.sim.saveState] || ''; };
  /** A saved build changed: it goes to its file half a second after the last change. */
  function simStore() {
    const M = S.sim;
    clearTimeout(M.storeTimer);
    M.saveState = 'saving'; simStatus();
    M.storeTimer = setTimeout(simStoreNow, 500);
  }
  async function simStoreNow() {
    const M = S.sim, id = M.id;
    clearTimeout(M.storeTimer); M.storeTimer = null;
    if (!id) return;
    const what = { id, name: M.name, ...simWhole() };
    try {
      const r = await host.call('simSaveBuild', what);
      M.builds = r.builds || [];
      if (M.id === id && !M.storeTimer) M.saveState = 'saved';
    } catch (e) {
      if (M.id === id) M.saveState = 'failed';
      toast('err', 'The build could not be saved', String((e && e.message) || e));
    }
    simStatus(); drawSimNav();
  }
  /** Before leaving a build: a save that was still waiting goes out now. */
  const simFlush = () => (S.sim.storeTimer ? simStoreNow() : Promise.resolve());
  async function simLoadBuilds() {
    const M = S.sim;
    try { const list = await host.call('simBuilds'); if (Array.isArray(list)) M.builds = list; } catch (e) { /* no reader behind the page: no saved builds */ }
    drawSimNav();
    if (S.page === 'sim' && M.view !== 'edit' && M.state === 'ready') render();
  }
  /**
   * Puts a build in the editor, on its base: a saved one by id, or (no id) the unsaved one. Resolves when its result
   * is in; throws when its base can't be read. The unsaved one falls back to the character shown.
   */
  async function simUse(id) {
    const M = S.sim, b = id ? (M.builds || []).find((x) => x.id === id) : null;
    // the unsaved build has the base that was imported into it; none until then (an empty build)
    let kept = b ? null : simDraft();
    const at = b ? simBaseOf(b) : { who: typeof kept.base === 'string' ? kept.base : '', level: n0(kept.level), align: kept.align || '' }, mine = ++M.useSeq;
    try { await simSetBase(at.who, at.level, at.align); }
    catch (e) { if (b || !at.who) throw e; kept = {}; await simSetBase('', 0, ''); }   // an unsaved build whose base can't be read any more: start empty
    if (mine !== M.useSeq) return true;   // another build was asked for meanwhile
    M.id = b ? b.id : null; M.name = b ? b.name : '';
    if (b) {
      M.build = { slots: simClone(b.slots), points: simClone(b.points) };
      M.cap = typeof b.cap === 'number' ? b.cap : null;
      M.demon = simDemonOf(b, M.init);
    } else {
      M.demon = typeof kept.demon === 'string' ? kept.demon : simDemonDefault(M.init);
      // what was being tried last time, put back on top of its base as read now
      M.build = simClone(M.init.build);
      Object.entries(kept.slots || {}).forEach(([k, v]) => { if (Number(k) >= 0 && Number(k) < 15) M.build.slots[k] = v; });
      Object.entries(kept.points || {}).forEach(([pid, v]) => { M.build.points[pid] = n0(v); });
      M.cap = typeof kept.cap === 'number' ? kept.cap : null;
    }
    Object.assign(M, { before: null, open: null, allDiff: false, saveState: b ? 'saved' : '', naming: false, confirmDel: false });
    const hit = b ? M.cards.get(b.id) : null;
    M.cur = hit && hit.res && hit.key === simCardKey(b) ? hit.res : M.base;   // the card's result, until the fresh one is in
    ++M.seq; M.working = false;
    return simRun(false);   // always: even an unchanged build can have another demon than the base's saved one
  }
  const simShow = () => { if (S.page !== 'sim') go('sim'); else { render(); $('#main').scrollTop = 0; } };
  /** Opens a build in the editor (no id: the unsaved one). */
  async function simOpen(id) {
    const M = S.sim;
    if (S.page === 'sim' && !(M.view === 'edit' && M.id === (id || null))) remember();   // (from another page, go() does it)
    if (M.state !== 'ready') { M.view = 'edit'; M.id = id || null; simShow(); return; }   // simLoad puts it in the editor once the character is read
    simFlush();
    try { await simUse(id || null); M.view = 'edit'; }
    catch (e) { toast('err', 'That build can’t be opened', String((e && e.message) || e)); }
    simShow();
  }

  /**
   * "Import base": the build starts again from that character as last read (its gear and expertise ranks; nothing
   * for "No base"), and its numbers are that character's from then on. Nothing of the character is changed. What was
   * in the build can be brought back until the next change ("Undo import").
   */
  async function simRebase(who) {
    const M = S.sim;
    if (who === M.char) return;
    const had = { build: M.build, cap: M.cap, demon: M.demon, char: M.char, level: M.level, align: M.align };
    try { await simSetBase(who, M.level, M.align); }
    catch (e) { toast('err', `${who || 'No base'} can’t be imported`, String((e && e.message) || e)); return; }
    Object.assign(M, { build: simClone(M.init.build), cap: null, demon: simDemonDefault(M.init), open: null, cur: M.base, working: false, allDiff: false });
    simRun();   // (keeps the build, and asks for its result: the demon may not be the saved one)
    M.before = had;
    render();
  }
  /** "No base": its level and alignment (level-scaled effects, and lines for one alignment). The build stays. */
  async function simBlankSet(level, align) {
    const M = S.sim;
    try { await simSetBase('', Math.max(1, Math.min(99, level)), align); } catch (e) { toast('err', 'That could not be worked out', String((e && e.message) || e)); return; }
    await simRun();
    render();
  }
  /** The page of builds. The build that was open keeps its result for its card. */
  function simGoHome() {
    const M = S.sim;
    if (M.view === 'edit' && M.id && M.cur && !M.working && M.state === 'ready') M.cards.set(M.id, { key: simCardKey({ base: M.char, level: M.level, alignment: M.align, demon: M.demon, slots: M.build.slots, points: M.build.points }), res: M.cur });
    simFlush();
    if (S.page === 'sim' && M.view === 'edit') remember();
    M.view = 'home';
    simShow();
  }
  const simNewName = (base) => {
    const names = new Set((S.sim.builds || []).map((b) => b.name));
    if (base) {
      // "<name> copy", then "copy 2"...: the name is shortened to make room for the ending. (Cutting the whole thing to
      // 60 letters cut the ending off a name that was already 60 long, so no free name was ever found: the page hung.)
      const fit = (ending) => String(base).slice(0, 60 - ending.length).trimEnd() + ending;
      let n = fit(' copy'), i = 2;
      while (names.has(n)) n = fit(` copy ${i++}`);
      return n;
    }
    let i = names.size + 1; while (names.has(`Build ${i}`)) i++;
    return `Build ${i}`;
  };
  /** The unsaved build becomes a saved one with this name; "New build" then starts from what you wear again. */
  async function simSaveDraft(name) {
    const M = S.sim;
    const r = await host.call('simSaveBuild', { name, ...simWhole() });
    M.builds = r.builds || [];
    M.id = r.id; M.name = (M.builds.find((b) => b.id === r.id) || {}).name || name;
    M.saveState = 'saved'; M.naming = false;
    store.set(simKey(), '{}');
    drawSimNav();
  }
  async function simDuplicate(b, open) {
    const M = S.sim;
    try {
      const at = simBaseOf(b);
      const r = await host.call('simSaveBuild', { name: simNewName(b.name), ...simArgs(at.who, at.level, at.align), demon: typeof b.demon === 'string' ? b.demon : undefined,
        slots: b.slots, points: b.points, cap: typeof b.cap === 'number' ? b.cap : null });
      M.builds = r.builds || [];
      drawSimNav();
      if (open) simOpen(r.id); else render();
    } catch (e) { toast('err', 'The build could not be copied', String((e && e.message) || e)); }
  }
  async function simDelete(id) {
    const M = S.sim;
    clearTimeout(M.storeTimer); M.storeTimer = null;
    try {
      const r = await host.call('simDeleteBuild', { id });
      M.builds = r.builds || [];
      M.cards.delete(id);
      if (M.id === id) { M.view = 'home'; M.id = null; if (M.state === 'ready') await simUse(null).catch(() => {}); }
      drawSimNav(); render();
    } catch (e) { toast('err', 'The build could not be deleted', String((e && e.message) || e)); }
  }
  // --- sharing a build: a short text code ("BABEL1-…") that another player pastes into "Import build". The reader
  //     writes and reads the code (host "simShareCode" / "simImportCode": crates\imagine\src\build\share_code.rs). A
  //     pasted code is a stranger's input, so the page never looks inside one: it hands the text to the reader and
  //     shows what the reader made of it, as text. A code holds the gear, the expertise points and the level and
  //     alignment they stand on: no character name, no build name, no demon. Importing only ever adds a new build. ---
  const simCodeDlg = $('#sim-code');
  $('#sim-code-close').addEventListener('click', () => simCodeDlg.close());
  let simCodeSeq = 0;   // which filling of the dialog is the current one (an answer for an earlier one is dropped)
  /** "Share": the code of a build ({ base, level, alignment, slots, points, cap }), to copy. */
  async function simShare(spec, name) {
    let code;
    try { code = (await host.call('simShareCode', { base: spec.base, level: spec.level, alignment: spec.alignment, slots: spec.slots, points: spec.points, cap: spec.cap })).code; }
    catch (e) { toast('err', 'This build can’t be shared', String((e && e.message) || e)); return; }
    ++simCodeSeq;
    $('#sim-code-title').textContent = 'Share build';
    const box = h('textarea', { class: 'input sim-code-box', readonly: true, rows: 4, spellcheck: 'false', 'aria-label': 'Build code' });
    box.value = code;
    box.addEventListener('focus', () => box.select());
    const copy = h('button', { type: 'button', class: 'btn btn-primary', onclick: async () => {
      if (await copyText(code)) flashLabel(copy, 'Copied');
      else { box.focus(); box.select(); toast('info', 'Copy it by hand', 'The code is selected: press Ctrl+C.'); }
    } }, 'Copy code');
    simFill($('#sim-code-body'),
      h('p', { class: 'field-help', text: `The code of ${name ? `“${name}”` : 'this build'}. Send it to another player: they paste it into “Import build” in their Build Planner.` }),
      box,
      h('div', { class: 'field-row' }, copy, h('span', { class: 'field-help sim-code-count', text: `${code.length} characters` })),
      h('p', { class: 'dlg-note' }, icon('shield'), h('span', { text: 'The code holds the pieces (with their fusions and mods), the expertise ranks, and the level and alignment they stand on. It does not hold your character’s name, the build’s name or its digitalize demon.' })));
    if (!simCodeDlg.open) simCodeDlg.showModal();
    copy.focus();
  }
  /** Overview's "Export build": the shown character's build as last read (what "New build" starts from), shared as a code. */
  async function exportCurrentBuild(btn) {
    const me = simActive();
    if (!me || (btn && btn.disabled)) return;
    if (btn) btn.disabled = true;
    try {
      const { init } = await simBase(me);
      await simShare({ ...simArgs(me), slots: init.build.slots, points: init.build.points, cap: null }, '');
    } catch (e) { toast('err', 'This build can’t be exported', String((e && e.message) || e)); }
    finally { if (btn) btn.disabled = false; }
  }
  /** "Import build": paste a code, see what it holds, choose what it stands on, and save it as a new build. */
  function simImportOpen() {
    const M = S.sim, mine = ++simCodeSeq, me = simActive();
    let chars = (S.state.characters || []).filter((c) => c.hasData || c.name === me).map((c) => c.name);
    if (me && !chars.includes(me)) chars = [me, ...chars];
    // what was read from the code (build), what it is shown on (who: a character, '' = no base), and its result there
    const st = { build: null, who: me || '', readSeq: 0, showSeq: 0, points: null, ready: false };
    $('#sim-code-title').textContent = 'Import build';
    const box = h('textarea', { class: 'input sim-code-box', rows: 4, spellcheck: 'false', autocomplete: 'off', maxlength: 2048, placeholder: 'BABEL1-…', 'aria-label': 'Build code' });
    const status = h('p', { class: 'field-help sim-code-status', role: 'status' });
    const preview = h('div', { class: 'sim-code-preview' });
    const names = new Set((M.builds || []).map((b) => b.name));
    let fresh = 'Imported build', i = 2;
    while (names.has(fresh)) fresh = `Imported build ${i++}`;
    const nameIn = h('input', { class: 'input sim-name', type: 'text', maxlength: 60, value: fresh, 'aria-label': 'Name of the build', spellcheck: 'false' });
    const saveBtn = h('button', { type: 'button', class: 'btn btn-primary', disabled: true, onclick: () => save() }, 'Save as build');
    const foot = h('div', { class: 'sim-code-foot', hidden: true }, h('label', { class: 'field' }, h('span', { class: 'field-label', text: 'Name' }), nameIn), saveBtn);
    const say = (text, bad) => { status.textContent = text || ''; status.classList.toggle('bad', !!bad); };
    const here = () => mine === simCodeSeq && simCodeDlg.open;

    /** The build on what was chosen: the pieces and ranks by name, as the reader names them. */
    async function show() {
      const b = st.build, seq = ++st.showSeq;
      st.ready = false; saveBtn.disabled = true;
      if (!b) { simFill(preview); foot.hidden = true; return; }
      foot.hidden = false;
      const pick = h('div', { class: 'seg sim-code-on', role: 'group', 'aria-label': 'What the build stands on' },
        chars.map((c) => h('button', { type: 'button', 'aria-pressed': String(st.who === c), 'data-tip': `Its numbers on ${c} as last read: ${c}’s stats, with this gear and these expertise ranks`, onclick: () => { st.who = c; show(); } }, c)),
        h('button', { type: 'button', 'aria-pressed': String(st.who === ''), 'data-tip': `No character behind it: only what the build gives, at level ${n0(b.level)} as ${alignName(b.alignment)}, as it was shared`, onclick: () => { st.who = ''; show(); } }, 'No base'));
      const body = h('div', { class: 'sim-code-list' }, h('p', { class: 'note', text: 'Working it out…' }));
      simFill(preview, h('div', { class: 'field' }, h('span', { class: 'field-label', text: 'Show it on' }), pick), body);
      let entry, res;
      try {
        entry = await simBase(st.who, n0(b.level), b.alignment);
        st.points = { ...Object.fromEntries(Object.keys(entry.init.build.points || {}).map((k) => [k, 0])), ...b.points };   // whole: an expertise the code has no points in is at 0
        res = await host.call('simulate', { ...simArgs(st.who, n0(b.level), b.alignment), demon: '', slots: b.slots, points: st.points });
      } catch (e) {
        if (seq !== st.showSeq || !here()) return;
        simFill(body, h('div', { class: 'notice notice-warn' }, icon('alert'), h('div', null, h('b', { text: st.who ? `${st.who} can’t be used as its base. ` : 'It can’t be worked out. ' }), String((e && e.message) || e), st.who ? ' Pick another character, or No base.' : '')));
        return;
      }
      if (seq !== st.showSeq || !here()) return;
      const pieces = (res.slots || []).filter((x) => x && !x.empty);
      const ranks = (entry.init.expertise || []).filter((x) => !x.chain && n0(st.points[x.id]) >= 10000);
      simFill(body,
        h('p', { class: 'note', text: `${pieces.length} piece${pieces.length === 1 ? '' : 's'} and ${ranks.length} expertise${ranks.length === 1 ? '' : 's'}, made at level ${n0(b.level)} as ${alignName(b.alignment)}.` }),
        pieces.map((x) => h('div', { class: 'sim-code-row' },
          x.icon ? h('img', { src: x.icon, alt: '' }) : h('span'),
          h('span', { class: 'dgt-line', text: x.slot }),
          h('span', { class: 'sim-code-piece' }, h('b', { text: x.name }),
            h('span', { class: 'dgt-line', text: [x.basic ? `Basic features: ${x.basic}` : '', x.special ? `Characteristics: ${x.special}` : '', x.tarot ? `Tarot: ${x.tarot}` : '', x.soul ? `Soul: ${x.soul}` : ''].filter(Boolean).join(' · ') })))),
        ranks.length ? h('div', { class: 'sim-code-ranks' }, ranks.map((x) => h('span', { class: 'tag', text: `${x.name} · ${simRank(Math.floor(n0(st.points[x.id]) / 10000))}` }))) : null);
      st.ready = true; saveBtn.disabled = false;
    }
    /** What is in the box, to the reader; its answer is the build, or why not in plain words. */
    async function read() {
      const text = box.value.trim(), seq = ++st.readSeq;
      st.build = null; show();
      if (!text) { say(''); return; }
      say('Reading the code…');
      try {
        const r = await host.call('simImportCode', { code: text });
        if (seq !== st.readSeq || !here()) return;
        st.build = r.build; say('');
        show();
      } catch (e) { if (seq === st.readSeq && here()) say(String((e && e.message) || e), true); }
    }
    async function save() {
      const b = st.build;
      if (!b || !st.ready) return;
      const name = nameIn.value.replace(/\s+/g, ' ').trim() || fresh;
      saveBtn.disabled = true;
      try {
        const r = await host.call('simSaveBuild', { name, ...simArgs(st.who, n0(b.level), b.alignment), slots: b.slots, points: st.points, cap: typeof b.cap === 'number' ? b.cap : null });
        M.builds = r.builds || [];
        drawSimNav();
        simCodeDlg.close();
        toast('ok', `Imported “${name}”`, 'It is a new build of yours. Nothing else was changed.', 4000);
        simOpen(r.id);
      } catch (e) { saveBtn.disabled = false; toast('err', 'The build could not be saved', String((e && e.message) || e)); }
    }
    let timer = null;
    box.addEventListener('input', () => { clearTimeout(timer); timer = setTimeout(read, 200); });
    nameIn.addEventListener('keydown', (ev) => { if (ev.key === 'Enter') save(); });
    simFill($('#sim-code-body'),
      h('p', { class: 'field-help', text: 'Paste a build code another player shared with you. You see what it holds before anything is saved.' }),
      box, status, preview, foot,
      h('p', { class: 'dlg-note' }, icon('shield'), h('span', { text: 'A code is only numbers: pieces, fusions and expertise ranks, each checked against the game data. Importing adds a new build and changes nothing else.' })));
    if (!simCodeDlg.open) simCodeDlg.showModal();
    box.focus();
  }
  /** The saved builds under "Build Planner" in the left bar; the one in the editor is marked instead of the entry itself. */
  function drawSimNav() {
    const M = S.sim, list = M.builds || [], open = M.navOpen && list.length > 0;
    const editing = S.page === 'sim' && M.view === 'edit' && M.id ? M.id : null;
    simNavToggle.hidden = !list.length;
    simNavToggle.setAttribute('aria-expanded', String(open));
    simNavToggle.dataset.tip = open ? 'Hide your saved builds' : `Show your ${list.length} saved build${list.length === 1 ? '' : 's'}`;
    simNavBox.hidden = !open;
    simFill(simNavBox, list.map((b) => h('button', { type: 'button', class: 'nav-subitem', 'aria-current': b.id === editing ? 'page' : null, 'data-tip': b.name, onclick: () => simOpen(b.id) }, b.name)));
    const entry = nav.querySelector('.nav-item[data-page="sim"]');
    if (entry && editing && open) entry.removeAttribute('aria-current');
  }
  /**
   * The hover tooltip of a piece of a build, laid out as the Gear page's (gearTip): the icon, the name and the slot,
   * then the set bonus and the sections in the Gear page's order. The Gear page shows the game's own tooltip text,
   * which exists only for pieces a character was read wearing; here the sections are what the reader worked out for
   * the build (the lines the editor shows for the piece). `x`: the piece in the build's result; `note`: added to the slot.
   */
  const simPieceTip = (x, res, note) => () => {
    const box = h('div', { class: 'tt' }), L = x.lines || {};
    box.append(tipHead(gearIcon({ icon: x.icon }), x.empty ? 'Nothing worn' : x.name, [x.slot, note].filter(Boolean).join(' · ')));
    if (x.empty) return box;
    let any = false;
    // A section: its heading, then what it comes from as a picture and a name (the piece itself, the piece lending
    // that part, the crystal, a set's pieces), then its lines. `always`: shown even with no lines (a part taken from
    // another piece, or a crystal, that adds nothing is still what the piece is made of).
    const sec = (title, from, list, o) => {
      const { mark = '', icons = [], always = false } = o || {};
      if (!always && !(list && list.length)) return;
      any = true;
      box.append(h('div', { class: 'tt-sec', text: title }));
      if (from) box.append(h('div', { class: 'tt-line tt-why tt-src' }, icons.filter(Boolean).map((src) => h('img', { class: 'tt-src-pic', src, alt: '' })), h('span', { text: from })));
      (list || []).forEach((l) => box.append(h('div', { class: 'tt-line', text: mark + simLineText(l) + (l.n ? ` (${l.n})` : '') })));
    };
    const mine = simPlain(x.name);
    (res.sets || []).filter((s) => s.kind === 'set' && mine && String(s.name || '').includes(mine)).forEach((s) =>
      sec(s.active ? 'Set bonus' : `Set bonus · ${s.worn} worn, not complete`, `Pieces: ${s.name}`, s.lines, { mark: '* ', icons: s.icons || [], always: true }));
    sec('Appearance Bonus', x.name, L.appearance, { icons: [x.icon] });
    sec('Basic Features', x.basic || x.name, L.item, { icons: [x.basic ? x.basic_icon : x.icon], always: !!x.basic });
    // fusion great-success bonuses, as the Gear page marks them
    (x.fuse || []).filter((f) => n0(f.bonus)).forEach((f) => {
      any = true;
      box.append(h('div', { class: 'tt-line' }, `${f.stat}: `, h('span', { class: 'tt-fuse', text: `+${fmt(f.bonus)} from fusion` }), isTrue(f.max) ? h('span', { class: 'tt-max', text: 'MAX' }) : null));
    });
    sec('Characteristics', x.special || x.name, L.characteristic, { icons: [x.special ? x.special_icon : x.icon], always: !!x.special });
    sec('Tarot Fusion', x.tarot || '', L.tarot, { icons: [x.tarot_icon], always: !!x.tarot });
    sec('Soul Fusion', x.soul || '', L.soul, { icons: [x.soul_icon], always: !!x.soul });
    sec('Modification Slot', '', L.mod);
    if (!any) box.append(h('div', { class: 'tt-line', text: 'It adds nothing by itself.' }));
    return box;
  };
  /**
   * The hover tooltip of a piece a build changes: one tooltip with the usual sections (set bonus, appearance bonus,
   * basic features, characteristics, tarot, soul, mods). A section that is the same for the base and the build is
   * written once. One that is not shows the base's first, as the character wears it, and under it what the build has
   * there instead, in a card outlined in the purple of the ring around the picture and headed "Changes to". The name on top works the same
   * way when the piece itself is another. Both sides are what the reader worked out (simPieceTip), so they read alike.
   * `was`, `now`: the slot in the base's result and in the build's (`empty` = nothing worn there).
   */
  const simPieceCompare = (who, was, baseRes, now, res) => () => {
    // each side's tooltip, taken apart: its sections by heading (a set by its pieces, as a piece can be in several)
    const parts = (tt) => {
      const map = new Map(); let cur = null;
      [...tt.children].forEach((el) => {
        if (el.classList.contains('tt-head')) return;
        if (el.classList.contains('tt-sec')) { cur = { title: el.textContent, nodes: [] }; map.set(el.textContent.replace(/ · .*$/, ''), cur); return; }
        if (!cur) return;
        if (/^Set bonus/.test(cur.title) && !cur.nodes.length) { map.delete(cur.title.replace(/ · .*$/, '')); map.set('Set bonus: ' + el.textContent, cur); }
        cur.nodes.push(el);
      });
      map.forEach((p) => { p.text = p.title + ' | ' + p.nodes.map((n) => n.textContent).join(' | '); });
      return map;
    };
    const A = simPieceTip(was, baseRes, '')(), B = simPieceTip(now, res, '')();
    const pa = parts(A), pb = parts(B);
    /** What the build has instead, under the base's: a purple-outlined card headed "Changes to", its text in purple. */
    const fresh = (...kids) => h('div', { class: 'tt-new' }, h('div', { class: 'tt-new-cap', text: 'Changes to' }), kids);
    const gone = (text) => h('div', { class: 'tt-line tt-why', text });
    const box = h('div', { class: 'tt tt-cmp' });
    // the name on top: once when it is the same piece, else the base's and under it the build's
    const headA = A.querySelector('.tt-head'), headB = B.querySelector('.tt-head');
    if (headA.textContent === headB.textContent) box.append(headB);
    else { headA.classList.add('tt-head-was'); box.append(h('div', { class: 'tt-new-cap', text: `${who} wears` }), headA, fresh(headB)); }
    // the sections in the tooltip's own order: the sets first, then the fixed ones; one only a side has keeps its place
    const fixed = ['Appearance Bonus', 'Basic Features', 'Characteristics', 'Tarot Fusion', 'Soul Fusion', 'Modification Slot'];
    const keys = [...new Set([...pa.keys(), ...pb.keys()])];
    const order = (k) => { const i = fixed.indexOf(k); return i < 0 ? -1 : i; };
    keys.sort((x, y) => order(x) - order(y));
    keys.forEach((k) => {
      const x = pa.get(k), y = pb.get(k);
      if (x && y && x.text === y.text) { box.append(h('div', { class: 'tt-sec', text: y.title }), ...y.nodes); return; }
      box.append(h('div', { class: 'tt-sec', text: (x || y).title }));
      if (x) box.append(...x.nodes);
      // a set complete on one side only words its heading differently: the build's heading goes in its block
      box.append(fresh(x && y && x.title !== y.title ? h('div', { class: 'tt-line tt-why', text: y.title }) : null, y ? y.nodes : gone(now.empty ? 'Nothing worn' : 'None')));
    });
    if (!keys.length) box.append(h('div', { class: 'tt-line', text: 'Neither adds anything by itself.' }));
    return box;
  };
  /** One saved build as a card: its name, its base, how far it is from that base, and the biggest differences it makes. */
  function simCard(b) {
    const M = S.sim, key = simCardKey(b), hit = M.cards.get(b.id), known = hit && hit.key === key ? hit : null, res = known ? known.res : null;
    const at = simBaseOf(b), entry = M.bases.get(simBaseKey(at.who, at.level, at.align)) || null;
    const c = entry ? simChanges(b, entry.init) : { slots: [], ranks: [], total: 0 }, n = c.total;
    // "Rubito · 2 changes", or without a base what was put in: "No base · 9 pieces, 3 expertises"
    const count = !entry ? '' : at.who ? (n ? `${n} change${n === 1 ? '' : 's'}` : 'No changes')
      : [c.slots.length ? `${c.slots.length} piece${c.slots.length === 1 ? '' : 's'}` : '', c.ranks.length ? `${c.ranks.length} expertise${c.ranks.length === 1 ? '' : 's'}` : ''].filter(Boolean).join(', ') || 'Empty';
    const el = h('div', { class: 'dg-tile sim-build', onclick: (ev) => { if (!ev.target.closest('.sim-build-acts')) simOpen(b.id); } });
    const acts = h('span', { class: 'sim-build-acts' });
    const drawActs = (asking) => simFill(acts, asking
      ? [h('span', { class: 'dgt-line', text: 'Delete it?' }), h('button', { type: 'button', class: 'btn btn-sm', onclick: () => simDelete(b.id) }, 'Delete'),
        h('button', { type: 'button', class: 'btn btn-ghost btn-sm', onclick: () => drawActs(false) }, 'Keep')]
      : [h('button', { type: 'button', class: 'btn btn-ghost btn-square btn-sm', 'aria-label': `Share ${b.name}`, 'data-tip': 'Share: a code another player can import',
          onclick: () => simShare({ ...simArgs(at.who, at.level, at.align), slots: b.slots, points: b.points, cap: typeof b.cap === 'number' ? b.cap : null }, b.name) }, icon('share')),
        h('button', { type: 'button', class: 'btn btn-ghost btn-square btn-sm', 'aria-label': `Duplicate ${b.name}`, 'data-tip': 'Duplicate', onclick: () => simDuplicate(b, false) }, icon('copy')),
        h('button', { type: 'button', class: 'btn btn-ghost btn-square btn-sm', 'aria-label': `Delete ${b.name}`, 'data-tip': 'Delete', onclick: () => drawActs(true) }, icon('trash'))]);
    drawActs(false);
    // the left column: the build's pieces as pictures, one box per slot in the game's order (an empty slot stays an
    // empty box, so a piece is always in the same place); on a character's build the pieces that differ are ringed
    const changed = new Set(at.who ? c.slots : []);
    // Hovering a piece shows the Gear page's tooltip. A piece the shown character wears, unchanged in this build, gets
    // that very tooltip (the game's own text); any other piece gets the same layout from what the reader worked out.
    const wornNow = entry && at.who && at.who === simActive() && S.v && S.v.gear ? S.v.gear : [];
    const col = h('span', { class: 'sim-build-col' }, Array.from({ length: 15 }, (_, k) => {
      const x = res && res.slots[k] && !res.slots[k].empty ? res.slots[k] : null;
      const box = h('span', { class: 'sim-build-slot' + (x ? '' : ' none') + (res && changed.has(k) ? ' changed' : '') },
        x && x.icon ? h('img', { src: x.icon, alt: '' }) : null);
      // a ringed piece (or a ringed slot the build leaves empty) shows the two side by side: the base's, and the build's
      if (res && changed.has(k) && res.slots[k]) return withTip(box, simPieceCompare(at.who, (entry.init.result.slots || [])[k] || { slot: res.slots[k].slot, empty: true }, entry.init.result, res.slots[k], res));
      if (!x) return box;
      const worn = wornNow.find((g) => g.kind !== 'set' && g.item_id === x.id && String(g.slot).toLowerCase() === String(x.slot).toLowerCase());
      return withTip(box, worn ? gearTip(worn) : simPieceTip(x, res, ''));
    }));
    // what it changes: your stats first, then combat, then the rest; within each, the biggest first
    const rank = { Stats: 0, Combat: 1, Affinity: 2, Other: 3 };
    const diffs = res && entry ? simDiff(entry.init.result, res).filter((x) => rank[x.group] !== undefined).sort((x, y) => rank[x.group] - rank[y.group] || y.size - x.size) : [];
    simFill(el, col, h('span', { class: 'sim-build-body' },
      h('span', { class: 'sim-build-head' },
        h('span', { class: 'dgt-id' }, h('button', { type: 'button', class: 'sim-build-name', 'data-tip': `Open ${b.name}`, text: b.name }),
          h('span', { class: 'dgt-line', text: [at.who || 'No base', count].filter(Boolean).join(' · ') })),
        acts),
      !known ? h('span', { class: 'dgt-line', text: 'Working it out…' })
        : !res ? h('span', { class: 'dgt-line', text: known.why || 'This build can’t be worked out with the game data as it is now.' })
        : diffs.length ? h('span', { class: 'dgt-adds' }, diffs.slice(0, 4).map((x) => h('span', { class: 'dgt-add' }, h('span', { text: x.label }), h('b', { class: x.good ? 'good' : 'bad', text: fmt(x.d, x.unit, true) }))),
          diffs.length > 4 ? h('span', { class: 'dgt-line', text: `and ${diffs.length - 4} more` }) : null)
        : n ? h('span', { class: 'dgt-line', text: 'Moves none of your numbers.' }) : null,
      b.saved_at ? h('span', { class: 'dgt-line sim-build-saved', text: `Saved ${when(b.saved_at)}` }) : null));   // at the foot of the card
    // its result, one card after the other; the card is drawn again when it is in
    if (!known) M.jobs = M.jobs.then(async () => {
      const now = M.cards.get(b.id);
      if (!(now && now.key === key)) {
        let r = null, why = '';
        let its = null;
        try { its = await simBase(at.who, at.level, at.align); }
        catch (e) { why = at.who ? `Its base, ${at.who}, can’t be read: ${String((e && e.message) || e)}` : String((e && e.message) || e); }
        if (!why) try { r = await host.call('simulate', { ...simArgs(at.who, at.level, at.align), ...(await simDemonArgs(simDemonOf(b, its.init))), slots: b.slots, points: b.points }); } catch (e) { r = null; }
        M.cards.set(b.id, { key, res: r, why });
      }
      if (el.isConnected) el.replaceWith(simCard(b));
    });
    return el;
  }

  // "Import base": the saved characters, as the rows of the Characters window, in a dropdown under the button in the
  // editor. Picking one starts the build again from that character's gear and expertise as last read, and the build's
  // numbers are then that character's. "No base" goes back to an empty build.
  const simBasesMenu = $('#sim-bases');
  let simBasesFor = null;   // the button it hangs from
  function drawSimBases() {
    const M = S.sim, me = simActive();
    let chars = (S.state.characters || []).filter((c) => c.hasData || c.name === me);
    if (me && !chars.some((c) => c.name === me)) chars = [{ name: me }, ...chars];
    const row = (who, name, line, pic) => h('button', { type: 'button', class: 'char', 'aria-current': who === M.char ? 'true' : null, onclick: () => { simBasesMenu.hidePopover(); simRebase(who); } },
      pic ? h('img', { class: 'char-pic', src: pic, alt: '' }) : who ? h('span', { class: 'char-dot' }) : h('span', { class: 'dm-icon dm-none dm-off', text: '–' }),
      h('span', { class: 'char-text' }, h('span', { class: 'char-name', text: name }), h('span', { class: 'char-line', text: line })),
      who === M.char ? h('span', { class: 'tag', text: 'Base now' }) : null);
    simFill($('#sim-bases-list'),
      chars.map((c) => row(c.name, c.name, 'Gear, expertise and stats as last read', (c.name === me ? S.meta.picture : null) || c.picture || null)),
      row('', 'No base', 'An empty build: only what it gives by itself', null));
  }
  simBasesMenu.addEventListener('toggle', (ev) => {
    if (ev.newState !== 'open') return;
    drawSimBases();
    if (!simBasesFor || !simBasesFor.isConnected) return;
    const r = simBasesFor.getBoundingClientRect(), w = simBasesMenu.offsetWidth, hgt = simBasesMenu.offsetHeight;
    simBasesMenu.style.left = Math.max(8, Math.min(r.right - w, window.innerWidth - w - 8)) + 'px';   // under the button, right edges together
    simBasesMenu.style.top = (r.bottom + 6 + hgt > window.innerHeight - 8 ? Math.max(8, r.top - 6 - hgt) : r.bottom + 6) + 'px';
    const b = $('#sim-bases-list [aria-current="true"]') || $('#sim-bases-list button');
    if (b && simBasesFor.matches(':focus-visible')) b.focus();   // opened from the keyboard: go into the list
  });
  $('#main').addEventListener('scroll', () => { if (simBasesMenu.matches(':popover-open')) simBasesMenu.hidePopover(); });   // its button moves away
  // The demon picker dialog, shared by the Character Planner's demon band and the Demon Planner's "Import demon", in the
  // gear picker's dialog (#sim-pick): the COMP demons as last seen ("My demons") and the Demon Planner's saved builds as
  // they end ("From Demon Planner"), with a search, chips with counts, the keyboard (arrows, Enter), each demon's stats,
  // effects, skills, Demon Force, equipment and rebirth ranks. What differs is in `cfg`:
  //   title, sub        the dialog's head
  //   load()            -> { mine, plans, skillIcons }: the two lists (again after `refresh`)
  //   none              a "No demon" row first (the Character Planner)
  //   current(o)        the tag a row gets when it is what the page has now ("In use"), or null
  //   choose(o)         what picking a row does (the dialog closes first)
  //   previewRes(o)     the Character Planner's strip: the build's result with that demon, compared with cfg.cur()
  //   foot(refresh)     else: what goes where the strip is (Import demon: "Load demons from the game")
  async function demonPick(cfg) {
    const list = $('#sim-pick-list'), tools = $('#sim-pick-tools'), preview = $('#sim-pick-preview');
    const note = (text) => h('p', { class: 'note', style: 'padding:12px', text });
    simDlg.classList.add('sim-dpick');
    $('#sim-pick-title').textContent = cfg.title;
    $('#sim-pick-sub').textContent = cfg.sub;
    tools.replaceChildren(); preview.replaceChildren();
    list.replaceChildren(note('Reading your demons and your demon builds…'));
    if (!simDlg.open) simDlg.showModal();
    let alive = true, timer = null, look = 0;
    simPickDone = () => { alive = false; clearTimeout(timer); simDlg.classList.remove('sim-dpick'); };
    let got = await cfg.load();
    if (!alive) return;

    const GROUPS = [['mine', 'My demons'], ['plan', 'From Demon Planner']];
    let mine = [], plans = [], opts = [];
    const setLists = (r) => {
      mine = r.mine || []; plans = r.plans || [];
      opts = [...(cfg.none ? [{ key: '', name: 'No demon', group: 'none', none: true }] : []), ...mine.map((d) => ({ ...d, group: 'mine' })), ...plans.map((d) => ({ ...d, group: 'plan' }))];
      opts.forEach((o) => {
        const O = o.offer;
        o.hay = [o.name, o.demonName || '', o.race || '', O && O.alignment ? alignName(O.alignment) : '', o.mitama ? `${o.mitama} mitama` : '', o.group === 'plan' ? 'demon planner build' : '',
          ...(O ? (O.skills || []).map((s) => s.name) : []), ...(O ? COMBAT.filter(([k]) => n0(O.combat && O.combat[k])).map(([, l]) => l) : [])].join(' ').toLowerCase();
      });
    };
    setLists(got);
    const st = { q: '', group: 'all', active: 0, rows: [], amountLabel: null };
    // Searching a stat (SPD, Max HP...) or an effect (movement speed, rapid) puts the demons that give the most of it
    // first; damage taken and cooldown: the lowest first.
    const SHORT = { Strength: 'STR', Magic: 'MAG', Vitality: 'VIT', Intelligence: 'INT', Speed: 'SPD', Luck: 'LUCK', 'Close-Range': 'CLS-R', 'Long-Range': 'LNG-R' };
    const amountOf = (words, q) => {
      if (!words.length) return null;
      const stat = SIM_STAT_OF.get(simStatKey(q));
      if (stat) {
        const short = SHORT[stat] || stat;
        return { label: short, of: (O) => { const s = (O.stats || []).find((x) => x.stat === short); return s && n0(s.value) ? n0(s.value) : null; } };
      }
      const c = COMBAT.find(([, l]) => words.every((w) => l.toLowerCase().includes(w)));
      return c ? { label: c[1], of: (O) => (n0(O.combat && O.combat[c[0]]) ? n0(O.combat[c[0]]) * (/^taken_|cooldown/.test(c[0]) ? -1 : 1) : null) } : null;
    };
    const choose = (o) => { simDlg.close(); cfg.choose(o); };
    // the Character Planner: what picking the highlighted one would do, the build with that demon against the build now
    const showPreview = (o) => {
      if (!cfg.previewRes) return;
      clearTimeout(timer);
      if (!o) { preview.replaceChildren(); return; }
      if (cfg.current(o)) { preview.replaceChildren(h('span', { class: 'sim-preview-label', text: 'In use now.' })); return; }
      const label = `With ${o.none ? 'no demon' : o.name}:`;
      simPreviewWait(preview, label);
      const mine2 = ++look;
      timer = setTimeout(async () => {
        let res;
        try { res = await cfg.previewRes(o); } catch (e) { res = null; }
        if (!alive || mine2 !== look) return;
        const head = h('span', { class: 'sim-preview-label', text: label }), cur = cfg.cur();
        if (!res || !cur) { preview.replaceChildren(head, h('span', { class: 'note', text: 'could not be worked out.' })); return; }
        const d = simDiff(cur, res, true).filter((x) => x.group !== 'Only in some fights' && x.group !== 'Partner');
        if (!d.length) { preview.replaceChildren(head, h('span', { class: 'note', text: 'no change to your numbers.' })); return; }
        const top = d.slice(0, 9);
        simFill(preview, head, top.map((x) => h('span', { class: 'sim-chip ' + (x.good ? 'good' : 'bad'), 'data-tip': `${x.label}: ${fmt(x.from, x.unit)} → ${fmt(x.to, x.unit)}` },
          h('span', { text: x.label }), h('b', { text: fmt(x.d, x.unit, true) }))), d.length > top.length ? h('span', { class: 'note', text: `and ${d.length - top.length} more` }) : null);
      }, 60);   // a short pause: moving across rows asks only for the one the pointer stops on
    };
    const setActive = (i, scroll) => {
      if (i === st.active && !scroll) return;
      st.active = Math.max(0, Math.min(st.rows.length - 1, i));
      list.querySelectorAll('.sim-opt').forEach((el) => { const on = Number(el.dataset.i) === st.active; el.setAttribute('aria-selected', String(on)); if (on && scroll) el.scrollIntoView({ block: 'nearest' }); });
      showPreview(st.rows[st.active]);
    };
    const pic = (o) => {
      const O = o.offer, p = h('span', { class: 'demon-pic sim-dpic ' + (o.none ? 'none' : alignClass(O && O.alignment)), 'aria-hidden': 'true' }, h('span', { text: o.none ? '–' : String(o.name || '?').charAt(0) }));
      if (o.icon) p.append(h('img', { src: o.icon, alt: '' }));
      if (o.mitama) mitamaMark(p, o.mitama);
      return p;
    };
    const hit = (label) => (st.amountLabel === label ? 'sim-hit' : null);
    const row = (o, i) => {
      const O = o.offer, plan = o.group === 'plan', now = cfg.current(o);
      const stats = O ? (O.stats || []).filter((s) => n0(s.value)) : [];
      const adds = O ? COMBAT.filter(([k]) => n0(O.combat && O.combat[k])) : [];
      const skills = O ? (O.skills || []).filter((s) => s.kind !== 'Feature' && s.kind !== 'Equipment') : [];
      return h('div', { class: 'sim-opt with-pic sim-dopt', role: 'option', 'aria-selected': String(i === st.active), 'data-i': i, onclick: () => choose(o), onpointermove: () => setActive(i) },
        pic(o),
        h('div', { class: 'sim-opt-body' },
          h('div', { class: 'sim-opt-head' }, h('b', { text: o.name }),
            o.none ? null : h('span', { class: 'tag', text: `Lv ${fmt(o.level)}` }), o.race ? h('span', { class: 'tag', text: o.race }) : null,
            O && O.alignment ? h('span', { class: 'tag tag-align ' + alignClass(O.alignment), text: alignName(O.alignment) }) : null,
            o.mitama ? h('span', { class: 'tag', text: `${o.mitama} Mitama` }) : null,
            plan ? h('span', { class: 'tag', 'data-tip': `Your Demon Planner build “${o.name}”: ${o.demonName} as the plan ends${o.planned ? '. The Mitamas it only plans count for the sets, not for their random bonus' : ''}`, text: o.demonName !== o.name ? `Demon Planner · ${o.demonName}` : 'Demon Planner' }) : null,
            now ? h('span', { class: 'tag sim-tag-use', text: now }) : null,
            o.group === 'mine' ? demonUnlockTags(o) : null),
          o.none ? h('div', { class: 'sim-opt-lines', text: 'Not digitalized: only what the build itself gives.' })
            : O ? h('div', { class: 'sim-opt-lines' }, `${fmt(O.rate)}% of its stats`, stats.map((s) => [' · ', h('span', { class: hit(s.stat), text: `${s.stat} ${fmt(s.value, '', true)}` })]))
            : h('div', { class: 'sim-opt-lines zero', text: o.loaded === false ? 'Not read yet. Click it in the COMP in game, or open the fusion window at the Cathedral of Shadows (that loads your whole COMP), then load the demons again.' : 'What it gives can’t be worked out for this character.' }),
          adds.length ? h('div', { class: 'sim-opt-lines' }, adds.map(([k, l, u], j) => [j ? ' · ' : null, h('span', { class: hit(l), text: `${l} ${fmt(O.combat[k], u, true)}` })])) : null,
          o.none || (o.loaded === false && !O) ? null : h('div', { class: 'sim-dbars' },
            skills.length ? h('span', { class: 'dg-skill-icons sim-dskills', role: 'group', 'aria-label': 'Skills' }, skills.map((s) => {
              const ic = skillIcon(s, got.skillIcons);
              if (!isTrue(s.counted)) ic.classList.add('off');   // dimmed: you don't get it while digitalized (the race's boost isn't learned)
              return withTip(ic, () => h('div', null, tipHead(skillIcon(s, got.skillIcons), s.name, s.kind), ...tipLines(s.tip), skillWhy(s)));
            })) : null,
            simForceBar(o), simEquipBar(o))),
        simRankRadar(o));
    };
    const draw = () => {
      const words = st.q.trim().toLowerCase().split(/\s+/).filter(Boolean);
      const A = amountOf(words, st.q.trim());
      st.amountLabel = A ? A.label : null;
      const val = (o) => (A && o.offer ? A.of(o.offer) : null);
      st.rows = opts.filter((o) => (st.group === 'all' || o.none || o.group === st.group) && (!words.length || (!o.none && ((A && val(o) !== null) || words.every((w) => o.hay.includes(w))))));
      if (A) st.rows.sort((a, b) => (val(b) === null ? -Infinity : val(b)) - (val(a) === null ? -Infinity : val(a)));
      sortNote.textContent = A ? `Most ${A.label} first` : '';
      st.active = Math.min(st.active, Math.max(0, st.rows.length - 1));
      const out = [], shown = new Set();
      st.rows.forEach((o, i) => {
        // each part under its heading, as long as the list keeps their order (a search by amount mixes them)
        if (!A && !o.none && !shown.has(o.group)) { shown.add(o.group); out.push(h('div', { class: 'list-head sim-dhead', text: GROUPS.find(([g]) => g === o.group)[1] })); }
        out.push(row(o, i));
      });
      if (!words.length) {   // an empty part says why
        if ((st.group === 'all' || st.group === 'mine') && !mine.length) out.splice(cfg.none ? 1 : 0, 0, h('div', { class: 'list-head sim-dhead', text: 'My demons' }), note(cfg.noMine || 'No demons seen yet. Open the COMP in game, then load the demons.'));
        if ((st.group === 'all' || st.group === 'plan') && !plans.length) out.push(h('div', { class: 'list-head sim-dhead', text: 'From Demon Planner' }), note('No saved demon builds. Plan a demon in the Demon Planner and press “Save build”: it shows up here.'));
      }
      list.replaceChildren(...out);
      if (!st.rows.length && words.length) list.append(h('p', { class: 'empty-row', text: `Nothing matches “${st.q.trim()}”.` }));
      showPreview(st.rows[st.active]);
    };
    const input = h('input', { class: 'input', type: 'search', spellcheck: 'false', 'aria-label': 'Search', placeholder: 'Search a demon, race, skill or effect (e.g. spd, movement)',
      oninput: () => { st.q = input.value; st.active = 0; draw(); list.scrollTop = 0; } });
    input.addEventListener('keydown', (ev) => {
      if (ev.key === 'ArrowDown' || ev.key === 'ArrowUp') { ev.preventDefault(); setActive(st.active + (ev.key === 'ArrowDown' ? 1 : -1), true); }
      else if (ev.key === 'Enter') { ev.preventDefault(); if (st.rows[st.active]) choose(st.rows[st.active]); }
    });
    const sortNote = h('span', { class: 'note', role: 'status' });
    const chips = h('div', { class: 'seg', role: 'group', 'aria-label': 'Which demons' });
    const drawChips = () => {
      const count = (g) => opts.filter((o) => (g === 'all' ? !o.none : o.group === g)).length;
      simFill(chips, [['all', 'All'], ...GROUPS].map(([id, label]) =>
        h('button', { type: 'button', 'aria-pressed': String(st.group === id), onclick: () => { st.group = id; st.active = 0; drawChips(); draw(); list.scrollTop = 0; } },
          `${label} `, h('span', { class: 'sim-dcount', text: String(count(id)) }))));
    };
    drawChips();
    simFill(tools, h('div', { class: 'search' }, icon('search'), input), chips, sortNote);
    // read the lists again (Import demon's "Load demons from the game"), keeping the search and the part shown
    const refresh = async () => { got = await cfg.load(); if (!alive) return; setLists(got); drawChips(); draw(); };
    if (!cfg.previewRes && cfg.foot) simFill(preview, cfg.foot(refresh));
    // opens on what the page has now
    const at = opts.findIndex((o) => cfg.current(o));
    st.active = at < 0 ? 0 : at;
    draw();
    const cur = list.querySelector('.sim-opt[aria-selected="true"]');
    if (cur) cur.scrollIntoView({ block: 'nearest' });
    input.focus();
  }
  /** The Character Planner's demon band: the base's COMP demons and the demon builds, digitalizing the build in the editor. */
  async function simDemonPick() {
    const M = S.sim, entry = () => M.bases.get(simBaseKey(M.char, M.level, M.align)) || {};
    return demonPick({
      title: 'Digitalize demon',
      sub: `${M.char}’s COMP demons as last seen, and your Demon Planner builds as they end. The pick is this build’s: ${M.char}’s own digitalize demon is not changed.`,
      none: true,
      noMine: `No demons seen for ${M.char} yet. Open the COMP in game as ${M.char}, then load the demons on the Digitalize page.`,
      load: async () => { const [mine, plans] = await Promise.all([simDemonList(), simPlanList()]); return { mine, plans, skillIcons: entry().skillIcons }; },
      current: (o) => (o.key === M.demon ? 'In use' : null),
      choose: (o) => { if (o.key === M.demon) return; M.demon = o.key; simRun(); render(); },
      previewRes: async (o) => host.call('simulate', { ...simNow(), ...(await simDemonArgs(o.key)), slots: M.build.slots, points: M.build.points }),
      cur: () => M.cur,
    });
  }
  /**
   * A demon's Demon Force stack as the Demon Planner's gallery draws it: 8 squares, the effect's item picture in a filled
   * one, a dashed one empty, a striped one that its form doesn't open. From the COMP reading (forceStackIds) or the
   * demon build as it ends. null while the Demon Planner's tables aren't in.
   */
  function simForceBar(o) {
    const P = S.rbp.state === 'ready' ? S.rbp.P : null, byId = P ? rbpForceById(P) : null;
    if (!byId || !Array.isArray(o.forceStackIds)) return null;
    const ids = o.forceStackIds.slice(0, RBP_FORCE_SLOTS).map(n0);
    while (ids.length < RBP_FORCE_SLOTS) ids.push(0);
    const count = rbpForceCount(P, o.type), name = o.demonName || o.name;
    return h('span', { class: 'rbp-minislots sim-dbar', role: 'group', 'aria-label': 'Demon Force' }, ids.map((id, i) => {
      const e = id ? rbpForceEffect(P, byId, id) : null, locked = i >= count;
      if (!e) return locked ? h('span', { class: 'rbp-mini rbp-mini-locked', 'data-tip': `Demon Force slot ${i + 1}: ${name} opens ${count} slots` })
        : h('span', { class: 'rbp-mini rbp-mini-blank', 'data-tip': `Demon Force slot ${i + 1}: empty` });
      return h('span', { class: 'rbp-mini', 'data-tip': `Demon Force: ${e.name}${e.texts.length ? '\n' + e.texts.join('\n') : ''}` },
        rbpForcePic(e.item, 22, 'rbp-mini-pic') || h('span', { class: 'rbp-mini-pic rbp-mini-q', text: e.item ? '?' : '✦' }));
    }));
  }
  /**
   * A demon's rebirth ranks as the Demon Planner gallery's radar, small, in a column of its own at the end of the row: a
   * COMP demon's ranks as last read; a demon build's as the plan ends, with the ranks before the Mitama fusion as the
   * second outline when the plan fuses it. An empty cell when there are none to draw (no demon, not read yet).
   */
  function simRankRadar(o) {
    const P = S.rbp.state === 'ready' ? S.rbp.P : null, box = h('span', { class: 'sim-dradar' });
    const post = Array.isArray(o.reunion) && o.reunion.length >= 12 ? o.reunion.slice(0, 12).map(n0) : null, pre = Array.isArray(o.preRanks) ? o.preRanks.map(n0) : null;
    if (!P || o.none || !post || (o.loaded === false && !o.offer)) return box;
    const sum = (r) => r.reduce((a, b) => a + b, 0);
    const tip = pre ? `${sum(pre)} rebirth ranks before the Mitama fusion, then ${sum(post)} after it, as planned` : `${sum(post)} rebirth rank${sum(post) === 1 ? '' : 's'} on the runes${o.group === 'plan' ? ', as planned' : ', as last read'}`;
    box.setAttribute('data-tip', tip);
    box.append(rbpRanksRadar(P, pre, post));
    return box;
  }
  /** The same for its 4 equipment slots (the Epitaph Parts worn in place of its features). */
  function simEquipBar(o) {
    const P = S.rbp.state === 'ready' ? S.rbp.P : null, eq = P ? rbpEqById(P) : null;
    if (!eq || !Array.isArray(o.equipmentIds)) return null;
    const ids = o.equipmentIds.slice(0, RBP_EQ_SLOTS).map(n0);
    while (ids.length < RBP_EQ_SLOTS) ids.push(0);
    return h('span', { class: 'rbp-minislots sim-dbar', role: 'group', 'aria-label': 'Equipment' }, ids.map((id, i) => {
      const it = id ? eq.get(id) : null;
      if (!it) return h('span', { class: 'rbp-mini rbp-mini-blank', 'data-tip': id ? `Equipment slot ${i + 1}: a part the game data doesn’t list` : `Equipment slot ${i + 1}: no Epitaph Part` });
      return h('span', { class: 'rbp-mini', 'data-tip': `Equipment: ${it.name}\n${rbpEqText(it)}` },
        rbpEqPic(it.id, 22) || h('span', { class: 'rbp-mini-pic rbp-mini-q', text: String(it.name || '?').charAt(0) }));
    }));
  }
  /** Puts a dropdown under its button (above it when there is no room below). */
  function simPlace(menu, btn) {
    if (!btn || !btn.isConnected || !menu.matches(':popover-open')) return;
    const r = btn.getBoundingClientRect(), w = menu.offsetWidth, hgt = menu.offsetHeight;
    menu.style.left = Math.max(8, Math.min(r.left, window.innerWidth - w - 8)) + 'px';
    menu.style.top = (r.bottom + 6 + hgt > window.innerHeight - 8 ? Math.max(8, r.top - 6 - hgt) : r.bottom + 6) + 'px';
  }
  // The Rebirth Planner's demon picker: under the name in the planner's demon card, any demon from the Demon Database
  // with a search. "Import demon" (top right) is the other way in: one of the COMP demons as last seen (S.rbp.comp),
  // with its ranks and Mitama slots; "Load demons from the game" there looks at the game again (the same read the
  // Overview's picker does). Either pick is the planner's own: the digitalize demon is not changed.
  const rbpDemonsMenu = $('#rbp-demons');
  let rbpDemonsFor = null, rbpDemonsLook = 0;
  function drawRbpDemons() {
    const U = S.rbp, list = $('#rbp-demons-list'), search = $('#rbp-demons-search');
    $('#rbp-demons-note').textContent = 'Only demons the game can Mitama-fuse and that you can get some way: a contract, a fusion, a DCM, an exchange or a gift.';
    // the Demon Database's demons that can be planned: ones with a Mitama form (the server Mitama-fuses only a type
    // with a mitamaFusionID, CharacterManager::MitamaDemon) that the player can get (ddbCanOwn); a search narrows
    // them, the first 80 are listed
    const d = S.ddb.data, q = (search.querySelector('input').value || '').trim().toLowerCase(), words = q.split(/\s+/).filter(Boolean);
    const all = d ? d.demons.filter((x) => rbpPlannable(x) && words.every((w) => (x.name + ' ' + x.race).toLowerCase().includes(w))) : [];
    all.sort((a, b) => (q && a.name.toLowerCase().startsWith(q) ? 0 : 1) - (q && b.name.toLowerCase().startsWith(q) ? 0 : 1) || a.name.localeCompare(b.name) || a.level - b.level);
    const shown = all.slice(0, 80);
    list.replaceChildren(...shown.map((x) => h('button', { type: 'button', class: 'char', 'aria-current': U.src === 'db' && x.id === U.demonId ? 'true' : null,
      onclick: () => { rbpDemonsMenu.hidePopover(); U.src = 'db'; U.demonId = x.id; U.compKey = null; U.compSnap = null; rbpSave(); render(); } },
      ddbImg(x.icon || (d && d.noIcon), 32, 'dm-icon rbp-dm-icon'), h('span', { class: 'char-text' }, h('span', { class: 'char-name', text: x.name }), h('span', { class: 'char-line', text: `Lv ${x.level} · ${x.race}${x.mt ? ' · has a Mitama form' : ''}` })))));
    $('#rbp-demons-status').textContent = !d ? 'Loading the demons…' : !all.length ? 'No demon matches.' : all.length > shown.length ? `${shown.length} of ${all.length} shown: type more to narrow it.` : '';
    simPlace(rbpDemonsMenu, rbpDemonsFor);
  }
  /**
   * "Import demon": the Character Planner's demon dialog. My demons are the COMP demons as last read (S.rbp.comp);
   * importing one plans it from where it stands (its ranks, Mitama slots, skills, Demon Force and equipment). A demon
   * build from the second part opens as it is. Where the Character Planner has its numbers strip, this one has "Load
   * demons from the game" (the same read the Overview's picker does). Nothing here changes the digitalize demon.
   */
  function rbpImportPick() {
    const U = S.rbp, cache = U.planCache || (U.planCache = {});
    return demonPick({
      title: 'Import demon',
      sub: 'Your COMP demons as BABEL last read them, and your saved demon builds. Importing a COMP demon plans it from where it stands: its rebirth ranks, Mitama slots, skills, Demon Force and equipment. A demon build opens as it is. Your digitalize demon is not changed.',
      // the COMP demons as last read, through the Character Planner's command (the shown character's list, with what
      // each gives): both dialogs list the same demons, also in the browser preview, where only those commands reach
      // the reader; the planner's own list (S.rbp.comp) follows it
      load: async () => {
        let mine = U.comp || [], icons = null;
        try { const r = await host.call('simDemons', {}); if (r && Array.isArray(r.demons)) { mine = U.comp = r.demons; icons = r.skillIcons || null; } } catch (e) { /* no reader: as the planner had them */ }
        const plans = await demonPlanList(cache);
        return { mine, plans, skillIcons: icons };
      },
      current: (o) => (o.group === 'plan' ? (U.view === 'edit' && U.id && o.key === SIM_PLAN + U.id ? 'Open' : null) : (U.src === 'comp' && o.key === U.compKey ? 'In the planner' : null)),
      choose: (o) => {
        if (o.group === 'plan') { rbpOpen(o.key.slice(SIM_PLAN.length)); return; }
        U.src = 'comp'; U.compKey = o.key; U.compSnap = rbpSnap(o); rbpSave(); render();
      },
      foot: (refresh) => {
        const status = h('span', { class: 'field-help', role: 'status' });
        const btn = h('button', { type: 'button', class: 'btn btn-sm btn-primary', onclick: async () => {
          const look = ++rbpDemonsLook;
          btn.disabled = true; btn.textContent = 'Reading the game…'; status.textContent = '';
          try {
            const demons = await host.call('compDemons', {});
            if (look !== rbpDemonsLook) return;
            U.comp = Array.isArray(demons) ? demons : [];
            S.demons = U.comp;   // the Overview's picker sees the same read
            const live = U.comp.filter((d) => d.inGame).length;
            status.textContent = !host.native ? 'Browser preview: stand-in demons.' : !live ? 'The game isn’t running, or has no demons loaded. The list is as before.' : live === 1 ? 'Only the summoned demon was loaded. Open the COMP in game, then load again for all of them.' : `${live} demons in your COMP.`;
            await refresh();
            if (S.page === 'rebirth') render();
          } catch (e) { if (look === rbpDemonsLook) status.textContent = String((e && e.message) || e); }
          btn.disabled = false; btn.textContent = 'Load demons from the game';
        } }, 'Load demons from the game');
        return h('div', { class: 'sim-dfoot' }, btn, status);
      },
    });
  }
  rbpDemonsMenu.addEventListener('toggle', (ev) => {
    if (ev.newState !== 'open') return;
    const inp = $('#rbp-demons-search input'); inp.value = ''; drawRbpDemons(); inp.focus();
  });
  $('#rbp-demons-search input').addEventListener('input', () => drawRbpDemons());
  $('#rbp-demons-search input').addEventListener('keydown', (ev) => { if (ev.key === 'Enter') { const b = $('#rbp-demons-list button'); if (b) b.click(); ev.preventDefault(); } else if (ev.key === 'ArrowDown') { const b = $('#rbp-demons-list button'); if (b) { b.focus(); ev.preventDefault(); } } });
  rbpDemonsMenu.addEventListener('keydown', (ev) => {
    if (ev.key !== 'ArrowDown' && ev.key !== 'ArrowUp') return;
    const rows = [...rbpDemonsMenu.querySelectorAll('.char')], i = rows.indexOf(document.activeElement);
    const next = rows[ev.key === 'ArrowDown' ? Math.min(rows.length - 1, i + 1) : Math.max(0, i - 1)];
    if (next) { next.focus(); ev.preventDefault(); }
  });
  $('#main').addEventListener('scroll', () => { if (rbpDemonsMenu.matches(':popover-open')) rbpDemonsMenu.hidePopover(); });
  /** The Build Planner's first page: "New build", then every saved build as a card. */
  function simHome(page) {
    const M = S.sim, builds = M.builds || [], n = simDraftCount();
    page.append(pageHead('Your builds. Open one to keep working on it, or start a new one. Nothing here touches the game or your saved snapshots.'));   // (no demon picker here: it is the editor's, and the cards follow what was picked there)
    page.append(h('div', { class: 'sim-builds' },
      // one card for both ways to start: the whole card is "New build", the small button in it takes a shared code
      h('div', { class: 'dg-tile sim-build sim-new', onclick: (ev) => { if (!ev.target.closest('.sim-new-import')) simOpen(null); } },
        h('button', { type: 'button', class: 'sim-new-main' },
          h('span', { class: 'sim-new-plus', 'aria-hidden': 'true' }, icon('plus')),
          h('span', { class: 'dgt-id' }, h('span', { class: 'dgt-name', text: 'New build' }),
            h('span', { class: 'dgt-line', text: n ? `Continue where you left off: ${n} change${n === 1 ? '' : 's'} not saved yet`
              : simDraft().base ? `Continue where you left off: ${simDraft().base} as imported, not saved yet` : 'Start empty, or import a character as its base' }))),
        h('button', { type: 'button', class: 'btn btn-sm btn-ghost sim-new-import', 'data-tip': 'Paste a code another player shared with you', onclick: () => simImportOpen() },
          h('span', { class: 'btn-icon', 'aria-hidden': 'true' }, icon('download')), 'Import build')),
      builds.map(simCard)));
    page.append(h('p', { class: 'note', text: builds.length
      ? 'Each card compares the build with its base as last read (digitalized when the build has a demon); one with no base shows what it gives by itself. A saved build keeps every change you make to it.'
      : 'No saved builds yet. Start a new one and press “Save build”: it then shows up here and under Build Planner in the left bar.' }));
    return page;
  }
  // What was picked before, per slot and part ("0:special" -> [{id, name}], newest first): shown as chips under each
  // dropdown, so a few candidates can be swapped back and forth with one click. Kept in the page's storage.
  const simHistKey = () => 'sim-hist';
  function simHist() {
    try { const v = JSON.parse(store.get(simHistKey()) || '{}'); return v && typeof v === 'object' ? v : {}; } catch (e) { return {}; }
  }
  function simHistAdd(k, part, id, name) {
    if (!id) return;
    const all = simHist(), key = `${k}:${part}`;
    if ((all[key] || []).some((x) => x.id === id)) return;   // already a chip: it stays where it is
    all[key] = [{ id, name }, ...(all[key] || [])].slice(0, 8);
    store.set(simHistKey(), JSON.stringify(all));
  }
  function simHistDrop(k, part, id) {
    const all = simHist(), key = `${k}:${part}`;
    all[key] = (all[key] || []).filter((x) => x.id !== id);
    if (!all[key].length) delete all[key];
    store.set(simHistKey(), JSON.stringify(all));
  }
  /** Asks the reader for the build's result. Only the answer to the latest change is used. */
  async function simRun(keep) {
    const M = S.sim, seq = ++M.seq;
    clearTimeout(M.timer);
    if (keep !== false) simSave();   // false: a build that was only opened, nothing to write back
    M.working = true;
    try {
      const res = await host.call('simulate', { ...simNow(), ...(await simDemonArgs(M.demon)), slots: M.build.slots, points: M.build.points });
      if (seq !== M.seq) return true;
      M.cur = res; M.working = false;
      if (M.redraw) M.redraw();
      return true;
    } catch (e) {
      if (seq !== M.seq) return true;
      M.working = false;
      if (M.redraw) M.redraw();
      toast('err', 'That build could not be worked out', String((e && e.message) || e));
      return false;
    }
  }
  /** The same, a moment later: while a slider is being dragged, one request per pause instead of one per step. */
  const simRunSoon = () => { const M = S.sim; clearTimeout(M.timer); M.working = true; M.timer = setTimeout(() => simRun(), 90); };

  async function simLoad() {
    const M = S.sim;
    if (M.state !== 'idle') return;
    M.state = 'loading';
    try {
      Object.assign(M, { inits: new Map(), bases: new Map(), cards: new Map() });   // everything known was against the last reading
      const builds = await host.call('simBuilds').catch(() => null);
      if (Array.isArray(builds)) M.builds = builds;
      // the build that was open (a saved one, or the unsaved one), on top of its base as read now
      let ok;
      try { ok = await simUse(M.view === 'edit' ? M.id : null); }
      catch (e) { if (!M.id || M.view !== 'edit') throw e; M.view = 'home'; toast('err', 'That build can’t be opened', String((e && e.message) || e)); ok = await simUse(null); }
      if (!ok && !M.id) { M.build = simClone(M.init.build); M.cap = null; M.cur = M.base; simSave(); }   // an unsaved build the data no longer has: start over
      M.state = 'ready';
    } catch (e) { M.state = 'failed'; M.error = String((e && e.message) || e); }
    drawSimNav();
    if (S.page === 'sim') render();
  }

  // --- reading a result ---
  /** Digitalized numbers are a result's own: it has them when its build has a demon (the base as read: the saved one). */
  const simDigi = (...results) => results.every((r) => r && r.digi);
  const simStatsOf = (res, d) => ((d === undefined ? simDigi(res) : d) ? res.digi : res.stats);
  /** A bonus row's value in the shown mode. Rows that are not added up (only in some fights) show their largest part. */
  const simRowVal = (r, d) => {
    if (!r) return 0;
    if (r.total === null || r.total === undefined) return round2(n0(d ? r.gear_digi : r.gear) + n0(r.expertise) + (d ? n0(r.demon) : 0));
    return n0(d ? r.total_digi : r.total);
  };
  const simCombatRow = (res, k) => {
    const c = COMBAT.find((x) => x[0] === k);
    return c ? res.bonuses.find((r) => r.group === 'Combat' && r.bonus === (BONUS_NAME[k] || c[1]) && (r.unit || '') === (c[2] || '')) : null;
  };
  /** Everything that differs between two results, in the shown mode: the 12 stats and HP/MP, then every bonus. */
  function simDiff(a, b, asShown) {
    // b digitalized: against a digitalized too where it has a demon (what another demon, or other gear, changes),
    // else against a as it is (what digitalizing adds). asShown: each in its own mode (the demon picker: a digitalized
    // build against the same one with no demon is what the demon takes away)
    const d = !!b.digi, da = asShown ? !!a.digi : d && !!a.digi, out = [], sa = simStatsOf(a, da), sb = simStatsOf(b, d);
    [['max_hp', 'Max HP'], ['max_mp', 'Max MP'], ...STATS].forEach(([k, label]) => {
      if (sa[k] === undefined || sb[k] === undefined || sa[k] === sb[k]) return;
      out.push({ group: 'Stats', key: k, label, unit: '', from: sa[k], to: sb[k] });
    });
    const key = (r) => r.row + '|' + (r.unit || '');
    const ma = new Map(a.bonuses.map((r) => [key(r), r])), mb = new Map(b.bonuses.map((r) => [key(r), r]));
    [...new Set([...mb.keys(), ...ma.keys()])].forEach((id) => {
      const ra = ma.get(id), rb = mb.get(id), r = rb || ra;
      if (r.group === 'Stats') return;   // the stat tiles show where these end up
      const from = simRowVal(ra, da), to = simRowVal(rb, d);
      if (from === to) return;
      out.push({ group: r.group, label: r.bonus, unit: r.unit || '', from, to, gone: !rb, fresh: !ra });
    });
    out.forEach((x) => { x.d = round2(x.to - x.from); x.good = lowIsGood(x.label) ? x.d < 0 : x.d > 0; x.size = Math.abs(x.d) / Math.max(Math.abs(x.from), Math.abs(x.to), 1); });
    return out;
  }
  /** A little dot before a name the change touches: green new, blue changed, red gone (down to nothing). */
  const chgDot = (kind) => h('span', { class: 'chg-dot ' + kind, role: 'img', 'aria-label': { new: 'new', upd: 'changed', gone: 'gone' }[kind],
    'data-tip': { new: 'New with this change', upd: 'Changed by this change', gone: 'Gone with this change' }[kind] });
  const simDelta = (x, cls) => h('span', { class: (cls || 'delta') + ' ' + (x.good ? 'good' : 'bad'), text: (x.d > 0 ? '▲ ' : '▼ ') + fmt(x.d, x.unit, true) });

  // --- showing what something gives ---
  /** One effect line of the reader: its text, and in a quieter voice when it applies or why it is not counted. */
  function simLine(l) {
    const why = l.c === 'digitalized' ? 'while digitalized' : l.c === 'situational' ? 'only in some fights' : l.c === 'inactive' ? 'not active' : l.c === 'unknown' ? 'not counted: the game data has no numbers for it' : '';
    const note = [why, l.n].filter(Boolean).join(', ');
    return h('li', { class: 'sim-line' + (l.c ? ' is-' + l.c : '') }, l.t, note ? h('small', { text: ` (${note})` }) : null);
  }
  const simLines = (list, none) => (list && list.length ? h('ul', { class: 'sim-lines' }, list.map(simLine)) : (none ? h('p', { class: 'sim-none', text: none }) : null));
  const simLineText = (l) => l.t + (l.c === 'digitalized' ? ' (digitalized)' : l.c === 'inactive' ? ' (not active)' : '');
  // The stats as the pieces' lines name them ("Strength +19"), with the names the page shows for them (STR, CLS-R, P.Def)
  // and short forms. Searching one of these in the piece picker finds the pieces that raise that stat itself, most
  // first: "mag" is the Magic stat, not Magic Damage; "cls-r" is Close-Range, not Close-Range Damage.
  const SIM_STAT_NAMES = [['Strength', 'str'], ['Magic', 'mag'], ['Vitality', 'vit'], ['Intelligence', 'int'], ['Speed', 'spd'], ['Luck', 'luk'],
    ['Close-Range', 'cls-r', 'cls', 'close range'], ['Long-Range', 'lng-r', 'lng', 'long range'], ['Spell'], ['Support', 'supp'],
    ['Physical Defense', 'p.def', 'phys def'], ['Magic Defense', 'm.def', 'mag def'], ['Max HP', 'hp'], ['Max MP', 'mp'], ['Critical', 'crit']];
  const simStatKey = (s) => s.toLowerCase().replace(/[^a-z0-9]+/g, '');
  const SIM_STAT_OF = new Map(SIM_STAT_NAMES.flatMap((names) => names.map((n) => [simStatKey(n), names[0]])));
  /** The stat a line gives ("Strength +19" or "Max HP +10%": "strength", "max hp"); null for any other line. */
  const simLineStat = (t) => { const m = /^(.+?)\s*[+−-]\s?\d+(?:\.\d+)?%?$/.exec(t); return m ? m[1].toLowerCase() : null; };

  function simApply(build, k, part, id) {
    const s = build.slots[k];
    if (part === 'whole' || !s) {
      build.slots[k] = id ? { id, basic: id, special: id, tarot: s ? s.tarot : 0, soul: s ? s.soul : 0, fuse: s ? s.fuse : [0, 0, 0], mods: s ? s.mods : [0, 0, 0, 0, 0] } : null;
      return;
    }
    const i = simModIdx(part);
    if (i >= 0) { const mods = [...(s.mods || [0, 0, 0, 0, 0])]; while (mods.length < 5) mods.push(0); mods[i] = id; s.mods = mods; return; }
    s[part] = id;
  }

  // --- the picker: what goes in one part of a piece ---
  const simDlg = $('#sim-pick');
  let simPickDone = null;   // tidies up after the open picker
  $('#sim-pick-close').addEventListener('click', () => simDlg.close());
  simDlg.addEventListener('click', (ev) => { if (ev.target === simDlg) simDlg.close(); });   // a click on the dimmed page around it
  simDlg.addEventListener('close', () => { if (simPickDone) simPickDone(); simPickDone = null; $('#sim-pick-preview').style.minHeight = ''; });
  /**
   * The pickers' preview strip while a choice is worked out (the reader takes about 50 ms a build): the label at once,
   * then grey chips where the numbers will come. The strip keeps the tallest height it has had while the dialog is open,
   * so moving from row to row never makes the dialog jump.
   */
  function simPreviewWait(preview, label) {
    preview.style.minHeight = Math.max(parseFloat(preview.style.minHeight) || 0, preview.offsetHeight) + 'px';
    preview.replaceChildren(h('span', { class: 'sim-preview-label', text: label }),
      ...[68, 76, 58, 62, 60, 56, 64, 58, 66].map((w) => h('span', { class: 'sim-chip sim-chip-wait', style: `width:${w}px`, 'aria-hidden': 'true' })));
  }

  // The picker's pictures: the Item Database's (gear-db.json "icon", files in gear-db-icons\ made from the player's own
  // game), by item id, loaded once. A crystal's is its own item's (catalog "item"); mods have none.
  let simPics = null;
  const simPicMap = () => simPics || (simPics = loadGearDb()
    .then((d) => new Map(((d && d.items) || []).filter((it) => it.id && it.icon && !it.keyItem).map((it) => [it.id, it.icon]))));

  async function simPick(k, part, after) {
    const M = S.sim, slotName = M.init.meta.slots[k];
    const list = $('#sim-pick-list'), tools = $('#sim-pick-tools'), preview = $('#sim-pick-preview');
    $('#sim-pick-title').textContent = `${slotName} · ${SIM_PARTS[part][0]}`;
    $('#sim-pick-sub').textContent = SIM_PARTS[part][1];
    tools.replaceChildren(); preview.replaceChildren();
    list.replaceChildren(h('p', { class: 'note', style: 'padding:12px', text: 'Loading what fits this slot…' }));
    if (!simDlg.open) simDlg.showModal();
    let alive = true, timer = null, look = 0;
    simPickDone = () => { alive = false; clearTimeout(timer); };

    let cat = M.catalog[k];
    if (!cat) {
      try { cat = M.catalog[k] = await host.call('simCatalog', { ...simNow(), slot: k }); }
      catch (e) { if (alive) list.replaceChildren(h('p', { class: 'note', style: 'padding:12px', text: String((e && e.message) || e) })); return; }
    }
    const pics = await simPicMap();
    if (!alive) return;

    const spec = M.build.slots[k], was = M.init.build.slots[k];
    const crystal = part === 'tarot' || part === 'soul', modIdx = simModIdx(part);
    const inUse = part === 'whole' ? (spec && spec.basic === spec.id && spec.special === spec.id ? spec.id : -1) : spec ? simPartVal(spec, part) : -1;
    const asRead = was ? (part === 'whole' ? -1 : simPartVal(was, part)) : -1;
    const linesOf = (it) => (part === 'basic' ? it.basic : part === 'special' ? it.special : part === 'id' ? it.look : [...it.basic, ...it.special, ...it.look]);
    let opts = modIdx >= 0
      ? ((cat.mods || [])[modIdx] || []).map((m) => ({ id: m.id, name: m.name, lines: m.lines, sets: [] }))
      : crystal
      ? cat[part].map((c) => ({ id: c.id, name: c.name, pic: pics.get(c.item), lines: c.lines, sets: c.sets }))
      : cat.items.map((it) => ({ id: it.id, name: it.name, pic: pics.get(it.id), tag: it.tag, sex: it.sex, lines: linesOf(it), sets: part === 'id' || part === 'whole' ? it.sets : [], own: (part === 'basic' || part === 'special') && spec && it.id === spec.id }));
    // a piece made for the other sex can't be worn: it is left out where the piece itself is picked
    const mySex = M.init.meta.sex, worn = part === 'id' || part === 'whole';
    if (mySex && worn) opts = opts.filter((o) => !o.sex || o.sex === mySex || o.id === (spec && spec.id));
    // Soul infusion as the server allows it (comp_hack EquipmentSpiritFuse; the flags come from the simulator's
    // `insert_spirit_fuse`): picking one of Slot 1, 2, 3 keeps the other two, and unless all three end up the same piece
    // they must all be pieces that can be soul infused (not Bullets, no "no fusion" flag), of one sub-category, and for
    // armor of at most one sex. Items without the flags (an older item list) are let through, as before.
    if (spec && (part === 'id' || part === 'basic' || part === 'special')) {
      const info = new Map(cat.items.map((it) => [it.id, it]));
      const armor = slotName !== 'Weapon';
      const fits = (c) => {
        const ids = ['id', 'basic', 'special'].map((p) => (p === part ? c : spec[p]));
        if (ids.every((x) => x === ids[0])) return true;
        const its = [...new Set(ids)].map((x) => info.get(x));
        if (its.some((x) => !x || x.si === undefined)) return true;
        if (its.some((x) => !x.si)) return false;
        if (new Set(its.map((x) => x.sub)).size > 1) return false;
        return !armor || new Set(its.map((x) => x.gender).filter((g) => g !== 2)).size <= 1;
      };
      const before = opts.length;
      opts = opts.filter((o) => o.id === inUse || fits(o.id));
      if (opts.length < before) $('#sim-pick-sub').textContent = `${SIM_PARTS[part][1]} Only pieces the game lets you soul infuse with the other slots are listed.`;
    }
    if (crystal) opts.unshift({ id: 0, name: part === 'tarot' ? 'No tarot' : 'No soul fusion', lines: [], sets: [], first: true });
    if (modIdx >= 0) opts.unshift({ id: 0, name: 'No mod', lines: [], sets: [], first: true });
    if (part === 'whole') opts.unshift({ id: 0, name: 'Nothing: leave this slot empty', lines: [], sets: [], first: true });
    opts.forEach((o) => {
      o.gives = o.lines.length + o.sets.length > 0;
      o.hay = [o.name, o.tag || '', ...o.lines.map((l) => l.t), ...o.sets.flatMap((s) => [...s.with, ...s.lines.map((l) => l.t)])].join(' ').toLowerCase();
      o.rank = o.id === inUse ? 0 : o.first ? 1 : o.own ? 2 : o.id === asRead ? 3 : 4;
    });
    opts.sort((a, b) => a.rank - b.rank || a.name.localeCompare(b.name));
    const withPics = opts.some((o) => o.pic);   // a picture column only where there are pictures (none for mods)

    const st = { q: '', kind: 'all', empty: false, shown: 60, active: 0, rows: [] };
    const kinds = [...new Set(opts.map((o) => o.tag).filter(Boolean))];
    const pass = (o) => o.rank < 4 || ((st.empty || o.gives) && (st.kind === 'all' || o.tag === st.kind));

    const choose = (o) => {
      simApply(M.build, k, part, o.id);
      simHistAdd(k, part, o.id, o.name);
      simDlg.close();
      M.open = k;
      simRun();
      if (after) after();
    };
    // What picking the highlighted row would do, before picking it: the build with that choice, against the build now.
    const showPreview = (o) => {
      clearTimeout(timer);
      if (!o) { preview.replaceChildren(); return; }
      if (o.id === inUse) { preview.replaceChildren(h('span', { class: 'sim-preview-label', text: 'In use now.' })); return; }
      simPreviewWait(preview, `With ${o.first ? 'that' : o.name}:`);
      const mine = ++look;
      timer = setTimeout(async () => {
        const cand = simClone(M.build);
        simApply(cand, k, part, o.id);
        let res;
        try { res = await host.call('simulate', { ...simNow(), ...(await simDemonArgs(M.demon)), slots: cand.slots, points: cand.points }); } catch (e) { res = null; }
        if (!alive || mine !== look) return;
        const head = h('span', { class: 'sim-preview-label', text: `With ${o.first ? 'that' : o.name}:` });
        if (!res) { preview.replaceChildren(head, h('span', { class: 'note', text: 'could not be worked out.' })); return; }
        const d = simDiff(M.cur, res).filter((x) => x.group !== 'Only in some fights' && x.group !== 'Partner');
        if (!d.length) { preview.replaceChildren(head, h('span', { class: 'note', text: 'no change to your numbers.' })); return; }
        const top = d.slice(0, 9);
        simFill(preview, head, top.map((x) => h('span', { class: 'sim-chip ' + (x.good ? 'good' : 'bad'), 'data-tip': `${x.label}: ${fmt(x.from, x.unit)} → ${fmt(x.to, x.unit)}` },
          h('span', { text: x.label }), h('b', { text: fmt(x.d, x.unit, true) }))), d.length > top.length ? h('span', { class: 'note', text: `and ${d.length - top.length} more` }) : null);
      }, 60);   // a short pause: moving across rows asks only for the one the pointer stops on
    };
    const setActive = (i, scroll) => {
      if (i === st.active && !scroll) return;
      st.active = Math.max(0, Math.min(st.rows.length - 1, i));
      list.querySelectorAll('.sim-opt').forEach((el) => { const on = Number(el.dataset.i) === st.active; el.setAttribute('aria-selected', String(on)); if (on && scroll) el.scrollIntoView({ block: 'nearest' }); });
      showPreview(st.rows[st.active]);
    };
    // Searching an effect ("rapid", "limit break power") puts the pieces that give the most of it first: the number on
    // the line `hit` picks (the lowest first where less is better: damage taken, cooldown). The piece's own lines, or
    // with `sets` its set bonuses' (which need the other pieces too, so they come after). Lines of the same effect add
    // up (two "Close-Range +12" / "+10" lines give 22); different effects (Limit Break Power, Limit Break Cap) do not.
    const amount = (o, hit, sets) => {
      const sum = new Map();
      (sets ? o.sets.flatMap((s) => s.lines) : o.lines).forEach((l) => {
        if (l.c === 'inactive' || l.c === 'unknown' || !hit(l.t)) return;
        const m = /([+−-])\s?(\d+(?:\.\d+)?)/.exec(l.t);
        if (!m) return;
        const v = Number(m[2]) * (m[1] === '+' ? 1 : -1) * (lowIsGood(l.t) ? -1 : 1);
        const key = l.t.replace(/[+−-]\s?\d+(?:\.\d+)?/, '#');
        sum.set(key, (sum.get(key) || 0) + v);
      });
      return sum.size ? Math.max(...sum.values()) : null;
    };
    const draw = () => {
      const words = st.q.trim().toLowerCase().split(/\s+/).filter(Boolean);
      // A stat's name (STR, CLS-R, P.Def, Strength ...): the pieces with a line of that stat, then any other piece whose
      // text has the words. Otherwise the lines that have every searched word.
      const stat = words.length ? SIM_STAT_OF.get(simStatKey(st.q)) : undefined;
      st.hit = !words.length ? null : stat ? (t) => simLineStat(t) === stat.toLowerCase() : (t) => { const s = t.toLowerCase(); return words.every((w) => s.includes(w)); };
      const score = new Map();   // piece -> [0 its own line, 1 a set bonus; the amount]
      if (st.hit) opts.forEach((o) => {
        if (!pass(o)) return;
        const own = amount(o, st.hit, false), set = own === null ? amount(o, st.hit, true) : null;
        if (own !== null) score.set(o, [0, own]); else if (set !== null) score.set(o, [1, set]);
      });
      st.rows = opts.filter((o) => pass(o) && (score.has(o) || words.every((w) => o.hay.includes(w))));
      st.byAmount = score.size > 0;
      if (st.byAmount) {
        const tier = (o) => (score.has(o) ? score.get(o)[0] : 2), val = (o) => (score.has(o) ? score.get(o)[1] : 0);
        st.rows.sort((a, b) => (a.rank < 4 ? a.rank : 4) - (b.rank < 4 ? b.rank : 4) || tier(a) - tier(b) || val(b) - val(a) || a.name.localeCompare(b.name));
      }
      sortNote.textContent = !st.byAmount ? '' : stat ? `Most ${stat} first` : 'Most of the searched effect first';
      st.active = Math.min(st.active, Math.max(0, st.rows.length - 1));
      const row = (o, i) => h('div', { class: 'sim-opt' + (withPics ? ' with-pic' : ''), role: 'option', 'aria-selected': String(i === st.active), 'data-i': i, onclick: () => choose(o), onpointermove: () => setActive(i) },
        withPics ? (o.pic ? h('img', { class: 'slot-icon sim-opt-pic', src: o.pic, alt: '', loading: 'lazy' }) : h('span', { class: 'slot-icon sim-opt-pic sim-noicon' })) : null,
        h('div', { class: 'sim-opt-body' }, h('div', { class: 'sim-opt-head' }, h('b', { text: o.name }),
          o.tag ? h('span', { class: 'tag', text: o.tag }) : null,
          o.sex ? h('span', { class: 'tag', text: `${o.sex} only` }) : null,
          o.id === inUse ? h('span', { class: 'tag sim-tag-use', text: 'In use' }) : null,
          o.own ? h('span', { class: 'tag', text: 'This piece’s own' }) : null,
          o.id === asRead && o.id !== inUse ? h('span', { class: 'tag', text: 'As last read' }) : null),
        o.lines.length ? h('div', { class: 'sim-opt-lines' }, o.lines.map((l, j) => [j ? ' · ' : null,
          h('span', { class: [l.c ? 'is-' + l.c : '', st.hit && st.hit(l.t) ? 'sim-hit' : ''].join(' ').trim() || null, text: simLineText(l) })])) : null,
        o.sets.map((s) => h('div', { class: 'sim-opt-lines sim-opt-set' }, h('span', { class: 'q-key', text: `${crystal ? 'Fusion set' : 'Set'} with ${s.with.join(', ')}: ` }), s.lines.map((l) => simLineText(l)).join(' · ') || 'nothing the game data has numbers for')),
        o.gives || o.first ? null : h('div', { class: 'sim-opt-lines zero', text: 'Gives nothing here' })));
      list.replaceChildren(...st.rows.slice(0, st.shown).map(row));
      if (!st.rows.length) list.append(h('p', { class: 'empty-row', text: words.length ? `Nothing matches “${st.q.trim()}”.` : 'Nothing fits here.' }));
      if (st.rows.length > st.shown) list.append(h('button', { type: 'button', class: 'btn btn-sm sim-more', onclick: () => { st.shown += 120; draw(); } }, `Show more (${st.rows.length - st.shown} left)`));
      showPreview(st.rows[st.active]);
    };
    const input = h('input', { class: 'input', type: 'search', spellcheck: 'false', 'aria-label': 'Search', placeholder: crystal ? 'Search a crystal or an effect (e.g. rapid)' : modIdx >= 0 ? 'Search a mod or an effect (e.g. lng-r)' : 'Search a name or an effect (e.g. limit break)',
      oninput: () => { st.q = input.value; st.shown = 60; st.active = 0; draw(); list.scrollTop = 0; } });
    input.addEventListener('keydown', (ev) => {
      if (ev.key === 'ArrowDown' || ev.key === 'ArrowUp') {
        ev.preventDefault();
        const next = st.active + (ev.key === 'ArrowDown' ? 1 : -1);
        if (next >= st.shown && st.rows.length > st.shown) { st.shown += 120; st.active = next; draw(); }
        setActive(next, true);
      } else if (ev.key === 'Enter') { ev.preventDefault(); if (st.rows[st.active]) choose(st.rows[st.active]); }
    });
    const sortNote = h('span', { class: 'note', role: 'status' });
    const hidden = opts.filter((o) => o.rank === 4 && !o.gives).length;
    simFill(tools, h('div', { class: 'search' }, icon('search'), input),
      kinds.length > 1 ? h('div', { class: 'seg', role: 'group', 'aria-label': 'Kind' }, [['all', 'All'], ...kinds.map((x) => [x, x])].map(([id, label]) =>
        h('button', { type: 'button', 'aria-pressed': String(st.kind === id), onclick: (ev) => { st.kind = id; st.shown = 60; st.active = 0; ev.currentTarget.parentNode.querySelectorAll('button').forEach((b) => b.setAttribute('aria-pressed', String(b === ev.currentTarget))); draw(); } }, label))) : null,
      hidden ? h('label', { class: 'gdb-check' }, h('input', { type: 'checkbox', onchange: (ev) => { st.empty = ev.target.checked; st.shown = 60; draw(); } }), `Also show the ${hidden} that give nothing here`) : null,
      sortNote);
    draw();
    input.focus();
  }

  /**
   * The expertise editor, shared by the Build Planner's Expertise tab and the Expertise page's planner: the points spent
   * against the cap (a cap set by hand only changes what counts as free or over, never a stat), one slider per expertise
   * with what its ranks unlock, and the chains worked out by the reader. `c` holds the build's points (`P`, changed in
   * place), the points as read (`P0`), the expertise list (`defs`), the game's cap (`cap0`, with `RP` for its make-up),
   * the cap in use and how to keep another (`cap()`, `setCap(v)`), the open rows (`open`), the chain ranks of the result
   * and of the character as read (`ranksNow()`, `ranksBefore()`) and what to do after a change (`onRank`, `onCap`).
   * `fold`: the expertises not trained yet sit behind a button. Returns the box and `live`, which refreshes the parts a
   * result changes (the sliders are left alone).
   */
  function expEditor(c) {
    const { defs, P, P0, cap0 } = c, RP = c.RP || {};
    const baseRank = (id) => Math.floor(n0(P0[id]) / 10000), rankOf = (id) => Math.floor(n0(P[id]) / 10000);
    const box = h('div', { class: 'sim-exp' });
    // The line is built once and only its texts are refreshed, so the box being typed in is never replaced under the cursor.
    const spent = h('span', { class: 'card-sub' }), freeTag = h('span', { class: 'tag' }), meter = h('span', { class: 'exp-bar' }, h('i')), capNote = h('p', { class: 'note' });
    const capIn = h('input', { class: 'input sim-cap-in', type: 'number', min: 0, step: 1000, inputmode: 'numeric', placeholder: 'no cap', 'aria-label': 'Expertise point cap, in pt' });
    const setCap = (v, typing) => {
      c.setCap(v === null || v === cap0 ? null : Math.max(0, Math.round(v)));
      drawBar(typing); if (c.onCap) c.onCap();
    };
    const capUndo = h('button', { type: 'button', class: 'btn btn-ghost btn-sm sim-undo', 'data-tip': cap0 ? `Back to your cap in game, ${simPt(cap0)}` : 'Back to no cap, as in game', onclick: () => setCap(null) }, 'Undo');
    const capStep = (by, text, tip) => h('button', { type: 'button', class: 'btn btn-ghost btn-square btn-sm', 'aria-label': tip, 'data-tip': tip, onclick: () => setCap(n0(c.cap()) + by) }, text);
    capIn.addEventListener('input', () => { if (capIn.value !== '') setCap(Number(capIn.value) * 100, true); });
    capIn.addEventListener('change', () => { if (capIn.value === '') setCap(null); else drawBar(); });   // an emptied box: back to the game's
    const capTip = (cap0 ? `Your cap in game is ${simPt(cap0)}` : 'The game has no cap on your points')
      + (cap0 && RP.level !== undefined && RP.extension !== undefined ? ` = 17,000.00pt + 1,000.00pt per 10 levels (level ${RP.level})${RP.level === 99 ? ' + 1,000.00pt at 99' : ''} + ${RP.extension} expansions × 1,000.00pt` : '')
      + '. Set another one to plan ahead: each expansion adds 1,000.00pt.';
    const bar = h('div', { class: 'sim-points' },
      h('div', { class: 'sim-points-top' }, h('b', { text: 'Expertise points' }), spent,
        h('span', { class: 'sim-cap' }, h('span', { class: 'card-sub', 'data-tip': capTip, text: 'of a cap of' }), capStep(-100000, '−', 'Cap 1,000.00pt lower'), capIn, h('span', { class: 'card-sub', text: 'pt' }),
          capStep(100000, '+', 'Cap 1,000.00pt higher (one more expansion)'), capUndo),
        freeTag),
      meter, capNote);
    const drawBar = (typing) => {
      const total = defs.filter((d) => d.counts).reduce((a, d) => a + n0(P[d.id]), 0), cap = c.cap();
      const free = cap ? cap - total : 0, moved = cap0 && cap ? cap - cap0 : 0;
      spent.textContent = simPt(total);
      if (!typing) capIn.value = cap ? String(cap / 100) : '';
      capUndo.hidden = cap === cap0;
      freeTag.hidden = !cap; freeTag.className = 'tag ' + (free < 0 ? 'tag-over' : '');
      freeTag.textContent = free < 0 ? `${simPt(-free)} over the cap` : `${simPt(free)} free`;
      meter.hidden = !cap; meter.className = 'exp-bar' + (free < 0 ? ' over' : '');
      meter.firstChild.style.width = `${cap ? Math.max(0, Math.min(100, (total / cap) * 100)) : 0}%`;
      capNote.textContent = [
        moved ? `A cap you set: ${simPt(Math.abs(moved))} ${moved > 0 ? 'above' : 'below'} yours in game (${simPt(cap0)})${moved > 0 && moved % 100000 === 0 ? `, which is ${moved / 100000} more expansion${moved === 100000 ? '' : 's'}` : ''}.` : (!cap0 && cap ? 'A cap you set: the game has none on your points.' : ''),
        free < 0 ? 'The ranks above need more points than this cap allows: lower an expertise or raise the cap.' : ''].filter(Boolean).join(' ');
      capNote.hidden = !capNote.textContent;
    };
    const unlockList = (d, rank) => h('ul', { class: 'sim-unlocks' }, d.unlocks.length ? d.unlocks.map((u) => h('li', { class: u.rank <= rank ? 'on' : '' },
      h('span', { class: 'sim-unlock-rank', text: simRank(u.rank) }),
      h('span', null, unlockSkills(u, d.name)))) : h('li', { text: 'The game data lists no skills for its ranks.' }));
    const row = (d) => {
      const base = baseRank(d.id), open = c.open.has(d.id);
      const label = h('span', { class: 'sim-exp-rank' }), was = h('span', { class: 'sim-exp-was' }), dot = h('span', { class: 'sim-exp-dot' });
      const slider = h('input', { type: 'range', class: 'sim-range', min: 0, max: d.max, step: 1, value: rankOf(d.id), 'aria-label': `${d.name}: rank` });
      const undo = h('button', { type: 'button', class: 'btn btn-ghost btn-sm sim-undo', onclick: () => set(base) }, 'Undo');
      const detail = h('div', { class: 'sim-exp-detail' });
      const el = h('div', { class: 'sim-exp-row' });
      const paint = () => {
        const v = rankOf(d.id);
        label.textContent = simRank(v);
        slider.value = v; slider.style.setProperty('--fill', `${d.max ? (v / d.max) * 100 : 0}%`);
        el.classList.toggle('changed', v !== base);
        dot.replaceChildren(...(v === base ? [] : [chgDot(!base ? 'new' : !v ? 'gone' : 'upd')]));
        was.textContent = v !== base ? `was ${Math.floor(base / 10)}-${base % 10}` : '';
        undo.hidden = v === base;
        if (c.open.has(d.id)) detail.replaceChildren(unlockList(d, v)); else detail.replaceChildren();
        drawBar();
      };
      const set = (v) => {
        v = Math.max(0, Math.min(d.max, v));
        P[d.id] = v === base ? n0(P0[d.id]) : v * 10000;   // untouched: the exact points as read (part of a rank counts toward chains)
        paint(); c.onRank();
      };
      slider.addEventListener('input', () => set(Number(slider.value)));
      const step = (by, text, tip) => h('button', { type: 'button', class: 'btn btn-ghost btn-square btn-sm', 'aria-label': tip, 'data-tip': tip, onclick: () => set(rankOf(d.id) + by) }, text);
      el.append(h('button', { type: 'button', class: 'sim-exp-name', 'aria-expanded': String(open), 'data-tip': 'What its ranks unlock',
          onclick: (ev) => { if (c.open.has(d.id)) c.open.delete(d.id); else c.open.add(d.id); ev.currentTarget.setAttribute('aria-expanded', String(c.open.has(d.id))); paint(); } },
          h('span', { class: 'chev' }, icon('chevron')), h('span', { text: d.name }), dot),
        h('span', { class: 'sim-exp-val' }, label, was),
        h('span', { class: 'sim-exp-ctl' }, step(-10, '«', 'One class down'), step(-1, '−', 'One rank down'), slider, step(1, '+', 'One rank up'), step(10, '»', 'One class up')),
        undo, detail);
      paint();
      return el;
    };
    const regular = defs.filter((d) => !d.chain), chains = defs.filter((d) => d.chain);
    const trained = regular.filter((d) => baseRank(d.id) > 0), rest = regular.filter((d) => baseRank(d.id) === 0);
    const chainBox = h('div', { class: 'sim-chains' });
    const drawChains = () => {
      const now = c.ranksNow(), before = c.ranksBefore();
      chainBox.replaceChildren(...chains.map((d) => {
        const a = n0(before[d.id]), b = n0(now[d.id]);
        if (!a && !b && !c.open.has('chains')) return null;
        const needs = d.needs.map((x) => `${Math.round(x.share * 100)}% of ${x.name}${x.rank ? ` (needs ${simRank(x.rank)})` : ''}`).join(' + ');
        return h('div', { class: 'sim-chain' + (a !== b ? ' changed' : '') + (b ? '' : ' off'), 'data-tip': needs ? `${d.name} = ${needs}` : null },
          h('span', null, d.name, a === b ? null : chgDot(!a ? 'new' : !b ? 'gone' : 'upd')),
          h('span', { class: 'sim-exp-val' }, h('span', { class: 'sim-exp-rank', text: b ? simRank(b) : 'Not reached' }), a && a !== b ? h('span', { class: 'sim-exp-was', text: `was ${Math.floor(a / 10)}-${a % 10}` }) : null));
      }).filter(Boolean));
      if (!chainBox.childNodes.length) chainBox.append(h('p', { class: 'note', text: 'No chain expertise reached.' }));
    };
    const allChains = h('button', { type: 'button', class: 'btn btn-ghost btn-sm', onclick: () => { if (c.open.has('chains')) c.open.delete('chains'); else c.open.add('chains'); allChains.textContent = c.open.has('chains') ? 'Only the ones reached' : 'Show every chain'; drawChains(); } },
      c.open.has('chains') ? 'Only the ones reached' : 'Show every chain');
    // not trained yet: on the Expertise page behind a button (the list is long), unless one of them was raised here
    const restBox = h('div');
    const drawRest = () => {
      const shown = !c.fold || c.open.has('rest') || rest.some((d) => rankOf(d.id) > 0);
      const btn = c.fold ? h('button', { type: 'button', class: 'btn btn-ghost btn-sm', onclick: () => { if (c.open.has('rest')) c.open.delete('rest'); else c.open.add('rest'); drawRest(); } },
        shown ? 'Hide them' : `Show all ${rest.length}`) : null;
      simFill(restBox, h('h3', { class: 'list-head sim-list-head' }, 'Not trained yet', h('small', { text: c.fold ? 'raise one to plan it' : '' }), btn), shown ? rest.map(row) : null);
    };
    simFill(box, bar,
      trained.length ? [h('h3', { class: 'list-head', text: 'Your expertise' }), trained.map(row)] : null,
      h('h3', { class: 'list-head sim-list-head' }, 'Chain expertise', h('small'), allChains), chainBox,
      rest.length ? restBox : null);
    if (rest.length) drawRest();
    drawChains();
    return { box, live: () => { drawBar(); drawChains(); } };
  }

  function pageSim() {
    const M = S.sim;
    const page = h('div', { class: 'page page-sim' });   // (no snapshot of the shown character needed: a build can stand on any character, or on none)
    simLoad();
    if (M.state !== 'ready') {
      page.append(pageHead(SIM_SUB));
      page.append(M.state === 'failed'
        ? h('div', { class: 'notice notice-warn' }, icon('alert'), h('div', null, h('b', { text: 'The Build Planner can’t start. ' }), M.error, ' ',
          h('button', { type: 'button', class: 'btn btn-sm', style: 'margin-left:8px', onclick: () => { M.state = 'idle'; render(); } }, 'Try again')))
        : h('p', { class: 'note', text: 'Reading your build…' }));
      return page;
    }
    if (M.view !== 'edit') return simHome(page);
    const meta = M.init.meta;
    const blank = !!meta.blank, mine = !!M.char && M.char === simActive();   // no base; the base is the character the app is showing
    // Under the title: the digitalize demon picker, in the Overview's format (it is the Overview's own band). There is
    // no character picker here: the build is the character's chosen at the top left of the app. A build with no base
    // (no character behind it) has its level and alignment there instead: it has no demon.
    function blankBand() {
      const lv = h('input', { class: 'input sim-cap-in sim-lv-in', type: 'number', min: 1, max: 99, step: 1, value: M.level, 'aria-label': 'Level', 'data-tip': 'The level that level-scaled effects are worked out for' });
      lv.addEventListener('change', () => simBlankSet(n0(lv.value) || M.level, M.align));
      return h('section', { class: 'card demon-card closed sim-band' }, h('div', { class: 'demon-pic none', 'aria-hidden': 'true' }, h('span', { text: '–' })),
        h('div', { class: 'demon-main' }, h('div', { class: 'demon-id' }, h('h3', { class: 'demon-name', text: 'No base' }),
          h('span', { class: 'card-sub', text: 'Level' }), lv,
          h('div', { class: 'seg', role: 'group', 'aria-label': 'Alignment' }, ['LAW', 'NEUTRAL', 'CHAOS'].map((a) => h('button', { type: 'button', 'aria-pressed': String(M.align === a),
            'data-tip': `Count the effects that need you to be ${alignName(a)}`, onclick: () => { if (M.align !== a) simBlankSet(M.level, a); } }, alignName(a)))),
          h('span', { class: 'tag tag-off', tabindex: 0, 'data-tip': 'No character behind the build: the numbers are only what its pieces and expertise ranks give', text: 'Only what the build gives' }))));
    }
    // The build's digitalize demon, in the format of the Overview's demon band (picture framed in its alignment, the
    // name as the dropdown, who it is, its skills). The dropdown lists the BASE's demons (the imported character's COMP
    // demons as last seen), whoever the app is showing, and the pick is the build's own.
    const bandBox = h('div', { class: 'sim-bands' });
    function demonBand() {
      if (blank) return blankBand();
      const entry = M.bases.get(simBaseKey(M.char, M.level, M.align)) || {}, plan = simIsPlan(M.demon);
      const list = plan ? entry.plans || null : entry.demons || null;   // a Demon Planner build is found among those, a COMP demon among the base's
      if (!list) (plan ? simPlanList() : simDemonList()).then(() => { if (bandBox.isConnected) simFill(bandBox, demonBand()); });
      const d = M.demon && list ? list.find((x) => x.key === M.demon) : null, O = d ? d.offer : null;
      const shown = M.demon ? (d ? d.name : plan ? 'A deleted demon build' : simDemonName(M.demon)) : 'No demon';
      const pic = h('div', { class: 'demon-pic ' + (M.demon ? alignClass(O && O.alignment) : 'none'), 'aria-hidden': 'true' }, h('span', { text: M.demon ? shown.charAt(0) : '–' }));
      if (d && d.icon) pic.append(h('img', { src: d.icon, alt: '' }));
      if (d && plan && d.mitama) mitamaMark(pic, d.mitama);
      const name = h('button', { type: 'button', class: 'name-btn', 'aria-haspopup': 'dialog',
        'data-tip': M.demon ? `This build is digitalized with ${plan && d ? `${d.demonName} as your Demon Planner build “${d.name}” ends` : shown}. Pick another of ${M.char}’s demons or of your demon builds, or no demon` : `Pick one of ${M.char}’s demons or of your demon builds to see this build digitalized with it`,
        onclick: () => simDemonPick() }, shown, h('span', { class: 'chev down' }, icon('chevron')));
      const chips = !M.demon ? [h('span', { class: 'tag tag-off', text: 'Not digitalized' })]
        : !d ? [h('span', { class: 'tag tag-off', text: !list ? (plan ? 'Reading your demon builds…' : 'Reading the demons…') : plan ? 'Not among your Demon Planner builds any more: not counted' : `Not among ${M.char}’s demons as last seen: not counted` })]
        : [plan ? h('span', { class: 'tag', 'data-tip': `Your Demon Planner build “${d.name}”, as the plan ends${d.planned ? '. The Mitamas it only plans count for the sets, not for their random bonus' : ''}`, tabindex: 0, text: d.demonName !== d.name ? `Demon Planner · ${d.demonName}` : 'Demon Planner' }) : null,
          h('span', { class: 'tag', text: `Lv ${fmt(d.level)}` }), d.race ? h('span', { class: 'tag', text: d.race }) : null,
          O && O.alignment ? h('span', { class: 'tag tag-align ' + alignClass(O.alignment), text: alignName(O.alignment) }) : null,
          d.mitama ? h('span', { class: 'tag', text: `${d.mitama} Mitama` }) : null,
          O ? h('span', { class: 'tag tag-demon', tabindex: 0, 'data-tip': `${M.char} gets ${fmt(O.rate)}% of its stats (at most ${fmt(O.maxRate)}%).`, text: `${fmt(O.rate)}% stats` }) : null];
      const skills = O ? (O.skills || []).filter((s) => s.kind !== 'Feature' && s.kind !== 'Equipment') : [];
      const icons = skills.length ? h('span', { class: 'dg-skill-icons' }, skills.map((s) => {
        const ic = skillIcon(s, entry.skillIcons);
        if (!isTrue(s.counted)) ic.classList.add('off');
        return withTip(ic, () => h('div', null, tipHead(skillIcon(s, entry.skillIcons), s.name, s.kind), ...tipLines(s.tip), skillWhy(s)));
      })) : null;
      return h('section', { class: 'card demon-card closed sim-band' }, pic, h('div', { class: 'demon-main' }, h('div', { class: 'demon-id' }, h('h3', { class: 'demon-name' }, name), chips, icons)));
    }
    const headLeft = h('div', { class: 'sim-title' }), headRight = h('div', { class: 'toolbar' }), editor = h('div', { class: 'sim-edit' }), out = h('aside', { class: 'sim-out' });
    // the editor's own top line: the tabs on the left, the reset on the right; under it the gear list or the expertise
    const tabsBox = h('div', { class: 'seg sim-tabs', role: 'group', 'aria-label': 'What to change' }), resetBox = h('div', { class: 'toolbar' }), bodyBox = h('div', { class: 'sim-edit-body' });
    editor.append(h('div', { class: 'sim-edit-top' }, tabsBox, resetBox), bodyBox);
    page.append(h('div', { class: 'page-head sim-head' }, headLeft, headRight), simFill(bandBox, demonBand()));
    // Some clients don't let BABEL read the worn pieces' own records: the build starts from the items alone.
    if (meta.gear_partial) page.append(h('div', { class: 'notice notice-warn' }, icon('alert'), h('div', null,
      h('b', { text: 'Your worn gear is missing some details. ' }),
      'BABEL couldn’t read the mods, the stats taken from another item and the fusion bonuses on your worn pieces, so they show without them. ',
      'Your totals still match the game; only changing those parts of a worn piece isn’t exact. Sending this through Send feedback with the diagnostics files helps fix it.')));
    page.append(h('div', { class: 'sim-layout' }, editor, out));

    // The build's name and what can be done with it. Drawn only when that changes (never by a result coming in), so
    // a name being typed is left alone.
    function drawTitle() {
      const back = h('button', { type: 'button', class: 'btn btn-ghost sim-back', 'data-tip': 'All your builds', onclick: simGoHome }, h('span', { class: 'chev' }, icon('chevron')), 'Builds');
      if (!M.id && !M.naming) {
        simFill(headLeft, back, h('b', { class: 'sim-name-text', text: 'New build' }), h('span', { class: 'card-sub', text: 'not saved' }),
          h('button', { type: 'button', class: 'btn btn-primary', 'data-tip': 'Keep this build: it gets a name, a card and a line under Build Planner', onclick: () => { M.naming = true; drawTitle(); } }, 'Save build…'),
          h('button', { type: 'button', class: 'btn btn-ghost btn-square', 'aria-label': 'Share this build', 'data-tip': 'Share: a code another player can import', onclick: () => simShare(simWhole(), '') }, icon('share')));
        return;
      }
      if (!M.id) {
        const input = h('input', { class: 'input sim-name', type: 'text', maxlength: 60, value: simNewName(), 'aria-label': 'Name of the build', spellcheck: 'false' });
        const save = async () => {
          const name = input.value.replace(/\s+/g, ' ').trim();
          if (!name) { input.focus(); return; }
          try { await simSaveDraft(name); } catch (e) { toast('err', 'The build could not be saved', String((e && e.message) || e)); return; }
          drawTitle(); drawHead();
          toast('ok', `Saved “${M.name}”`, 'It keeps every change you make to it from now on.', 4000);
        };
        input.addEventListener('keydown', (ev) => { if (ev.key === 'Enter') save(); else if (ev.key === 'Escape') { ev.stopPropagation(); M.naming = false; drawTitle(); } });
        simFill(headLeft, back, input, h('button', { type: 'button', class: 'btn btn-primary', onclick: save }, 'Save'),
          h('button', { type: 'button', class: 'btn btn-ghost', onclick: () => { M.naming = false; drawTitle(); } }, 'Cancel'));
        input.focus(); input.select();
        return;
      }
      const name = h('input', { class: 'input sim-name', type: 'text', maxlength: 60, value: M.name, 'aria-label': 'Name of the build', 'data-tip': 'Click to rename', spellcheck: 'false' });
      name.addEventListener('input', () => {
        const v = name.value.replace(/\s+/g, ' ').trim();
        if (!v) return;
        M.name = v;
        const b = M.builds.find((x) => x.id === M.id);
        if (b) b.name = v;
        simStore(); drawSimNav();
      });
      name.addEventListener('change', () => { if (!name.value.trim()) name.value = M.name; });
      name.addEventListener('keydown', (ev) => { if (ev.key === 'Enter') name.blur(); });
      M.statusEl = h('span', { class: 'card-sub', id: 'sim-status', role: 'status', 'data-tip': 'A saved build keeps every change as you make it' });
      simFill(headLeft, back, name, M.statusEl,
        M.confirmDel
          ? [h('span', { class: 'card-sub', text: 'Delete this build?' }), h('button', { type: 'button', class: 'btn btn-sm', onclick: () => simDelete(M.id) }, 'Delete'),
            h('button', { type: 'button', class: 'btn btn-ghost btn-sm', onclick: () => { M.confirmDel = false; drawTitle(); } }, 'Keep')]
          : [h('button', { type: 'button', class: 'btn btn-ghost btn-square', 'aria-label': 'Share this build', 'data-tip': 'Share: a code another player can import', onclick: () => simShare(simWhole(), M.name) }, icon('share')),
            h('button', { type: 'button', class: 'btn btn-ghost btn-square', 'aria-label': 'Duplicate this build', 'data-tip': 'Duplicate: a copy to change freely',
              // the whole build as saved (its base, level, alignment and demon too), so the copy stands where it does
              onclick: async () => { await simFlush(); simDuplicate({ name: M.name, ...simWhole() }, true); } }, icon('copy')),
            h('button', { type: 'button', class: 'btn btn-ghost btn-square', 'aria-label': 'Delete this build', 'data-tip': 'Delete this build', onclick: () => { M.confirmDel = true; drawTitle(); } }, icon('trash'))]);
      simStatus();
    }

    const change = () => { drawEditor(); drawHead(); simRun(); };
    // Reset to default: the character as last read (gear, expertise ranks and the point cap). What was there is kept
    // until the next change, so a reset by mistake can be taken back. The "picked before" chips stay.
    const resetAll = () => {
      const had = { build: M.build, cap: M.cap, demon: M.demon };
      Object.assign(M, { build: simClone(M.init.build), cap: null, demon: simDemonDefault(M.init), open: null });
      change(); M.before = had; drawHead(); simFill(bandBox, demonBand());
    };
    const unReset = async () => {
      const had = M.before;
      M.before = null;
      if (had.char !== undefined) {   // a base was imported: back to the base it had, with the build it had
        try { await simSetBase(had.char, had.level, had.align); } catch (e) { toast('err', 'That base can’t be read any more', String((e && e.message) || e)); return; }
        M.build = had.build; M.cap = had.cap; M.demon = had.demon;
        simRun(); render();
        return;
      }
      M.build = had.build; M.cap = had.cap; M.demon = had.demon; change(); simFill(bandBox, demonBand());
    };

    function drawHead() {
      const c = simChanges(), dirty = c.total > 0 || (M.cap !== null && M.cap !== undefined) || (!blank && M.demon !== simDemonDefault(M.init));
      if (dirty) M.before = null;   // something new was tried: the build from before the last reset is let go
      // the reset sits at the top right of the gear list (the user's place for it), beside the Gear / Expertise tabs
      // top right: what the build stands on, and the button that imports a character as its base
      simFill(headRight,
        h('span', { class: 'card-sub', text: blank ? 'No base' : `Base: ${M.char}` }),
        h('button', { type: 'button', class: 'btn sim-import', popovertarget: 'sim-bases', 'aria-haspopup': 'menu', 'data-fk': 'import',
          'data-tip': 'Start this build from one of your characters: its gear and expertise as last read. Nothing of the character is changed',
          onclick: (ev) => { simBasesFor = ev.currentTarget; } }, 'Import base', h('span', { class: 'chev down' }, icon('chevron'))));
      const imported = M.before && M.before.char !== undefined;
      simFill(resetBox,
        M.before && !dirty ? h('button', { type: 'button', class: 'btn btn-ghost', 'data-fk': 'unreset', 'data-tip': imported ? 'Back to the build you had before importing' : 'Put back what you had before the reset', onclick: unReset },
          imported ? 'Undo import' : 'Undo reset') : null,
        h('button', { type: 'button', class: 'btn', 'data-fk': 'reset', disabled: dirty ? null : true,
          'data-tip': blank ? 'Back to an empty build' : `Back to ${M.char} as last read: gear, expertise ranks, point cap and demon`, onclick: resetAll }, 'Reset to default'));
    }

    // --- gear ---
    // One box per worn piece, always open (no folding). Its first line is the piece: picture, slot, name and what can be
    // done with the whole piece. Under it the tarot and soul crystals side by side (where the slot takes them), then the
    // three soul infusion slots of the game's fusion window side by side (Slot 1 appearance and set bonuses, Slot 2
    // basic features, Slot 3 characteristics, which bullets don't have), then the mods.
    function slotCard(k) {
      const spec = M.build.slots[k], was = M.init.build.slots[k], res = M.cur.slots[k] || {}, slotName = meta.slots[k];
      const fresh = spec && res.id === spec.id;   // the result on screen is of this build (a moment after a change it is not yet)
      const changed = !simSame(spec, was);
      const name = spec ? (fresh ? res.name : '…') : 'Empty';
      const after = () => { drawEditor(); drawHead(); };
      const card = h('section', { class: 'sim-slot' + (changed ? ' changed' : '') + (spec ? '' : ' sim-slot-empty'), 'data-slot': k, 'aria-label': `${slotName}: ${name}` });
      // The chips of one part: what was picked for it before. A click uses that one again (Undo still goes back to
      // what you wear); the x takes the chip off the list. The one in use is marked.
      const hist = simHist();
      const histChips = (part) => {
        const list = hist[`${k}:${part}`] || [];
        if (!list.length) return null;
        const cur = !spec ? -1 : part === 'whole' ? (spec.basic === spec.id && spec.special === spec.id ? spec.id : -1) : simPartVal(spec, part);
        return h('div', { class: 'sim-hist', role: 'group', 'aria-label': 'Picked before' }, h('span', { class: 'sim-hist-label', text: 'Picked before' }),
          list.map((x) => h('span', { class: 'sim-hchip' + (x.id === cur ? ' on' : '') },
            h('button', { type: 'button', class: 'sim-hchip-use', 'aria-pressed': String(x.id === cur), 'data-fk': `h${k}${part}${x.id}`, 'data-tip': x.id === cur ? `${x.name}: in use` : `Use ${x.name}`,
              onclick: () => { if (x.id === cur) return; simApply(M.build, k, part, x.id); change(); } }, h('span', { text: x.name })),
            h('button', { type: 'button', class: 'sim-hchip-x', 'aria-label': `Remove ${x.name} from this list`, 'data-tip': 'Remove from this list',
              onclick: () => { simHistDrop(k, part, x.id); drawEditor(); } }, icon('x')))));
      };
      // the dropdown of one part: the picture and name of what is in it now
      const pickBtn = (part, text, pic) => h('button', { type: 'button', class: 'sim-pickbtn', 'data-fk': `p${k}${part}`, 'data-tip': `${text}. ${SIM_PARTS[part][1]}`, 'aria-label': `${slotName}, ${SIM_PARTS[part][0]}: ${text}`,
        onclick: () => simPick(k, part, after) },
        pic ? h('img', { class: 'sim-pick-icon', src: pic, alt: '' }) : null, h('span', { class: 'sim-pick-text', text }), h('span', { class: 'chev' }, icon('chevron')));

      // first line: the piece
      const acts = !spec
        ? [h('button', { type: 'button', class: 'btn btn-sm btn-primary', 'data-fk': `w${k}`, onclick: () => simPick(k, 'whole', after) }, 'Put a piece here…'),
          was ? h('button', { type: 'button', class: 'btn btn-sm', onclick: () => { M.build.slots[k] = simClone(was); change(); } }, 'Put it back') : null]
        : [h('button', { type: 'button', class: 'btn btn-sm', 'data-fk': `w${k}`, 'data-tip': SIM_PARTS.whole[1], onclick: () => simPick(k, 'whole', after) }, 'Replace piece…'),
          h('button', { type: 'button', class: 'btn btn-ghost btn-sm', 'data-tip': 'Wear nothing in this slot', onclick: () => { M.build.slots[k] = null; change(); } }, 'Take off'),
          changed && was ? h('button', { type: 'button', class: 'btn btn-ghost btn-sm', 'data-tip': 'Back to the piece as you wear it', onclick: () => { M.build.slots[k] = simClone(was); change(); } }, 'Undo') : null];
      card.append(h('div', { class: 'sim-slot-head' },
        spec && fresh && res.icon ? h('img', { class: 'slot-icon', src: res.icon, alt: '' }) : h('span', { class: 'slot-icon sim-noicon' }),
        h('span', { class: 'slot-name', text: slotName }), h('b', { class: 'sim-slot-item', text: name }),
        fresh && res.limit ? h('span', { class: 'tag sim-tag-limit', 'data-tip': 'Fusion bonus at MAX', text: 'LIMIT' }) : null,
        fresh && res.si === false ? h('span', { class: 'tag tag-off', tabindex: 0, 'data-tip': 'The game won’t soul infuse this piece: Slot 1, 2 and 3 stay its own', text: 'No soul infusion' }) : null,
        changed && !blank ? h('span', { class: 'tag sim-tag-changed', text: 'Changed' }) : null,
        h('span', { class: 'sim-slot-acts' }, acts), histChips('whole')));

      // second line: the parts, one column each
      if (!spec) {
        card.append(h('div', { class: 'sim-cols-none note', text: was ? 'Taken off in this build.' : 'Nothing is worn here.' }));
        return card;
      }
      const L = fresh ? res.lines : {};
      const moved = (part) => !!was && !simSame(simPartVal(spec, part), simPartVal(was, part));
      const undo = (part) => (moved(part)
        ? h('button', { type: 'button', class: 'btn btn-ghost btn-sm sim-undo', 'data-tip': 'Back to what you wear',
          onclick: () => { if (simModIdx(part) >= 0) simApply(M.build, k, part, simPartVal(was, part)); else spec[part] = simClone(was[part]); change(); } }, 'Undo') : null);
      const col = (title, sub, isMoved, ...kids) => h('div', { class: 'sim-col' + (isMoved ? ' changed' : '') },
        h('div', { class: 'sim-col-key' }, h('span', { text: title }), sub ? h('small', { text: sub }) : null), ...kids);
      // A piece the game won't soul infuse (Bullets, or the item's "no fusion" flag: `si` false) keeps its three slots
      // as they are: they show the piece without a dropdown (another piece is "Replace piece…").
      const noSI = fresh && res.si === false;
      const fixed = (text, pic) => h('div', { class: 'sim-pickbtn sim-pick-fixed', 'data-tip': `${text}. This piece can’t be soul infused, so this slot stays its own.` },
        pic ? h('img', { class: 'sim-pick-icon', src: pic, alt: '' }) : null, h('span', { class: 'sim-pick-text', text }));
      const slotPick = (key, value, pic) => (noSI && key !== 'tarot' && key !== 'soul'
        ? [h('div', { class: 'sim-part-pick' }, fixed(value, pic))]
        : [h('div', { class: 'sim-part-pick' }, pickBtn(key, value, pic), undo(key)), histChips(key)]);
      const part = (key, value, pic, lines, none) => [...slotPick(key, value, pic), fresh ? simLines(lines, none) : null];
      const own = res.icon || null;

      // Slot 1: the piece it counts as, so its appearance bonus and the sets it is in (by name, as the set list has them)
      const mySets = fresh ? M.cur.sets.filter((s) => s.kind === 'set' && s.name.split(', ').includes(simPlain(res.name))) : [];
      const setBox = mySets.length ? h('div', { class: 'sim-sets' }, mySets.map((s) => h('div', { class: 'sim-set' + (s.active ? '' : ' off') },
        h('span', { class: 'q-key', text: `Set ${s.worn} worn${s.active ? '' : ' (not complete)'}: ${s.name}` }), simLines(s.lines)))) : null;
      const slot1 = col('Slot 1', 'Appearance · set bonus', moved('id'), part('id', fresh ? res.name : '…', own, L.appearance, mySets.length ? null : 'No appearance bonus and no set.'), setBox);

      // Slot 2: basic features. The fusion bonus switch sits in its title line, on the right (LIMIT is every bonus of
      // this kind of piece at the top of its table, "MAX" in the game), and the bonus it gives is added to the basic
      // feature of the same stat, in brackets in the fusion colour: "Long-Range +82 (+45)".
      let limitBox = null;
      if (fresh && res.fuse_max && res.fuse_max.length) {
        const max = res.fuse_max.map((f) => `${f.stat} +${f.bonus}`).join(' · ');
        const now = (res.fuse || []).map((f) => `${f.stat} +${f.bonus}`).join(' · ');
        const sw = h('input', { type: 'checkbox', class: 'switch', role: 'switch', checked: res.limit ? true : null, 'data-fk': `f${k}`, 'aria-label': 'LIMIT: fusion bonus at MAX',
          onchange: () => { spec.fuse = sw.checked ? [50, 50, 50] : [0, 0, 0]; change(); } });
        limitBox = h('div', { class: 'sim-limit-box' },
          h('label', { class: 'sim-limit', 'data-tip': res.limit ? `Fusion bonus at MAX: ${max}` : now ? `Fusion bonus now ${now}. LIMIT gives ${max}` : `LIMIT: the fusion bonus at MAX gives ${max}` },
            h('b', { text: 'LIMIT' }), sw),
          undo('fuse'));
      }
      const itemLines = L.item || [], isFuse = (l) => l.n === 'fusion great success';
      const flat = (t) => /^(.*?)\s*([+−-])\s?(\d+(?:\.\d+)?)$/.exec(t);   // "Long-Range +82" -> name, sign, number (no %)
      const fuseBy = new Map();
      itemLines.filter(isFuse).forEach((l) => { const m = flat(l.t); if (m) fuseBy.set(m[1], (fuseBy.get(m[1]) || 0) + (m[2] === '+' ? 1 : -1) * Number(m[3])); });
      const fuseTag = (v) => h('span', { class: 'sim-fuse-add', 'data-tip': 'Fusion bonus', text: ` (${v < 0 ? '−' : '+'}${fmt(Math.abs(v))})` });
      const basicLines = itemLines.filter((l) => !isFuse(l));
      let basicBox;
      if (!fresh) basicBox = null;
      else if (!basicLines.length && !fuseBy.size) basicBox = h('p', { class: 'sim-none', text: 'No basic features.' });
      else {
        const left = new Map(fuseBy);
        basicBox = h('ul', { class: 'sim-lines' }, basicLines.map((l) => {
          const li = simLine(l), m = !l.c && flat(l.t);
          if (m && left.has(m[1])) { li.insertBefore(fuseTag(left.get(m[1])), li.childNodes[1] || null); left.delete(m[1]); }
          return li;
        }), [...left].map(([name, v]) => h('li', { class: 'sim-line' }, name, fuseTag(v))));   // a bonus to a stat the piece has no basic feature for
      }
      const slot2 = h('div', { class: 'sim-col' + (moved('basic') || moved('fuse') ? ' changed' : '') },
        h('div', { class: 'sim-col-top' }, h('div', { class: 'sim-col-key' }, h('span', { text: 'Slot 2' }), h('small', { text: 'Basic features' })), limitBox),
        slotPick('basic', fresh ? (res.basic || res.name) : '…', fresh ? (res.basic_icon || own) : null), basicBox);

      // Slot 3: characteristics (bullets have no third slot)
      const slot3 = slotName === 'Bullets'
        ? h('div', { class: 'sim-col off' }, h('div', { class: 'sim-col-key' }, h('span', { text: 'Slot 3' }), h('small', { text: 'Characteristics' })), h('p', { class: 'sim-none', text: 'Bullets have no Slot 3.' }))
        : col('Slot 3', 'Characteristics', moved('special'), part('special', fresh ? (res.special || res.name) : '…', fresh ? (res.special_icon || own) : null, L.characteristic, 'No characteristics.'));

      // above the slots: the tarot, then the soul, a line each, where the slot takes one (no line when it takes none):
      // its name and its dropdown, and under them what it gives
      const crystal = (key, label) => {
        const [pick, chips, lines] = part(key, fresh ? (res[key] || 'None') : '…', fresh && spec[key] ? res[key + '_icon'] : null, L[key], spec[key] ? 'Nothing the game data has numbers for.' : null);
        return h('div', { class: 'sim-col sim-crystal' + (moved(key) ? ' changed' : '') },
          h('div', { class: 'sim-col-key' }, h('span', { text: label }), h('small', { text: 'Crystal' })),
          h('div', { class: 'sim-crystal-pick' }, pick, chips), lines);
      };
      const crystals = [meta.tarot[k] ? crystal('tarot', 'Tarot') : null, meta.soul[k] ? crystal('soul', 'Soul') : null].filter(Boolean);

      // Under the slots: the mods. Only weapons have mod slots in NewMoon's item data: a weapon gets five columns, one per
      // mod slot, each with its dropdown (the mods the game lets you reach in that slot: simulator `weapon_mod_reach`)
      // and what the mod gives. Slots past the weapon's own count (`mod_slots`) are ones the game adds ("Added slot").
      // Another piece shows its mods only if it has any (as read), without dropdowns.
      let mods = null;
      if (slotName === 'Weapon') {
        const ownSlots = fresh && typeof res.mod_slots === 'number' ? res.mod_slots : null, each = (fresh && res.mods) || [];
        mods = h('div', { class: 'sim-col sim-mods' }, h('div', { class: 'sim-col-key' }, h('span', { text: 'Mods' }), h('small', { text: ownSlots === null ? 'five mod slots' : `${ownSlots} of its own; the game can add more, up to 5` })),
          h('div', { class: 'sim-mod-slots' }, [0, 1, 2, 3, 4].map((i) => {
            const key = 'mod' + i, v = simPartVal(spec, key), m = each[i];
            const name = !fresh ? '…' : v && v !== 255 ? (m && m.name) || 'A mod the game data has no name for' : 'No mod';
            return h('div', { class: 'sim-mod' + (moved(key) ? ' changed' : '') },
              // "Added" sits on the title's line, so every slot's dropdown stays level with the others
              h('div', { class: 'sim-col-key sim-mod-key' }, h('span', { text: `Slot ${i + 1}` }), ownSlots !== null && i >= ownSlots ? h('small', { class: 'sim-mod-added', 'data-tip': 'The weapon doesn’t come with this slot: the game adds it (a mod slot expansion)', tabindex: 0, text: 'Added' }) : null),
              h('div', { class: 'sim-part-pick' }, pickBtn(key, name, null), undo(key)), histChips(key),
              fresh && m ? simLines(m.lines, 'Nothing the game data has numbers for.') : null);
          })));
      } else if (fresh && L.mod && L.mod.length) {
        mods = h('div', { class: 'sim-col sim-mods' }, h('div', { class: 'sim-col-key' }, h('span', { text: 'Mods' }), h('small', { text: 'kept as they are' })), simLines(L.mod));
      }

      // (DOM append writes a null as the text "null": only the lines there are)
      card.append(...[crystals.length ? h('div', { class: 'sim-crystals' }, crystals) : null,
        h('div', { class: 'sim-cols' }, slot1, slot2, slot3), mods].filter(Boolean));
      return card;
    }

    // --- expertise ---
    let expLive = null;   // refreshes the parts of the expertise tab a result changes (the sliders are left alone)
    function expertiseTab() {
      const ed = expEditor({ defs: M.init.expertise, P: M.build.points, P0: M.init.build.points, cap0: M.base.expertise.cap,
        RP: (mine && S.data.ranks && S.data.ranks.points) || {}, open: M.expOpen,
        cap: () => (M.cap === null || M.cap === undefined ? M.base.expertise.cap : M.cap), setCap: (v) => { M.cap = v; simSave(); },
        ranksNow: () => M.cur.expertise.ranks, ranksBefore: () => M.base.expertise.ranks,
        onRank: () => { drawHead(); simRunSoon(); }, onCap: () => drawHead() });
      expLive = ed.live;
      return ed.box;
    }

    // The whole gear at a glance, and the way to pick the piece to edit: 15 small tiles in the game's equipment window order (its three rows of
    // five, which is also the slots' own order: Weapon, Bullets, Ring, Earring, Talisman / Head, Face, Top, Neck, Back /
    // Arms, COMP, Bottom, Feet, Extra). A tile shows the slot, the piece's picture and name (Slot 1), the pieces Slot 2
    // and Slot 3 come from (in the accent colour when soul infused from another piece), and marks for what is on it: the
    // tarot and soul crystals' pictures, LIMIT, the number of mods. Only the picked piece's box shows under the grid
    // (`M.gearSel`, the Weapon at first); a click on a tile shows that piece's box instead. Bullets are the exception
    // (the user's): there is nothing to edit on them but the bullet itself, so their tile opens the picker for another
    // bullet straight away and the box below stays as it is.
    const BULLETS = meta.slots.indexOf('Bullets');
    const gearSel = () => (Number.isInteger(M.gearSel) && M.gearSel >= 0 && M.gearSel < 15 && M.gearSel !== BULLETS ? M.gearSel : 0);
    function slotGrid() {
      const sel = gearSel();
      const tile = (k) => {
        const spec = M.build.slots[k], was = M.init.build.slots[k], res = M.cur.slots[k] || {}, slotName = meta.slots[k];
        const fresh = spec && res.id === spec.id, changed = !simSame(spec, was);
        const name = spec ? (fresh ? res.name : '…') : 'Empty';
        // Slot 2 and Slot 3: the piece each comes from (the piece itself unless soul infused; Bullets have no Slot 3)
        const donor = (n, from, icon, what) => h('span', { class: 'sim-grid-donor' + (from ? ' infused' : ''), 'data-tip': `Slot ${n}, ${what}: ${from || `${res.name} (its own)`}` },
          h('span', { class: 'sim-grid-n', text: String(n) }),
          (from ? icon : res.icon) ? h('img', { class: 'sim-grid-crystal', src: from ? icon : res.icon, alt: '' }) : null,
          h('span', { class: 'sim-grid-donor-name', text: from || res.name }));
        const donors = !fresh ? [] : [donor(2, res.basic, res.basic_icon, 'basic features'), slotName === 'Bullets' ? null : donor(3, res.special, res.special_icon, 'characteristics')];
        const modCount = spec ? (spec.mods || []).filter((m) => m && m !== 255).length : 0;
        const marks = !fresh ? [] : [
          spec.tarot && res.tarot_icon ? h('img', { class: 'sim-grid-crystal', src: res.tarot_icon, alt: '', 'data-tip': `Tarot: ${res.tarot}` }) : null,
          spec.soul && res.soul_icon ? h('img', { class: 'sim-grid-crystal', src: res.soul_icon, alt: '', 'data-tip': `Soul: ${res.soul}` }) : null,
          res.limit ? h('span', { class: 'sim-grid-mark limit', 'data-tip': 'Fusion bonus at MAX', text: 'LIMIT' }) : null,
          modCount ? h('span', { class: 'sim-grid-mark', 'data-tip': ((res.mods || []).filter(Boolean).map((m) => m.name).filter(Boolean).join(' · ')) || `${modCount} mods`, text: `${modCount} mod${modCount === 1 ? '' : 's'}` }) : null,
        ].filter(Boolean);
        return h('button', { type: 'button', class: 'sim-grid-tile' + (k === sel ? ' on' : '') + (spec ? '' : ' sim-grid-empty') + (changed && !blank ? ' changed' : ''), 'data-fk': 'g' + k,
          'aria-pressed': String(k === sel), 'aria-label': `${slotName}: ${name}. ${k === BULLETS ? 'Pick another bullet' : 'Edit it below'}`, 'data-tip': k === BULLETS ? 'Pick another bullet' : null,
          onclick: () => { if (k === BULLETS) simPick(k, 'whole', () => { drawEditor(); drawHead(); }); else if (k !== gearSel()) { M.gearSel = k; drawEditor(); } } },
          h('span', { class: 'sim-grid-slot', text: slotName }),
          h('span', { class: 'sim-grid-main' },
            spec && fresh && res.icon ? h('img', { class: 'slot-icon', src: res.icon, alt: '' }) : h('span', { class: 'slot-icon sim-noicon' }),
            h('span', { class: 'sim-grid-name', text: name })),
          donors.length ? h('span', { class: 'sim-grid-donors' }, donors) : null,
          marks.length ? h('span', { class: 'sim-grid-marks' }, marks) : null);
      };
      return h('div', { class: 'sim-grid', role: 'group', 'aria-label': 'Your gear at a glance' }, M.build.slots.map((_, k) => tile(k)));
    }

    // Where to get it: under the picked piece's box, how to get, in the game, each thing the piece is made of: the piece
    // in Slot 1, the pieces Slot 2 and Slot 3 come from, the tarot and soul crystals. The lines are the Item Database's
    // (`gdbWhere` on gear-db.json); an item used in several parts is listed once, with every part it is in. Mods are not
    // covered (the game data the app has doesn't say which item gives which mod).
    const WHERE_SHOWN = 6;   // lines per item before "Show all"
    function whereCard(k) {
      const spec = M.build.slots[k], res = M.cur.slots[k] || {};
      if (!spec || res.id !== spec.id) return null;
      const box = h('section', { class: 'card sim-where' });
      const want = new Map();   // item id -> the parts it is in
      const add = (id, part) => { if (id > 0) { if (!want.has(id)) want.set(id, []); want.get(id).push(part); } };
      add(spec.id, 'Slot 1'); add(spec.basic, 'Slot 2');
      if (meta.slots[k] !== 'Bullets') add(spec.special, 'Slot 3');
      add(spec.tarot ? res.tarot_item : 0, 'Tarot'); add(spec.soul ? res.soul_item : 0, 'Soul');
      const head = h('div', { class: 'card-head' }, h('h2', { class: 'section-title', text: 'Where to get it' }),
        h('span', { class: 'card-sub', text: `In the game, for ${res.name}: its slots and crystals` }));
      const fill = (d) => {
        const byId = d ? (S.gdb.byId || (S.gdb.byId = new Map(d.items.filter((it) => it.id && !it.keyItem).map((it) => [it.id, it])))) : null;
        if (!byId) { simFill(box, head, h('p', { class: 'note', text: 'The Item Database’s data can’t be read, so where things come from can’t be shown.' })); return; }
        M.whereOpen = M.whereOpen || new Set();
        const items = [...want].map(([id, parts]) => {
          const it = byId.get(id);
          const label = h('span', { class: 'sim-where-parts' }, parts.map((p) => h('span', { class: 'tag', text: p })));
          if (!it) return h('div', { class: 'sim-where-item' }, h('div', { class: 'sim-where-head' }, h('span', { class: 'slot-icon sim-noicon' }), h('b', { text: 'An item the Item Database doesn’t list' }), label));
          const { where, notes } = gdbWhere(it);
          const all = M.whereOpen.has(id), more = where.length - WHERE_SHOWN;
          return h('div', { class: 'sim-where-item' },
            h('div', { class: 'sim-where-head' }, it.icon ? h('img', { class: 'slot-icon', src: it.icon, alt: '' }) : h('span', { class: 'slot-icon sim-noicon' }),
              h('b', { text: it.name }), label,
              h('button', { type: 'button', class: 'btn btn-ghost btn-sm sim-where-open', 'data-tip': 'Open this item in the Item Database',
                onclick: () => { Object.assign(S.gdb, { q: it.name, kind: 'all', cls: 'all', known: false, gender: '', align: '', src: [], range: '', aff: [], maxLv: '', open: new Set([it.key]), pg: 0 }); go('geardb'); } }, 'Item Database')),
            where.length ? h('ul', { class: 'gdb-list gdb-get' }, all ? where : where.slice(0, WHERE_SHOWN)) : null,
            more > 0 ? h('button', { type: 'button', class: 'btn btn-ghost btn-sm', 'aria-expanded': String(all),
              onclick: () => { if (all) M.whereOpen.delete(id); else M.whereOpen.add(id); fill(d); } }, all ? 'Show fewer' : `Show all ${where.length} ways`) : null,
            notes.length ? h('div', { class: 'sim-where-notes' }, h('span', { class: 'q-key', text: 'Community notes' }), h('ul', { class: 'gdb-list' }, notes.map((n) => h('li', { text: n })))) : null,
            !where.length && !notes.length ? h('p', { class: 'note', text: 'Neither the game data nor the community notes say where this comes from.' }) : null);
        });
        simFill(box, head, items);
      };
      if (S.gdb.state === 'ready') fill(S.gdb.data);
      else { simFill(box, head, h('p', { class: 'note', text: 'Reading the Item Database…' })); loadGearDb().then(fill); }
      return box;
    }

    function drawEditor() {
      const keep = document.activeElement && editor.contains(document.activeElement) ? document.activeElement.dataset.fk : null;
      const c = simChanges();
      const tab = (id, label, n) => h('button', { type: 'button', 'aria-pressed': String(M.tab === id), onclick: () => { M.tab = id; drawEditor(); } }, label, n ? h('span', { class: 'sim-count', text: n }) : null);
      expLive = null;
      simFill(tabsBox, tab('gear', 'Gear', c.slots.length), tab('expertise', 'Expertise', c.ranks.length));
      simFill(bodyBox, M.tab === 'gear' ? h('div', { class: 'sim-slots' }, slotGrid(), slotCard(gearSel()), whereCard(gearSel())) : expertiseTab());
      if (keep) { const el = editor.querySelector(`[data-fk="${keep}"]`); if (el) el.focus({ preventScroll: true }); }
    }

    // --- results ---
    function drawResults() {
      const base = M.base, cur = M.cur, diff = simDiff(base, cur), c = simChanges();
      const D = !!cur.digi, Db = D && !!base.digi, st = simStatsOf(cur, D);   // digitalized: the build has a demon; the base's side too when it has one saved
      const byKey = new Map(diff.filter((x) => x.group === 'Stats').map((x) => [x.key, x]));
      const tile = ([k, label]) => {
        if (st[k] === undefined) return null;
        const x = byKey.get(k);
        // without a base a tile is what the build adds to that stat, tinted by its sign
        if (blank) return h('div', { class: 'sim-stat' + (st[k] ? ' changed ' + (st[k] > 0 ? 'up' : 'down') : ''), 'data-tip': `${label}: what the build adds` },
          h('span', { class: 'sim-stat-label', text: label }), h('b', { class: 'sim-stat-value', text: fmt(st[k], '', true) }));
        return h('div', { class: 'sim-stat' + (x ? ' changed ' + (x.good ? 'up' : 'down') : ''), 'data-tip': x ? `${label}: ${fmt(x.from)} → ${fmt(x.to)}` : `${label} ${fmt(st[k])}: as you are now` },
          h('span', { class: 'sim-stat-label', text: label }), h('b', { class: 'sim-stat-value', text: fmt(st[k]) }), x ? simDelta(x, 'sim-stat-delta') : h('span', { class: 'sim-stat-delta same', text: '—' }));
      };

      const keyRow = ([k, label]) => {
        const cdef = COMBAT.find((x) => x[0] === k) || OTHER_STATS.find((x) => x[0] === k), unit = cdef[2];   // a combat stat, or one of the Other group (EXP, costs...)

        const to = simRowVal(bonusRowOf(k, cdef[1], unit, cur.bonuses), D), from = simRowVal(bonusRowOf(k, cdef[1], unit, base.bonuses), Db), d = round2(to - from);
        const x = { d, unit, good: lowIsGood(cdef[1]) ? d < 0 : d > 0 };
        return h('div', { class: 'sim-key' + (d ? ' changed' : ''), 'data-tip': d ? `${cdef[1]}: ${fmt(from, unit)} → ${fmt(to, unit)}` : null },
          h('span', { class: 'sheet-label', text: label }), capTag(to, CAPS[k] === undefined ? null : CAPS[k], unit),
          h('b', { class: 'sheet-value' + (to < 0 && !lowIsGood(cdef[1]) ? ' bad' : ''), text: fmt(to, unit) }), d ? simDelta(x) : h('span', { class: 'delta same' }));
      };
      const sheet = SHEET.flat().map(([title, rows]) => h('div', { class: 'sheet-group' }, h('h3', { class: 'list-head', text: title }), rows.map(keyRow)));
      // every difference, in one list at the top: your stats, then the bonuses group by group
      const groups = [['Stats', blank ? 'Stats' : mine ? 'Your stats' : `${M.char}’s stats`], ['Combat', 'Combat'], ['Affinity', 'Affinity'], ['Other', 'Other'], ['Partner', blank ? 'A partner' : 'Your partner'], ['Only in some fights', 'Only in some fights']];
      const allRows = groups.flatMap(([g]) => diff.filter((x) => x.group === g));
      const few = M.allDiff || allRows.length <= 12;
      const diffList = h('div', { class: 'sim-others' });
      groups.forEach(([g, title]) => {
        const rows = (few ? allRows : allRows.slice(0, 10)).filter((x) => x.group === g);
        if (!rows.length) return;
        // without a base there is no "before": a line is what the build gives
        diffList.append(h('div', { class: 'chg-group', text: title }), ...rows.map((x) => (blank
          ? h('div', { class: 'chg' }, h('span', { text: x.label }), h('span'), h('span', { class: 'delta ' + (x.good ? 'good' : 'bad'), text: fmt(x.to, x.unit, true) }))
          : h('div', { class: 'chg' }, h('span', null, x.label, x.fresh ? chgDot('new') : x.gone ? chgDot('gone') : null),
            h('span', { class: 'chg-vals' }, x.fresh ? null : fmt(x.from, x.unit) + ' → ', h('b', { text: x.gone ? 'gone' : fmt(x.to, x.unit) })), simDelta(x)))));
      });
      if (allRows.length > 12) diffList.append(h('button', { type: 'button', class: 'btn btn-sm chg-more', style: 'margin-top:8px', 'aria-expanded': String(few),
        onclick: () => { M.allDiff = !M.allDiff; drawResults(); } }, few ? 'Show fewer' : `Show all ${allRows.length} differences`));
      // sets won and lost
      const setsOf = (res) => new Map(res.sets.filter((s) => s.active).map((s) => [s.kind + '|' + s.name, s]));
      const sa = setsOf(base), sb = setsOf(cur);
      const setNote = (s, text, cls) => h('div', { class: 'sim-setnote ' + cls }, h('b', { text }), `${s.kind === 'fusion' ? 'fusion set ' : 'set '}${s.name}`);
      const setNotes = [...[...sa.keys()].filter((key) => !sb.has(key)).map((key) => setNote(sa.get(key), 'Lost: ', 'bad')), ...[...sb.keys()].filter((key) => !sa.has(key)).map((key) => setNote(sb.get(key), 'Gained: ', 'good'))];
      // effects of changed pieces the data has no numbers for
      const blind = c.slots.filter((k) => cur.slots[k] && cur.slots[k].lines && Object.values(cur.slots[k].lines).some((list) => list.some((l) => l.c === 'unknown'))).map((k) => cur.slots[k].name);
      // (with no gear or expertise change, another demon than the saved one is still a difference: the title says which)
      const otherDemon = !blank && M.demon !== simDemonDefault(M.init);   // not the one a build starts with
      const title = blank ? 'What this build gives' : c.total ? `With your ${c.total} change${c.total === 1 ? '' : 's'}`
        : otherDemon ? (M.demon ? `Digitalized with ${cur.demon || simDemonName(M.demon)}` : 'Not digitalized') : `As ${mine ? 'you are' : M.char + ' is'} now`;
      simFill(out,
        h('section', { class: 'card sim-card' },
          h('div', { class: 'card-head' }, h('h2', { class: 'section-title', text: title }), h('span', { class: 'card-sub', role: 'status', text: M.working ? 'working it out…' : blank ? 'no base' : D ? `digitalized with ${cur.demon || simDemonName(M.demon)}` : 'not digitalized' })),
          !c.total && !diff.length ? h('p', { class: 'note', text: blank ? 'Put pieces in the slots and raise expertise ranks on the left: what they give together is listed here. Or press “Import base” at the top right to start from one of your characters.'
            : `Change a piece or an expertise rank on the left: what it does to ${mine ? 'your' : M.char + '’s'} numbers is listed here.` }) : null,
          c.total && !diff.length && !M.working ? h('p', { class: 'note', text: 'These changes move none of your numbers.' }) : null,
          setNotes.length ? h('div', { class: 'sim-setnotes' }, setNotes) : null,
          allRows.length ? diffList : null),
        h('section', { class: 'card sim-card' }, h('div', { class: 'sim-stats' + (st.max_hp === undefined ? ' no-hp' : '') }, [['max_hp', 'Max HP'], ['max_mp', 'Max MP'], ...STATS].map(tile))),
        h('section', { class: 'card sim-card sheet' }, sheet),
        blind.length ? h('div', { class: 'notice notice-warn' }, icon('alert'), h('div', null, h('b', { text: 'Not everything is counted. ' }), `The game data has no numbers for some effects of ${blind.join(', ')}. They are listed on the piece, and left out of the totals.`)) : null,
        h('p', { class: 'note', text: blank
          ? `No base: these are the bonuses of the pieces and expertise ranks alone, for level ${meta.level} and ${alignName(meta.alignment)}. There is no character behind them, so no base stats, no Max HP or MP, no learned skills and no demon.`
          : `Stats start from the game’s own numbers for ${M.char} (read ${when(meta.stats_at)}, level ${meta.level}) and move by what the change adds. As everywhere in the app: no buffs and nothing from the partner demon.` }));
    }

    M.redraw = () => {
      if (S.page !== 'sim' || !page.isConnected) return;
      drawHead();
      if (M.tab === 'gear') drawEditor(); else if (expLive) expLive();
      drawResults();
    };
    drawTitle(); drawHead(); drawEditor(); drawResults();
    return page;
  }

  // ---------- Tools: Digitalize Planner ----------
  // digi-planner.json is made by "imagine-reader digiplanner" (crates\imagine\src\build\digi_planner.rs) from the server's
  // Digitalization tables. The rules followed here are the server's (comp_hack CharacterManager::UpdateDigitalizePoints and
  // the digitalize check in SkillManager):
  //  - points go to the race of the demon you are digitalized with, so a race can only be raised with a demon of that race
  //    you may digitalize: one a level of that race names (at or below the level you have), or one a special unlock
  //    lets in (any one race:level pair of GuardianSpecialData);
  //  - a race is enabled at level 1 as soon as every race:level pair of its unlock row is reached;
  //  - levels stop at 10, and at 5 while the demon is not Mitama-fused.
  function loadDigiPlan() {
    if (S.dpl.state !== 'idle') return;
    S.dpl.state = 'loading';
    fetch('digi-planner.json', { cache: 'no-store' }).then((r) => (r.ok ? r.json() : null)).catch(() => null).then((d) => {
      S.dpl.eng = d ? digiEngine(d) : null;
      S.dpl.state = d ? 'ready' : 'failed';
      if (S.page === 'dplan') render();
    });
  }

  /** The planner over the exported tables: start states, the cheapest plan for a set of goals, and its steps. */
  function digiEngine(P) {
    const races = new Map(), demons = new Map();
    P.races.forEach((r) => {
      // cum[L]: points from level 1 to level L
      r.cum = [0, 0]; r.xp.forEach((x, i) => { r.cum[i + 2] = r.cum[i + 1] + x; });
      r.gated = r.demons.map(([id, name, level, base]) => ({ id, name, level, base }));
      races.set(r.id, r);
      r.gated.forEach((d) => { if (!demons.has(d.id)) demons.set(d.id, { id: d.id, name: d.name, race: r.id, base: d.base, gate: d.level, opts: [] }); });
    });
    P.special.forEach(([id, name, race, base, opts]) => {
      const d = demons.get(id) || { id, name, race, base, gate: null, opts: [] };
      d.opts = opts.filter(([r]) => races.has(r));
      demons.set(id, d);
    });
    const raceName = (id) => (races.get(id) || {}).name || `race ${id}`;
    const ofRace = new Map();
    demons.forEach((d) => { if (!ofRace.has(d.race)) ofRace.set(d.race, []); ofRace.get(d.race).push(d); });
    // a demon's ways in: its own race at the level that names it, or any special pair
    const waysIn = (d) => {
      const w = d.gate ? [[d.race, d.gate]] : [];
      d.opts.forEach(([r, l]) => { if (!w.some(([a, b]) => a === r && b <= l)) w.push([r, l]); });
      return w;
    };

    /** A start: { lv, pts } per race. From the game's levels (null: from scratch: the races with no unlock row at 1). */
    function start(read) {
      const lv = new Map(), pts = new Map();
      const got = new Map((read || []).map((e) => [n0(e.race_id), e]));
      races.forEach((r) => {
        const e = got.get(r.id);
        lv.set(r.id, e ? n0(e.level) : r.unlock.length ? 0 : 1);
        pts.set(r.id, e ? n0(e.points) : 0);
        if (!r.unlock.length && lv.get(r.id) < 1) lv.set(r.id, 1);   // enabled from the start
      });
      return { lv, pts };
    }
    const reached = (lv, [r, l]) => (lv.get(r) || 0) >= l;
    /** Races whose unlock pairs are all reached but that are still at 0 get level 1, as the server does. */
    function unlockAll(lv) {
      const opened = [];
      for (let again = true; again;) {
        again = false;
        races.forEach((r) => { if (!lv.get(r.id) && r.unlock.length && r.unlock.every((p) => reached(lv, p))) { lv.set(r.id, 1); opened.push(r.id); again = true; } });
      }
      return opened;
    }
    /** Points to take a race from where the start has it to level `to`. */
    function cost(st, id, to) {
      const r = races.get(id), cur = st.lv.get(id) || 0;
      if (!r || to <= Math.max(cur, 1)) return 0;
      return Math.max(0, r.cum[to] - r.cum[Math.max(cur, 1)] - (cur >= 1 ? st.pts.get(id) || 0 : 0));
    }
    /** The demons you could raise a race with at level `at` of it, with `lv` for the other races. */
    const trainers = (id, at, lv) => (ofRace.get(id) || []).filter((d) => (d.gate && d.gate <= at) || d.opts.some(([r, l]) => (r === id ? at : lv.get(r) || 0) >= l));
    /** What a demon needs to be digitalized, with `lv`: true when it already may be. */
    const canUse = (d, lv) => waysIn(d).some((p) => reached(lv, p));

    /**
     * The cheapest plan for the goals ({ k: 'race', race, level } | { k: 'demon', id }) from a start: the level each race
     * must reach. All unlock pairs of a race needed are needed too; a race that has to earn points needs a demon of it
     * you may digitalize, and a demon goal needs one of its ways in: those two are choices, tried cheapest first.
     */
    function solve(st, goals) {
      const base = new Map(), groups = [], bad = [];
      goals.forEach((g) => {
        if (g.k === 'race' && races.has(g.race)) base.set(g.race, Math.max(base.get(g.race) || 0, g.level));
        if (g.k === 'demon') {
          const d = demons.get(g.id);
          if (!d) return;
          const w = waysIn(d);
          if (!w.length) bad.push(`No level names ${d.name}, and it has no special unlock.`);
          else groups.push({ why: d.name, options: w });
        }
      });
      const eff = (need, r) => Math.max(st.lv.get(r) || 0, need.get(r) || 0);
      const close = (req) => {
        const need = new Map(req);
        for (let again = true; again;) {
          again = false;
          need.forEach((l, r) => {
            const R = races.get(r);
            if (!R || l < 1 || (st.lv.get(r) || 0) >= 1) return;
            R.unlock.forEach(([u, ul]) => { if ((need.get(u) || 0) < ul) { need.set(u, ul); again = true; } });
          });
        }
        return need;
      };
      const total = (need) => { let c = 0; need.forEach((l, r) => { c += cost(st, r, l); }); return c; };
      // a race that must earn points: the ways to have a demon of it to digitalize (null: one is there already)
      const trainerGroup = (need, r) => {
        const at = Math.max(st.lv.get(r) || 0, 1);
        if ((ofRace.get(r) || []).some((d) => d.gate && d.gate <= at)) return null;
        const options = [];
        (ofRace.get(r) || []).forEach((d) => d.opts.forEach(([a, l]) => {
          if (a === r ? l > at : false) return;   // its own race at a level it hasn't got yet: no help
          if (a === r) { options.length = 0; options.push(null); return; }
          if (!options.some((o) => o && o[0] === a && o[1] <= l)) options.push([a, l]);
        }));
        if (options.includes(null)) return null;
        return { why: `a demon to raise ${raceName(r)} with`, race: r, options };
      };
      let best = null, fail = null;
      const seen = new Set();
      let budget = 20000;   // the search is small (a few choices with 2-4 ways each); this only guards a runaway
      const rec = (req) => {
        if (--budget < 0) return;
        const need = close(req), c = total(need);
        if (best && c >= best.cost) return;
        const key = [...need].filter(([r, l]) => l > (st.lv.get(r) || 0)).sort((a, b) => a[0] - b[0]).map((p) => p.join(':')).join(',');
        if (seen.has(key)) return;
        seen.add(key);
        const all = groups.slice();
        need.forEach((l, r) => { if (l > Math.max(st.lv.get(r) || 0, 1)) { const g = trainerGroup(need, r); if (g) all.push(g); } });
        const open = all.find((g) => !g.options.some(([r, l]) => eff(need, r) >= l));
        if (!open) { best = { need, cost: c }; return; }
        if (!open.options.length) { fail = fail || `There is no way to get ${open.why}.`; return; }
        open.options
          .map((o) => { const n2 = new Map(req); n2.set(o[0], Math.max(n2.get(o[0]) || 0, o[1])); return { n2, c: total(close(n2)) }; })
          .sort((a, b) => a.c - b.c)
          .forEach(({ n2 }) => rec(n2));
      };
      if (!bad.length) rec(base);
      if (!best) return { ok: false, why: bad[0] || fail || 'No plan found.' };
      return { ok: true, need: best.need, cost: best.cost, steps: steps(st, best.need) };
    }

    /**
     * The plan as steps in an order that works: a race is raised only once it is enabled and you have a demon of it to
     * digitalize. Each step says what it opens: races enabled, demons you may digitalize, passives learned, assists offered.
     */
    function steps(st, need) {
      const lv = new Map(st.lv), out = [];
      let todo = [...need].filter(([r, l]) => l > Math.max(st.lv.get(r) || 0, 1)).map(([r, l]) => ({ r, l }));
      unlockAll(lv);
      const usableNow = () => new Set([...demons.values()].filter((d) => canUse(d, lv)).map((d) => d.id));
      let usable = usableNow();
      for (let guard = 0; todo.length && guard < 100; guard++) {
        // the ready ones: enabled, with a demon to raise it; cheapest first (so the early steps open things soonest)
        const ready = todo.filter((t) => (lv.get(t.r) || 0) >= 1 && trainers(t.r, lv.get(t.r), lv).length)
          .map((t) => ({ t, c: cost({ lv, pts: st.pts }, t.r, t.l) })).sort((a, b) => a.c - b.c);
        if (!ready.length) break;
        // a race another one waits on goes first only up to what that one needs; here every race goes to its final level
        const { t } = ready[0], R = races.get(t.r), from = lv.get(t.r);
        const with_ = trainers(t.r, from, lv);
        const dp = cost({ lv, pts: from === st.lv.get(t.r) ? st.pts : new Map() }, t.r, t.l);
        lv.set(t.r, t.l);
        const opened = unlockAll(lv);
        const now = usableNow(), newDemons = [...now].filter((id) => !usable.has(id)).map((id) => demons.get(id));
        usable = now;
        out.push({
          race: t.r, name: R.name, tier: R.tier, from, to: t.l, dp, with: with_,
          mitama: t.l > P.plainCap,
          opens: opened.map((id) => ({ id, name: raceName(id) })),
          demons: newDemons,
          extend: R.extend.filter(([l]) => l > from && l <= t.l).map(([l, id, name, text, who]) => ({ l, id, name, text, who })),
          assists: R.assists.filter(([l]) => l > from && l <= t.l).map(([l, ch]) => ({ l, ch })),
        });
        todo = todo.filter((x) => x !== t);
      }
      if (todo.length) out.stuck = todo.map((t) => raceName(t.r));
      return out;
    }
    /** The points DLv 1 to 10 takes in a tier (every race of a tier costs the same, give or take a rounding). */
    const tierPoints = (t) => { const r = [...races.values()].find((x) => x.tier === t); return r ? r.cum[P.cap] : 0; };
    return { P, races, demons, raceName, start, solve, cost, waysIn, canUse, trainers, unlockAll, tierPoints };
  }

  /** The planner's goals, kept between visits: race levels (with the boost they are for) and demons. */
  function dplGoalsSave() { store.set('dpl-goals', JSON.stringify(S.dpl.goals)); }

  /** A demon's picture (the Fusion Planner's, from the client); the first letter when there is none. */
  function dplPic(d, big) {
    const box = h('span', { class: 'dcm-icon' + (big ? ' fus-pic-big' : ''), 'aria-hidden': 'true' });
    box.append(h('img', { src: `dcm-icons/${d.id}.png`, alt: '', loading: 'lazy', onerror: (ev) => ev.target.remove() }));
    return box;
  }

  /**
   * A search field over the boosts you can aim for: the passive each level teaches (stats, affinities, Limit Break cap,
   * for you or for your demon), always on once learned. The digitalize-only choices (assists: duration, cooldown, the
   * demon's stat share and skills, Demon Force, Mitama sets) are not goals here. A demon to digitalize is one, found by
   * its name (listed once something is typed): the goal is being able to digitalize it.
   * Each says what it costs from where you are, and the cheapest comes first among equal matches.
   * Pick as many as you like: a click (or Enter) ticks or unticks one and the list stays open with the same search;
   * "Add all shown" ticks every match you don't have yet. `toggle(goal)` adds or removes it, `isOn(goal)` says if it's in.
   */
  /** A goal's chip text: its kind, and for a race its tier ("T5"), bluer the higher it is. */
  const dplKindText = (x) => (x.kind === 'Race' ? ['Race', h('span', { class: 'dpl-tier', 'data-tier': String(x.tier), text: `T${x.tier}` })] : [x.kind]);

  function dplGoalCombo(E, st, toggle, isOn) {
    const items = [];
    E.races.forEach((r) => {
      const cur = st.lv.get(r.id) || 0;
      r.extend.forEach(([l, , , text, who]) => items.push({ kind: who === 'demon' ? 'Demon' : 'You', label: text, race: r.name, l, have: cur >= l,
        goal: { k: 'race', race: r.id, level: l, why: `${text}${who === 'demon' ? ' (demon)' : ''}` } }));
      // the race itself: opening it is a goal like any boost (the race at DLv 1); the open ones are listed as had
      if (r.unlock.length) items.push({ kind: 'Race', label: `Unlock ${r.name}`, race: r.name, tier: r.tier, l: 1, have: cur >= 1, open: cur,
        needs: r.unlock.map(([u, ul]) => `${E.raceName(u)} ${ul}`).join(' + '), goal: { k: 'race', race: r.id, level: 1 } });
    });
    // every demon you can digitalize with, found by its name: the goal is being able to digitalize it. They are listed
    // only once something is typed (there are hundreds), and match on the demon's name alone.
    E.demons.forEach((d) => items.push({ kind: 'Digitalize', demon: true, label: d.name, race: E.raceName(d.race), d, have: E.canUse(d, st.lv),
      needs: E.waysIn(d).map(([r, l]) => `${E.raceName(r)} ${l}`).join(' or ') || 'a special unlock', goal: { k: 'demon', id: d.id } }));
    let costed = false;
    const costAll = () => {   // a plan per boost, once, when the list is first opened (a few ms in all)
      if (costed) return;
      costed = true;
      items.forEach((x) => { const p = x.have ? null : E.solve(st, [x.goal]); x.dp = x.have ? 0 : p && p.ok ? p.cost : Infinity; });
      items.forEach((x) => {
        const where = x.demon ? `${x.race}, ${x.have ? 'you can digitalize it' : `needs ${x.needs}`}` : x.kind === 'Race' ? (x.have ? `open, DLv ${x.open}` : `needs ${x.needs}`) : `${x.race} DLv ${x.l}`;
        x.sub = `${where} · ${x.have && !x.demon ? 'you have it' : x.have ? '' : x.dp === Infinity ? 'no way found' : `${dpShort(x.dp)} DP from here`}`.replace(/ · $/, '');
      });
    };
    const wrap = h('div', { class: 'fus-combo' });
    const ph = 'Add a goal: search a boost, a race to unlock or a demon to digitalize, e.g. Intelligence, Limit Break, Seraphim, Pixie';
    const input = h('input', { class: 'input', type: 'search', placeholder: ph, 'aria-label': ph, spellcheck: 'false', autocomplete: 'off', role: 'combobox', 'aria-expanded': 'false' });
    const menu = h('div', { class: 'fus-menu dpl-menu', role: 'listbox', 'aria-multiselectable': 'true', hidden: true });
    let hits = [], at = 0;
    const draw = () => {
      costAll();
      const keep = menu.scrollTop;
      const q = input.value.trim().toLowerCase();
      const words = q.split(/\s+/).filter(Boolean);
      // "demon" / "you" in the search pick whose boost it is
      hits = items.filter((x) => { if (x.demon && !words.length) return false; const hay = (x.demon ? x.label : `${x.kind}${x.tier ? ` t${x.tier}` : ''} ${x.label} ${x.race}`).toLowerCase(); return words.every((w) => hay.includes(w)); })
        // the ones you have last (dimmed): still listed, so you can see where a boost comes from
        .sort((a, b) => (a.have ? 1 : 0) - (b.have ? 1 : 0) || (a.label.toLowerCase().startsWith(q) ? 0 : 1) - (b.label.toLowerCase().startsWith(q) ? 0 : 1) || a.dp - b.dp || a.label.localeCompare(b.label)).slice(0, 80);
      at = Math.min(at, Math.max(0, hits.length - 1));
      // "Add all" never takes demons: a short name ("a") matches hundreds, and each one is picked on purpose
      const addable = hits.filter((x) => !x.have && !x.demon && !isOn(x.goal));
      const head = h('div', { class: 'dpl-menu-head' },
        h('span', { class: 'src-count', style: 'margin-left:0', text: `${hits.length}${hits.length === 80 ? '+' : ''} match${hits.length === 1 ? '' : 'es'} · ${U.goals.length} picked` }),
        h('span', { class: 'toolbar-spacer' }),
        addable.length && words.length ? h('button', { type: 'button', class: 'btn btn-sm btn-ghost', onmousedown: (ev) => { ev.preventDefault(); addable.forEach((x) => toggle(x.goal)); draw(); } }, `Add all ${addable.length} shown`) : null);
      menu.replaceChildren(head, ...(hits.length ? hits.map((x, i) => {
        const on = isOn(x.goal);
        // one you have already can't be picked: listed (dimmed, no box) only to show where you got it
        if (x.have) return h('div', { class: 'fus-opt dpl-have', role: 'option', 'aria-selected': 'false', 'aria-disabled': 'true', onmousedown: (ev) => ev.preventDefault() },
          h('span', { class: 'dpl-check dpl-check-none', 'aria-hidden': 'true' }),
          h('span', { class: 'tag dpl-kind' + (x.kind === 'Demon' ? ' tag-demon' : x.kind === 'Race' ? ' tag-digi' : '') }, ...dplKindText(x)),
          x.demon ? dplPic(x.d) : null,
          h('span', { class: 'fus-opt-name', text: x.label }), h('span', { class: 'src-count', text: x.sub }));
        return h('div', { class: 'fus-opt' + (i === at ? ' on' : ''), role: 'option', 'aria-selected': String(on),
          onmousedown: (ev) => { ev.preventDefault(); at = i; pick(x); } },
          h('span', { class: 'dpl-check' + (on ? ' ticked' : ''), 'aria-hidden': 'true' }, on ? icon('check') : null),
          h('span', { class: 'tag dpl-kind' + (x.kind === 'Demon' ? ' tag-demon' : x.kind === 'Race' ? ' tag-digi' : ''), 'data-tip': x.kind === 'Demon' ? 'A boost for your demon' : x.kind === 'Race' ? 'A race to unlock: it opens at DLv 1 once every race it needs reaches its level' : x.demon ? 'A demon to digitalize with: the goal is being able to digitalize it' : 'A boost for you' }, ...dplKindText(x)),
          x.demon ? dplPic(x.d) : null,
          h('span', { class: 'fus-opt-name', text: x.label }), h('span', { class: 'src-count', text: x.sub }));
      }) : [h('div', { class: 'fus-opt empty', text: 'Nothing matches.' })]));
      menu.scrollTop = keep;
      const cur = menu.querySelector('.fus-opt.on'); if (cur) cur.scrollIntoView({ block: 'nearest' });
    };
    const U = S.dpl;
    const open = () => { menu.hidden = false; input.setAttribute('aria-expanded', 'true'); draw(); };
    const close = () => { menu.hidden = true; input.setAttribute('aria-expanded', 'false'); };
    const pick = (x) => { if (x.have) return; toggle(x.goal); draw(); };   // stays open, same search: pick the next one
    // the arrow keys stop at the last one you can pick (the ones you have are listed after them)
    const lastPickable = () => Math.max(0, hits.findIndex((x) => x.have) < 0 ? hits.length - 1 : hits.findIndex((x) => x.have) - 1);
    input.addEventListener('focus', open);
    input.addEventListener('input', () => { at = 0; menu.scrollTop = 0; open(); });
    input.addEventListener('blur', close);
    input.addEventListener('keydown', (ev) => {
      if (ev.key === 'ArrowDown') { ev.preventDefault(); if (menu.hidden) open(); else { at = Math.min(at + 1, lastPickable()); draw(); } }
      else if (ev.key === 'ArrowUp') { ev.preventDefault(); at = Math.max(at - 1, 0); draw(); }
      else if (ev.key === 'Enter') { if (!menu.hidden && hits[at]) { ev.preventDefault(); pick(hits[at]); } }
      else if (ev.key === 'Escape') { if (!menu.hidden) { ev.stopPropagation(); close(); } }
    });
    wrap.append(h('div', { class: 'search' }, icon('search'), input), menu);
    return wrap;
  }

  const dpShort = (v) => (v >= 1e6 ? `${nf.format(Math.round(v / 1e5) / 10)}M` : v >= 1e3 ? `${nf.format(Math.round(v / 100) / 10)}K` : nf.format(v));

  function pageDigiPlan() {
    const U = S.dpl, page = h('div', { class: 'page page-dpl' });
    loadDigiPlan();
    const read = S.data && S.data.ranks && Array.isArray(S.data.ranks.digitalize_levels) && S.data.ranks.digitalize_levels.length ? S.data.ranks.digitalize_levels : null;
    const mine = U.from === 'mine' && !!read;
    page.append(pageHead('Pick what you want from Digitalize. The planner works out which races to raise, in what order, and the points it takes.',
      h('div', { class: 'seg', role: 'group', 'aria-label': 'Start from' },
        [['mine', 'From my levels'], ['scratch', 'From scratch']].map(([id, label]) =>
          h('button', { type: 'button', 'aria-pressed': String((id === 'mine') === mine), disabled: id === 'mine' && !read ? true : null,
            'data-tip': id === 'mine' ? (read ? `The Digitalization Levels and points read from the game (${(S.data.ranks.source || '').replace(/^Client memory, /, '')})` : 'No Digitalization Levels read yet. Take a snapshot.') : 'Only the races everyone starts with, at level 1',
            onclick: () => { U.from = id; store.set('dpl-from', id); render(); } }, label)))));
    if (U.state !== 'ready') {
      page.append(h('p', { class: 'note', text: U.state === 'failed' ? 'The planner data could not be loaded (digi-planner.json is missing beside the page).' : 'Loading the Digitalization tables…' }));
      return page;
    }
    const E = U.eng, st = E.start(mine ? read : null);
    if (U.from === 'mine' && !read) page.append(h('p', { class: 'note', text: 'No Digitalization Levels read yet, so this plans from scratch. Take a snapshot to plan from yours.' }));
    // The search stays in place while you tick boosts in it (it keeps its focus and list); everything under it is redrawn
    // at each tick: the goal cards, the plan and the race map.
    const chips = h('div', { class: 'dpl-goals-wrap' }), count = h('span', { class: 'card-sub' }), rest = h('div', { class: 'dpl-rest' });
    const clear = h('button', { type: 'button', class: 'btn btn-sm btn-ghost', onclick: () => { U.goals = []; dplGoalsSave(); render(); } }, 'Clear all');
    const redraw = () => {
      const plan = U.goals.length ? E.solve(st, U.goals) : null;
      count.textContent = U.goals.length ? `${U.goals.length} goal${U.goals.length === 1 ? '' : 's'}` : '';
      clear.hidden = !U.goals.length;
      chips.replaceChildren(dplGoalCards(E, st, toggle, redraw));
      // no goals: no plan section (the goals card then suggests some boosts); the race map is always there
      rest.replaceChildren(...[plan ? dplPlan(E, st, plan) : null, dplMap(E, st, plan, redraw)].filter(Boolean));
    };
    const same = (x, g) => x.k === g.k && (g.k === 'race' ? x.race === g.race && (x.why || '') === (g.why || '') : x.id === g.id);
    const isOn = (g) => U.goals.some((x) => same(x, g));
    const toggle = (g) => {
      const i = U.goals.findIndex((x) => same(x, g));
      if (i >= 0) U.goals.splice(i, 1); else U.goals.push({ ...g });
      dplGoalsSave(); redraw();
    };
    page.append(sect('Your goals', count, null, h('section', { class: 'card' },
      h('div', { class: 'toolbar' }, dplGoalCombo(E, st, toggle, isOn), h('span', { class: 'toolbar-spacer' }), clear), chips)));
    page.append(rest);
    redraw();
    page.append(h('p', { class: 'note fus-foot', text: 'Races, unlocks, points per level, demons, passives and assists: the server’s Digitalization tables, with the client’s names and texts. The rules (points go to your demon’s race; a race opens when all its needs are met; level 10 at most, 5 without a Mitama-fused demon): the server’s code, which NewMoon may have changed. Not checked in game yet.' }));
    return page;
  }

  /**
   * A race's picture: its first three demons (the ones its lowest levels open; for Entity and Fiend, whose levels open
   * none, the demons other races open for it) as small round orbs, like a gear set's pieces.
   */
  function dplRaceOrbs(E, r) {
    const ids = r.gated.map((d) => d.id);
    [...E.demons.values()].filter((d) => d.race === r.id && !d.gate).forEach((d) => ids.push(d.id));
    const three = [...new Set(ids)].slice(0, 3);
    return h('span', { class: `slot-icon orbs orbs-${three.length} dpl-orbs`, 'aria-hidden': 'true' },
      three.map((id) => h('img', { src: `dcm-icons/${id}.png`, alt: '', loading: 'lazy', onerror: (ev) => { ev.target.style.visibility = 'hidden'; } })));
  }

  /**
   * The strongest boosts you don't have yet, one of each kind (Limit Break cap, each stat...; for you and for your demon
   * apart): the biggest number wins, and among equals the race that costs the least from where you are.
   */
  function dplSuggestions(E, st, count) {
    const cands = [];
    E.races.forEach((r) => {
      const cur = st.lv.get(r.id) || 0;
      r.extend.forEach(([l, , , text, who]) => {
        if (cur >= l) return;
        cands.push({ text, who, r, l, val: Number((/\+(\d+)/.exec(text) || [])[1]) || 0, goal: { k: 'race', race: r.id, level: l, why: `${text}${who === 'demon' ? ' (demon)' : ''}` } });
      });
    });
    cands.forEach((c) => { const p = E.solve(st, [c.goal]); c.dp = p.ok ? p.cost : Infinity; });
    const seen = new Set(), top = [];
    for (const c of cands.filter((x) => x.dp < Infinity).sort((a, b) => b.val - a.val || a.dp - b.dp)) {
      const kind = c.text.replace(/\s*\+\d+.*$/, '') + '|' + c.who;
      if (seen.has(kind)) continue;
      seen.add(kind);
      top.push(c);
      if (top.length === count) break;
    }
    return top;
  }

  /**
   * The goal cards under the search: what each boost is, its race and level (changeable), and a remove button.
   * With no goals yet: a few suggestions to click instead, so the page shows what Digitalize gives before anything is typed.
   * `toggle(goal)` adds or removes a goal and redraws; `redraw()` redraws after a change made here.
   */
  function dplGoalCards(E, st, toggle, redraw) {
    const U = S.dpl;
    if (!U.goals.length) {
      const top = dplSuggestions(E, st, 6);
      return h('div', { class: 'dpl-empty' },
        h('p', { class: 'dpl-empty-title', text: 'What do you want from Digitalize?' }),
        h('p', { class: 'note', text: 'Search the boosts the Digitalization Levels teach (stats, affinities, the Limit Break cap), for you or for your demon, a race to unlock, or a demon by name to digitalize, and tick the ones you want. Or start from one of the strongest you don’t have yet:' }),
        h('div', { class: 'dpl-suggs' }, top.map((c) => h('button', { type: 'button', class: 'dpl-sugg', 'data-tip': `Add “${c.text}” for ${c.who === 'demon' ? 'your demon' : 'you'} as a goal: ${c.r.name} DLv ${c.l}, ${nf.format(c.dp)} Digitalize points from here`, onclick: () => toggle(c.goal) },
          dplRaceOrbs(E, c.r),
          h('span', { class: 'dpl-goal-text' }, h('span', { class: 'dpl-goal-title' }, h('b', { text: c.text }), h('span', { class: 'tag dpl-kind' + (c.who === 'demon' ? ' tag-demon' : ''), text: c.who === 'demon' ? 'Demon' : 'You' })),
            h('span', { class: 'card-sub', text: `${c.r.name} DLv ${c.l} · ${dpShort(c.dp)} DP` })),
          h('span', { class: 'dpl-sugg-plus', 'aria-hidden': 'true' }, icon('plus'))))));
    }
    const chip = (g) => {
      const remove = h('button', { type: 'button', class: 'dpl-x', 'aria-label': 'Remove this goal', onclick: () => toggle(g) }, icon('x'));
      if (g.k === 'demon') {
        const d = E.demons.get(g.id);
        if (!d) return null;
        const ok = E.canUse(d, st.lv);
        return h('span', { class: 'dpl-goal' }, dplPic(d), h('span', { class: 'dpl-goal-text' }, h('b', { text: `Digitalize ${d.name}` }),
          h('span', { class: 'card-sub', text: ok ? 'you can already' : E.waysIn(d).map(([r, l]) => `${E.raceName(r)} ${l}`).join(' or ') })), remove);
      }
      const R = E.races.get(g.race);
      if (!R) return null;
      const cur = st.lv.get(g.race) || 0;
      const sel = h('select', { class: 'input dpl-lvl', 'aria-label': `${R.name} level to reach`, onchange: () => { g.level = Number(sel.value); dplGoalsSave(); redraw(); } },
        Array.from({ length: E.P.cap }, (_, k) => k + 1).map((l) => h('option', { value: l, selected: l === g.level ? true : null, text: `DLv ${l}` })));
      // the boost is the card's title (what you want); the race and level under it
      const demonSide = /\(demon\)$/.test(g.why || '');
      // a race goal (no boost: "Unlock X" from the search, or "Plan to here"): while the race is locked, the card
      // says what opens it, each race level ticked when you have it
      const title = g.why ? g.why.replace(/ \(demon\)$/, '') : cur ? R.name : `Unlock ${R.name}`;
      const done = cur >= g.level;
      const needs = !g.why && !cur && R.unlock.length ? h('span', { class: 'dpl-needs dpl-goal-needs', 'data-tip': `${R.name} opens at DLv 1 once all of these are reached` },
        h('span', { text: 'needs' }), R.unlock.map(([u, l]) => h('span', { class: (st.lv.get(u) || 0) >= l ? 'met' : '', text: `${E.raceName(u)} ${l}` }))) : null;
      return h('span', { class: 'dpl-goal' + (done ? ' done' : '') }, dplRaceOrbs(E, R),
        h('span', { class: 'dpl-goal-text' }, h('span', { class: 'dpl-goal-title' }, h('b', { text: title }),
          g.why ? h('span', { class: 'tag dpl-kind' + (demonSide ? ' tag-demon' : ''), text: demonSide ? 'Demon' : 'You' })
            : h('span', { class: 'tag dpl-kind tag-digi', text: 'Race' })),
          h('span', { class: 'card-sub', text: [g.why ? R.name : null, done ? 'you have it' : cur ? `you have DLv ${cur}` : 'locked'].filter(Boolean).join(' · ') }), needs), sel, remove);
    };
    return h('div', { class: 'dpl-goals' }, U.goals.map(chip));
  }

  /**
   * The plan: the points in all, each race's share of them as one bar, and the steps down a numbered line, in an order
   * that works (a race is raised only once it is open and you have a demon of it).
   */
  function dplPlan(E, st, plan) {
    if (!plan.ok) return sect('The plan', null, null, h('section', { class: 'card' }, h('p', { class: 'note', text: plan.why })));
    const S2 = plan.steps;
    if (!S2.length) return sect('The plan', null, null, h('section', { class: 'card dpl-done' }, icon('check'),
      h('div', null, h('b', { text: 'You already have every goal.' }), h('p', { class: 'note', text: 'Nothing to raise. Add another boost above to plan further.' }))));
    const mit = S2.filter((x) => x.mitama);
    const mitTip = `Past DLv ${E.P.plainCap} a race only rises while you are digitalized with a Mitama-fused demon of it (a plain one stops at ${E.P.plainCap})`;
    const sum = h('div', { class: 'dpl-sum' },
      h('div', { class: 'dpl-stat' }, h('span', { class: 'card-sub', text: 'Digitalize points' }), h('b', { 'data-tip': `${nf.format(plan.cost)} Digitalize points`, text: dpShort(plan.cost) })),
      h('div', { class: 'dpl-stat' }, h('span', { class: 'card-sub', text: S2.length === 1 ? 'Race to raise' : 'Races to raise' }), h('b', { text: String(S2.length) })),
      mit.length ? h('div', { class: 'dpl-stat dpl-stat-list' },
        h('span', { class: 'card-sub', 'data-tip': mitTip, text: 'Needs a Mitama-fused demon' }), h('span', { 'data-tip': mitTip, text: mit.map((x) => x.name).join(', ') })) : null);
    // each step's share of the points, in the plan's order (hover a part for its race). A part, and the step's number
    // on the line below, wear the colour of the race's side (Law, Neutral, Chaos), as the race map's columns do.
    const share = (x) => Math.round((x.dp / plan.cost) * 100);
    const side = (x) => String((E.races.get(x.race) || {}).side || '').toLowerCase();
    const sideName = (x) => { const s = side(x); return s ? s.charAt(0).toUpperCase() + s.slice(1) : ''; };
    const bar = S2.length > 1 && plan.cost > 0 ? h('div', { class: 'dpl-dpbar' },
      S2.map((x, i) => h('i', { class: side(x), style: `flex-grow:${Math.max(1, Math.round((x.dp / plan.cost) * 1000))}`, tabindex: 0, role: 'img', 'aria-label': `Step ${i + 1}, ${x.name} (${sideName(x)}): ${share(x)}% of the points`,
        'data-tip': `${i + 1}. ${x.name} (${sideName(x)}) · ${nf.format(x.dp)} DP · ${share(x) || '<1'}% of the plan` }))) : null;
    const line = (key, ...body) => h('div', { class: 'dpl-line' }, h('span', { class: 'dpl-key', text: key }), body);
    const list = h('ol', { class: 'dpl-steps' }, S2.map((x, i) => {
      const R = E.races.get(x.race), lines = [];
      if (x.with.length) lines.push(line('Raise it with',
        h('span', { class: 'dpl-pics' }, x.with.slice(0, 8).map((d) => h('span', { class: 'dpl-mini', 'data-tip': `${d.name} (base level ${d.base})` }, dplPic(d), h('span', { text: d.name }))),
          x.with.length > 8 ? h('span', { class: 'src-count', text: `+${x.with.length - 8} more` }) : null)));
      // the boosts learned on the way (for you, or for your demon); the assists and demons a level opens are in the race's table
      if (x.extend.length) lines.push(line('You get',
        h('span', { class: 'dpl-tags' }, x.extend.map((e) => h('span', { class: 'tag' + (e.who === 'demon' ? ' tag-demon' : ''), 'data-tip': `${e.name}, learned at DLv ${e.l}, for ${e.who === 'demon' ? 'your demon' : 'you'}`, text: `${e.text}${e.who === 'demon' ? ' (demon)' : ''}` })))));
      if (x.opens.length) lines.push(line('Opens', h('span', { class: 'dpl-tags' }, x.opens.map((r) => h('span', { class: 'tag tag-digi', text: r.name })))));
      // the step wears its race's side: the card's tint, its number, and the line down to the next number
      return h('li', { class: 'dpl-step ' + side(x) },
        h('span', { class: 'dpl-n', 'aria-hidden': 'true', text: String(i + 1) }),
        h('div', { class: 'dpl-step-head' }, R ? dplRaceOrbs(E, R) : null,
          h('span', { class: 'dpl-step-name' }, h('b', { text: x.name }),
            h('span', { class: 'dpl-step-lv' }, `DLv ${x.from}`, h('span', { class: 'dpl-arrow', text: ` → ${x.to}` }))),
          h('span', { class: 'tag', 'data-tip': 'Tier: how many points its levels take, 1 (least) to 5 (most)', text: `Tier ${x.tier}` }),
          x.mitama ? h('span', { class: 'tag dpl-mitama', 'data-tip': mitTip, text: 'Mitama-fused demon' }) : null,
          h('span', { class: 'toolbar-spacer' }),
          h('span', { class: 'dpl-dp', 'data-tip': `${nf.format(x.dp)} Digitalize points`, text: `${dpShort(x.dp)} DP` })),
        lines);
    }));
    const stuck = S2.stuck ? h('p', { class: 'note warn', text: `Could not place: ${S2.stuck.join(', ')}.` }) : null;
    return sect('The plan', 'cheapest first, in an order that works', null, h('section', { class: 'card dpl-plan-card' }, sum, bar, list, stuck));
  }

  /** Every race as a tile, by side (Law, Neutral, Chaos) and tier, with your level, the plan's level and what opens it. */
  function dplMap(E, st, plan, redraw) {
    const U = S.dpl, need = plan && plan.ok ? plan.need : new Map();
    const sides = [['LAW', 'Law', 'law'], ['NEUTRAL', 'Neutral', 'neutral'], ['CHAOS', 'Chaos', 'chaos']];
    const tiers = [...new Set([...E.races.values()].map((r) => r.tier))].sort((a, b) => a - b);
    const tile = (r) => {
      const cur = st.lv.get(r.id) || 0, to = Math.max(need.get(r.id) || 0, cur), pts = st.pts.get(r.id) || 0;
      const next = cur >= 1 && cur < E.P.cap ? r.xp[cur - 1] : 0;
      const pct = next ? Math.min(100, (pts / next) * 100) : cur >= E.P.cap ? 100 : 0;
      // ten parts, one per level: yours, the plan's, and the part you are working on filled as far as your points go
      const bar = h('span', { class: 'dpl-bar', 'aria-hidden': 'true' },
        Array.from({ length: E.P.cap }, (_, k) => h('i', { class: k < cur ? 'on' : k < to ? 'plan' : '',
          style: k === cur && k >= to && next && pct >= 1 ? `background:linear-gradient(90deg, var(--text-2) ${pct}%, var(--grid) ${pct}%)` : null })));
      // what opens it: a need you already meet gets a tick; a race open from the start says so (every tile the same height)
      const needs = r.unlock.length ? h('span', { class: 'dpl-needs' }, r.unlock.map(([u, l]) => h('span', { class: (st.lv.get(u) || 0) >= l ? 'met' : '', text: `${E.raceName(u)} ${l}` })))
        : h('span', { class: 'dpl-needs dpl-needs-none', text: 'Open from the start' });
      const needTip = r.unlock.length && !cur ? ` It opens with ${r.unlock.map(([u, l]) => `${E.raceName(u)} ${l}`).join(' and ')}.` : '';
      return h('button', { type: 'button', class: 'dpl-tile' + (cur ? '' : ' locked') + (to > cur ? ' planned' : '') + (U.sel === r.id ? ' sel' : ''), 'aria-pressed': String(U.sel === r.id),
        'data-tip': cur ? `${r.name}: DLv ${cur}` + (next ? `, ${nf.format(pct)}% to the next` : '') + (to > cur ? `. The plan takes it to ${to}.` : '') : `${r.name} is locked.${needTip}` + (to ? ` The plan opens it${to > 1 ? ` and takes it to ${to}` : ''}.` : ''),
        onclick: () => { U.sel = U.sel === r.id ? null : r.id; redraw(); } },
        // two columns: the race's orbs, then the rest (name and level, the level bar, what opens it)
        dplRaceOrbs(E, r),
        h('span', { class: 'dpl-tile-body' },
          h('span', { class: 'dpl-tile-head' }, h('b', { text: r.name }), h('span', { class: 'toolbar-spacer' }), h('span', { class: 'dpl-lv', text: cur ? (to > cur ? `${cur} → ${to}` : String(cur)) : to ? `→ ${to}` : '—' })),
          bar, needs));
    };
    const grid = h('div', { class: 'dpl-map' },
      h('span'), sides.map(([, label, cls]) => h('span', { class: 'dpl-col-head ' + cls, text: label })),
      tiers.map((t) => [h('span', { class: 'dpl-row-head', 'data-tip': `Tier ${t}: DLv 1 to 10 takes ${nf.format(E.tierPoints(t))} Digitalize points` + (t > 1 ? `, ${t}× tier 1` : '') },
        h('b', { text: `Tier ${t}` }), h('span', { class: 'dpl-mult', text: `${dpShort(E.tierPoints(t))} DP` })),
        sides.map(([side]) => h('div', { class: 'dpl-cell' }, [...E.races.values()].filter((r) => r.side === side && r.tier === t).map(tile)))]));
    const body = [h('section', { class: 'card dpl-map-card' }, grid, legend(['dpl-sw-on', 'Your levels'], ['dpl-sw-plan', 'Levels the plan adds']))];
    if (U.sel && E.races.has(U.sel)) body.push(dplRace(E, st, E.races.get(U.sel), need, redraw));
    const open = [...E.races.values()].filter((r) => (st.lv.get(r.id) || 0) >= 1).length;
    return sect('Races', `${open} of ${E.races.size} open · click one for its levels`, null, ...body);
  }

  /**
   * One race's ten levels: points, demons, passive and assists, each with a button to plan up to it. The levels you
   * have are dimmed, the one you are at is marked, and the ones the plan adds are tinted.
   */
  function dplRace(E, st, r, need, redraw) {
    const U = S.dpl, cur = st.lv.get(r.id) || 0, to = Math.max(need.get(r.id) || 0, cur);
    const next = cur >= 1 && cur < E.P.cap ? r.xp[cur - 1] : 0, pct = next ? Math.min(100, ((st.pts.get(r.id) || 0) / next) * 100) : 0;
    const rows = Array.from({ length: E.P.cap }, (_, k) => k + 1).map((l) => {
      const ds = r.gated.filter((d) => d.level === l), ex = r.extend.filter(([x]) => x === l), as = r.assists.find(([x]) => x === l);
      return h('tr', { class: l <= cur ? 'dpl-have' : l <= to ? 'dpl-plan-row' : '' },
        h('td', { class: 'num' }, l === cur ? h('span', { class: 'dpl-here', 'data-tip': 'Your level now' + (next ? `, ${nf.format(pct)}% to the next` : ''), text: '●' }) : null, String(l)),
        h('td', { class: 'num', text: l === 1 ? '—' : nf.format(r.xp[l - 2]) }),
        h('td', null, ds.length ? h('span', { class: 'dpl-pics' }, ds.map((d) => h('span', { class: 'dpl-mini' }, dplPic(d), h('span', { text: d.name })))) : null),
        h('td', null, ex.map(([, , name, text, who]) => h('span', { class: 'tag' + (who === 'demon' ? ' tag-demon' : ''), 'data-tip': `${name}, for ${who === 'demon' ? 'your demon' : 'you'}`, text: `${text}${who === 'demon' ? ' (demon)' : ''}` }))),
        h('td', { class: 'dpl-assist' }, as ? as[1].map(([, t], i) => [i ? h('span', { class: 'dpl-or', text: 'or' }) : null, h('span', { text: t })]) : null),
        h('td', { class: 'dpl-act' }, l > cur ? h('button', { type: 'button', class: 'btn btn-sm btn-ghost', 'data-tip': `Add ${r.name} DLv ${l} as a goal`, onclick: () => {
          const g = U.goals.find((x) => x.k === 'race' && x.race === r.id && !x.why);
          if (g) g.level = l; else U.goals.push({ k: 'race', race: r.id, level: l });
          dplGoalsSave(); redraw();
        } }, 'Plan to here') : h('span', { class: 'card-sub', text: 'have' })));
    });
    const others = [...E.demons.values()].filter((d) => d.opts.some(([x]) => x === r.id) && d.race !== r.id);
    const own = [...E.demons.values()].filter((d) => d.race === r.id && !d.gate);
    return h('section', { class: 'card dpl-race' },
      h('div', { class: 'dpl-race-head' }, dplRaceOrbs(E, r),
        h('div', { class: 'dpl-goal-text' }, h('h3', { class: 'card-title', text: r.name }),
          h('span', { class: 'card-sub', text: [cur ? `you have DLv ${cur}` + (next && pct >= 1 ? ` (${nf.format(pct)}% to ${cur + 1})` : '') : 'locked',
            r.unlock.length ? `opens with ${r.unlock.map(([u, l]) => `${E.raceName(u)} ${l}`).join(' + ')}` : 'open from the start'].join(' · ') })),
        h('span', { class: 'toolbar-spacer' }),
        h('span', { class: 'tag', 'data-tip': `DLv 1 to 10 takes ${nf.format(r.cum[E.P.cap])} Digitalize points`, text: `Tier ${r.tier} · ${dpShort(r.cum[E.P.cap])} DP` }),
        h('button', { type: 'button', class: 'dpl-x', 'aria-label': 'Close this race', onclick: () => { U.sel = null; redraw(); } }, icon('x'))),
      h('div', { class: 'tbl-wrap' }, h('table', { class: 'tbl dpl-tbl' },
        h('thead', null, h('tr', null, ['DLv', 'Points to reach', 'Demons it opens', 'Boost learned', 'Assist: pick one', ''].map((t, i) => h('th', { class: i < 2 ? 'num' : null, text: t })))),
        h('tbody', null, rows))),
      own.length ? h('div', { class: 'dpl-line' }, h('span', { class: 'dpl-key', text: 'Its other demons' }),
        h('span', { class: 'dpl-pics' }, own.map((d) => h('span', { class: 'dpl-mini', 'data-tip': `Opened by ${d.opts.map(([x, l]) => `${E.raceName(x)} ${l}`).join(' or ')}` }, dplPic(d), h('span', { text: d.name }))))) : null,
      others.length ? h('div', { class: 'dpl-line' }, h('span', { class: 'dpl-key', text: 'Also opens' }),
        h('span', { class: 'dpl-pics' }, others.map((d) => h('span', { class: 'dpl-mini', 'data-tip': `${d.name} (${E.raceName(d.race)}): ${d.opts.map(([x, l]) => `${E.raceName(x)} ${l}`).join(' or ')}` }, dplPic(d), h('span', { text: d.name }))))) : null);
  }

  // ---------- Tools: Fusion Planner ----------
  // fusion-data.json is made by tools\fusionexport: the demons, races and special fusions are the IMAGINE client's; the
  // race chart and the tri-fusion rules are the comp_hack server's (FusionManager.cpp), followed here step by step,
  // including its quirks, so the planner says what that server would make. IMAGINE may have changed them.
  function loadFusion() {
    if (S.fus.state !== 'idle') return;
    S.fus.state = 'loading';
    fetch('fusion-data.json', { cache: 'no-store' }).then((r) => (r.ok ? r.json() : null)).catch(() => null).then((d) => {
      S.fus.eng = d ? fusionEngine(d) : null;
      S.fus.state = d ? 'ready' : 'failed';
      if (S.page === 'fusion' || S.page === 'ddb' || S.page === 'rebirth') render();   // the Demon Database's "Where to get it" and the Rebirth Planner use it too
    });
  }

  /** The fusion rules over the exported data: fuse(), the demon list, and every recipe for a target. */
  function fusionEngine(F) {
    const byId = new Map();
    const demons = F.demons.map(([id, name, race, fam, lvl, lnc, base, opt, mit, ic]) => {
      const d = { id, name, race, fam, lvl, lnc, base: base || id, opt, mit, icon: ic, raceName: F.races[race] || '' };
      d.mitamaFused = (opt & 8) !== 0;
      byId.set(id, d);
      return d;
    });
    demons.forEach((d) => { d.label = d.mitamaFused ? `${d.name} (Mitama)` : d.name; });
    const raceName = (r) => F.races[r] || 'an unknown race';
    const raceIdx = new Map(F.raceOrder.map((r, i) => [r, i]));
    const RANGE = F.rangeFlag;
    // each race's demons in the order the server keeps them: by base level (a stable sort, so equal levels keep file order)
    const ranges = new Map();
    demons.filter((d) => (d.opt & RANGE) !== 0).sort((a, b) => a.lvl - b.lvl || a.id - b.id)
      .forEach((d) => { if (!ranges.has(d.race)) ranges.set(d.race, []); ranges.get(d.race).push(d); });
    const elemIdx = new Map(F.elementals.map((id, i) => [id, i]));
    const mitamaIdx = new Map(F.mitamas.map((id, i) => [id, i]));
    const prio = (race) => { const i = F.triPriority.indexOf(race); return i < 0 ? F.triPriority.length : i; };
    const triFamilyFlat = F.triFamily.flat(2);
    const FAM = F.families, ELEMENTAL_RACE = 55, MITAMA_RACE = 57, HAUNT = 103, FOUL = 101;
    const isDark = (d) => d.race === HAUNT || d.race === FOUL;
    const specialsBySrc = new Map();
    F.specials.forEach((sp, i) => { sp.i = i; sp.src.forEach(([id]) => { if (!specialsBySrc.has(id)) specialsBySrc.set(id, []); if (!specialsBySrc.get(id).includes(sp)) specialsBySrc.get(id).push(sp); }); });
    const specialsFor = new Map();
    F.specials.forEach((sp) => { if (!specialsFor.has(sp.result)) specialsFor.set(sp.result, []); specialsFor.get(sp.result).push(sp); });

    const lvlSum2 = (a, b) => Math.trunc((a + b) / 2 + 1);
    /** The race's demon for a fusion level: the first at that level or above, else the race's highest. */
    function pickInRace(race, adj) {
      const list = ranges.get(race);
      if (!list || !list.length) return null;
      let r = list[0];
      for (const d of list) { r = d; if (d.lvl >= adj) break; }
      return r;
    }
    function rankUpDown(race, id, up) {
      const list = ranges.get(race) || [];
      const i = list.findIndex((d) => d.id === id);
      if (i < 0) return id;
      if (up) return i + 1 < list.length ? list[i + 1].id : id;
      return i > 0 ? list[i - 1].id : id;
    }
    /** An elemental fused with a demon: one rank up or down within the demon's race, or nothing (0). */
    function elementalFuse(elemId, race, id) {
      const ri = raceIdx.get(race), ei = elemIdx.get(elemId);
      if (ri === undefined || ei === undefined) return { id: 0, dir: 0 };
      const dir = F.elemAdjust[ri][ei];
      return dir ? { id: rankUpDown(race, id, dir === 1), dir } : { id: 0, dir: 0 };
    }
    const mapRace = (a, b) => { const i = raceIdx.get(a), j = raceIdx.get(b); return i === undefined || j === undefined ? null : F.raceMap[i][j]; };

    /** A special fusion these demons make (any order), or null. usePlugins false skips the ones that need a plugin. */
    function special(xs, usePlugins) {
      const tri = xs.length === 3, seen = [];
      for (const x of xs) for (const sp of specialsBySrc.get(x.d.id) || []) {
        if (seen.includes(sp) || (sp.src.length === 3) !== tri) continue;
        seen.push(sp);
        if (sp.plugin && !usePlugins) continue;
        const match = sp.src.map(([sid, variant]) => {
          const sd = byId.get(sid);
          return xs.map((x, i) => ((variant ? sd && x.d.base === sd.base : x.d.id === sid) ? i : -1)).filter((i) => i >= 0);
        });
        const ok = (k, used) => k === match.length || match[k].some((i) => !used.has(i) && ok(k + 1, new Set([...used, i])));
        if (ok(0, new Set())) return sp;
      }
      return null;
    }

    /**
     * What the server makes from these demons ({ d, lvl } each; lvl is the demon's current level). Returns
     * { id, special, how: [lines] } with id 0 when they can't be fused. log false skips the explanation (searches).
     */
    function fuse(xs, opts) {
      const o = opts || {}, usePlugins = o.plugins !== false, log = o.log !== false;
      const how = [], say = (t) => { if (log) how.push(t); };
      const out = (id, extra) => Object.assign({ id: id || 0, how }, extra || {});
      // seen in game (2026-11-23): the fusion window won't take a demon with a Mitama fused into it
      const fused = xs.find((x) => x.d.mitamaFused);
      if (fused) { say(`${fused.d.name} has a Mitama fused into it, and the game won’t fuse such a demon.`); return out(0); }
      if (xs.length === 3 && xs.some((x) => x.d.race === MITAMA_RACE || mitamaIdx.has(x.d.base)))
        { say('A Mitama, or a demon with a Mitama fused into it, can’t take part in a tri-fusion.'); return out(0); }
      const sp = special(xs, usePlugins);
      if (sp) { say('This is a special fusion: these demons always make this one, whatever their levels.'); return out(sp.result, { special: sp }); }
      if (xs.length === 3) return out(tri(xs, say));
      const [a, b] = xs;
      if (a.d.race === MITAMA_RACE || b.d.race === MITAMA_RACE) {
        if (a.d.race === b.d.race) { say('Two Mitamas can’t be fused together.'); return out(0); }
        const m = a.d.race === MITAMA_RACE ? a : b, other = m === a ? b : a;
        if (!mitamaIdx.has(m.d.base)) return out(0);
        say(`A Mitama fused into ${other.d.name} makes a Mitama ${other.d.name}: same demon, with Mitama bonuses.`);
        say(`The game only allows it once ${other.d.name}’s reunion ranks add up to 48 or more.`);
        if (!other.d.mit) say(`${other.d.label} has no Mitama form.`);
        return out(other.d.mit, { mitama: true });
      }
      if (a.d.race === ELEMENTAL_RACE || b.d.race === ELEMENTAL_RACE) {
        if (a.d.race === b.d.race) {
          const i = elemIdx.get(a.d.base), j = elemIdx.get(b.d.base);
          const k = i === undefined || j === undefined ? -1 : F.elemMitama[i][j];
          if (k < 0) { say('These two elementals don’t make anything together (two of the same never do).'); return out(0); }
          say('Two different elementals make a Mitama.');
          return out(F.mitamas[k]);
        }
        const e = a.d.race === ELEMENTAL_RACE ? a : b, other = e === a ? b : a;
        const r = elementalFuse(e.d.base, other.d.race, other.d.base);
        if (!r.id) { say(`${e.d.name} does nothing to a ${raceName(other.d.race)}: this pair can’t be fused.`); return out(0); }
        const base = byId.get(other.d.base) || other.d;
        if (r.id === base.id) say(`${e.d.name} moves a ${raceName(other.d.race)} one rank ${r.dir > 0 ? 'up' : 'down'}, but ${base.name} is already the ${r.dir > 0 ? 'highest' : 'lowest'} fusable one, so it stays ${base.name}.`);
        else say(`${e.d.name} moves a ${raceName(other.d.race)} one rank ${r.dir > 0 ? 'up' : 'down'}: ${base.name} (level ${base.lvl}) becomes ${byId.get(r.id).name} (level ${byId.get(r.id).lvl}).`);
        return out(r.id, { elemental: r.dir });
      }
      const race = mapRace(a.d.race, b.d.race);
      if (!race) { say(`${raceName(a.d.race)} and ${raceName(b.d.race)} can’t be fused together.`); return out(0); }
      if (a.d.race === b.d.race) {
        const el = F.elementals[race - 1];
        say(`Two ${raceName(a.d.race)} demons make an elemental.`);
        return out(el);
      }
      const adj = lvlSum2(a.lvl, b.lvl), r = pickInRace(race, adj);
      say(`${raceName(a.d.race)} × ${raceName(b.d.race)} makes a ${raceName(race)}.`);
      if (r) say(`Fusion level: (${a.lvl} + ${b.lvl}) ÷ 2 + 1 = ${adj}. ${r.lvl >= adj ? `${r.name} (level ${r.lvl}) is the first ${raceName(race)} at that level or above.` : `No ${raceName(race)} is that high, so the highest one: ${r.name} (level ${r.lvl}).`}`);
      return out(r && r.id);
    }

    /** Tri-fusion, as the server does it (FusionManager::GetResultDemon, tri branch). */
    function tri(xs, say) {
      const t = xs.slice().sort((x, y) => (y.d.lvl - x.d.lvl) || (prio(x.d.race) - prio(y.d.race)));
      const [x1, x2, x3] = t;
      const darks = t.filter((x) => isDark(x.d)), elems = t.filter((x) => x.d.fam === FAM.ELEMENTAL);
      const nm = (x) => x.d.name;
      if (darks.length === 3) {
        const sum = x1.lvl + x2.lvl + x3.lvl;
        let id = F.triDark[0][1];
        for (const [min, rid] of F.triDark) { if (min > sum) break; id = rid; }
        say(`Three Foul or Haunt demons: the result only depends on their levels added up (${sum}).`);
        return id;
      }
      if (darks.length) {
        // one dark: the other two first, then the dark one; two darks: the higher dark with the other, then the lower dark
        let p, q, last;
        if (darks.length === 1) { const o = t.filter((x) => !isDark(x.d)); [p, q] = o; last = darks[0]; }
        else { p = darks[0]; q = t.find((x) => !isDark(x.d)); last = darks[1]; }
        const r1 = mapRace(p.d.race, q.d.race);
        if (r1 === null) { say(`${raceName(p.d.race)} and ${raceName(q.d.race)} can’t be fused together.`); return 0; }
        const mid = pickInRace(r1, lvlSum2(p.lvl, q.lvl));
        if (!mid) { say(`${nm(p)} and ${nm(q)} don’t make anything.`); return 0; }
        say(`With a Foul or Haunt demon, the others are fused first: ${nm(p)} + ${nm(q)} → ${mid.name} (level ${mid.lvl}).`);
        const r2 = mapRace(mid.race, last.d.race);
        if (r2 === null) { say(`${raceName(mid.race)} and ${raceName(last.d.race)} can’t be fused together.`); return 0; }
        const res = pickInRace(r2, lvlSum2(last.lvl, mid.lvl));
        if (res) say(`Then ${mid.name} + ${nm(last)} → ${res.name}.`);
        else say(`Then ${mid.name} + ${nm(last)} makes nothing.`);
        return res ? res.id : 0;
      }
      if (elems.length === 3) { say('Three elementals can’t be fused.'); return 0; }
      if (elems.length === 2) {
        const other = t.find((x) => x.d.fam !== FAM.ELEMENTAL);
        const [e1, e2] = elems.map((x) => x.d.id);
        const hit = F.triElemental.find((r) => ((r[0] === e1 && r[1] === e2) || (r[0] === e2 && r[1] === e1)) && [r[2], r[3], r[4]].includes(other.d.race));
        if (hit) { say(`Two elementals and a ${raceName(other.d.race)}: a fixed result.`); return hit[5]; }
        say(`These two elementals don’t make anything with a ${raceName(other.d.race)}.`);
        return 0;
      }
      if (elems.length === 1) {
        const o = t.filter((x) => x.d.fam !== FAM.ELEMENTAL), e = elems[0];
        const r1 = mapRace(o[0].d.race, o[1].d.race);
        if (r1 === null) { say(`${raceName(o[0].d.race)} and ${raceName(o[1].d.race)} can’t be fused together.`); return 0; }
        const mid = pickInRace(r1, lvlSum2(o[0].lvl, o[1].lvl));
        if (!mid) { say(`${nm(o[0])} and ${nm(o[1])} don’t make anything.`); return 0; }
        const res = pickInRace(r1, lvlSum2(e.lvl, mid.lvl));
        say(`With one elemental, the other two are fused first: ${nm(o[0])} + ${nm(o[1])} → ${mid.name} (level ${mid.lvl}).`);
        if (res) say(`Then the elemental’s level (${e.lvl}) and ${mid.name}’s give the level: (${e.lvl} + ${mid.lvl}) ÷ 2 + 1 = ${lvlSum2(e.lvl, mid.lvl)} → ${res.name}.`);
        return res ? res.id : 0;
      }
      let extra = 0;
      if (t.some((x) => x.d.fam === FAM.GOD)) extra += 4;
      const neutral = t.every((x) => x.d.lnc < 5000 && x.d.lnc > -5000);
      if (neutral) extra -= 4;
      const sum = x1.lvl + x2.lvl + x3.lvl, adj = Math.trunc(sum / 3 + 1 + extra);
      if (x1.d.race === x2.d.race) {
        const ei = mapRace(x1.d.race, x2.d.race);
        if (!ei) { say(`The two highest are both ${raceName(x1.d.race)}, and two of them don’t make an elemental: nothing comes out.`); return 0; }
        const el = byId.get(F.elementals[ei - 1]);
        const r = elementalFuse(F.elementals[ei - 1], x3.d.race, x3.d.id);
        const up = rankUpDown(x3.d.race, r.id, true);
        say(`The two highest are both ${raceName(x1.d.race)}: they make ${el ? el.name : 'an elemental'}, which then moves ${nm(x3)} one rank ${r.dir > 0 ? 'up' : r.dir < 0 ? 'down' : '(it does nothing to this race)'}, and a tri-fusion always adds one rank more.`);
        if (up && byId.get(up)) say(`${nm(x3)} → ${byId.get(up).name}.`);
        return up;
      }
      const f1 = x1.d.fam, f2 = x2.d.fam, f3 = x3.d.fam, inR = (f) => f >= FAM.GOD && f <= FAM.BEAST;
      if (!(inR(f1) && inR(f2) && inR(f3))) { say('One of these demons is of a kind (such as a Fiend) that a normal tri-fusion doesn’t take.'); return 0; }
      const i1 = Math.min(f1, f2) - 2, i2 = Math.max(f1, f2) - 3, i3 = f3 - 2, flat = i1 * 56 + i2 * 8 + i3;
      const race = flat >= 0 && flat < triFamilyFlat.length ? triFamilyFlat[flat] : 0;
      if (!race) { say('The server’s tri-fusion table has no result for the two highest demons’ families.'); return 0; }
      const res = pickInRace(race, adj);
      say(`The two highest demons’ families and the third’s make a ${raceName(race)}.`);
      say(`Fusion level: (${x1.lvl} + ${x2.lvl} + ${x3.lvl}) ÷ 3 + 1${extra > 0 ? ' + 4 (a God-family demon is in)' : ''}${neutral ? ' − 4 (all three are Neutral)' : ''} = ${adj}${res ? ` → ${res.name} (level ${res.lvl})` : ''}.`);
      return res ? res.id : 0;
    }

    // ---- reverse: every recipe, worked out once over the demons the race chart can make, at their base levels ----
    const pool = demons.filter((d) => (d.opt & RANGE) !== 0 && !d.mitamaFused);
    let two = null, three = null;
    function buildAll() {
      if (two) return;
      const n = pool.length, at = (d) => ({ d, lvl: d.lvl });
      const res2 = [], res3 = new Int32Array(((n * (n + 1) * (n + 2)) / 6) | 0);
      // (a special fusion's pair is listed under "Special fusion", not again here: -1)
      const idOf = (r) => (r.special ? -1 : r.id);
      for (let i = 0; i < n; i++) for (let j = i; j < n; j++) res2.push(idOf(fuse([at(pool[i]), at(pool[j])], { log: false, plugins: false })));
      let k = 0;
      for (let i = 0; i < n; i++) for (let j = i; j < n; j++) for (let l = j; l < n; l++)
        res3[k++] = idOf(fuse([at(pool[i]), at(pool[j]), at(pool[l])], { log: false, plugins: false }));
      two = res2; three = res3;
    }
    /** Every way to make a demon: { special: [], two: [[a, b]], three: [[a, b, c]] } (demons, base levels). */
    function recipes(target) {
      buildAll();
      const n = pool.length, out = { special: specialsFor.get(target.id) || [], two: [], three: [] };
      let k = 0;
      for (let i = 0; i < n; i++) for (let j = i; j < n; j++) { if (two[k++] === target.id) out.two.push([pool[i], pool[j]]); }
      k = 0;
      for (let i = 0; i < n; i++) for (let j = i; j < n; j++) for (let l = j; l < n; l++) { if (three[k++] === target.id) out.three.push([pool[i], pool[j], pool[l]]); }
      const cost = (r) => r.reduce((s2, d) => s2 + d.lvl, 0);
      out.two.sort((x, y) => cost(x) - cost(y)); out.three.sort((x, y) => cost(x) - cost(y));
      return out;
    }
    // the pickers leave out Mitama-fused demons: the game won't fuse them, and the planner only makes them from a Mitama
    const pickable = demons.filter((d) => !d.mitamaFused);
    // the demons some fusion makes (special, two-demon or tri-fusion): the only ones "Find a recipe" offers
    let makeable = null;
    function canMake(d) {
      if (!makeable) {
        buildAll();
        makeable = new Set(F.specials.map((sp) => sp.result));
        for (const id of two) if (id > 0) makeable.add(id);
        for (const id of three) if (id > 0) makeable.add(id);
      }
      return makeable.has(d.id);
    }
    const targets = () => pickable.filter(canMake);
    return { F, demons, pickable, targets, canMake, byId, fuse, recipes, raceName, pool, ranges, built: () => !!two, build: buildAll, isElemental: (d) => d.race === ELEMENTAL_RACE };
  }

  /**
   * The easiest way to make every demon from scratch ("Fusion tree"). A demon you can simply go and get is a start: one
   * a spawn lets you contract all year, or one a Summon PG gives when an NPC trades that PG outside events (demon-db.json:
   * `contract`, `get`). Everything else is fused from those, and the planner keeps, per demon, the way whose starts are
   * the easiest: every start has a price (below), a fusion adds 1, a plugin a little more, and the lowest total wins.
   * Levels are free: a demon can be raised to any level with Magical Golden Apples, so a pair works when SOME levels at or
   * above their base levels give the result (the lowest such levels are the ones shown). Every way is checked with the
   * engine's own fuse() before it is kept. Normal tri-fusions are left out (every race the chart's demons belong to can be
   * made from two demons); special fusions of three are in.
   *   opt: { pg, events, have: Set of demon ids the player already has }
   */
  function fusionPlanner(E, db, opt) {
    const F = E.F, o = opt || {}, have = o.have || new Set();
    const ELEMENTAL_RACE = 55, MITAMA_RACE = 57, STEP = 1, RAISE = 0.005;
    const dbById = new Map(db.demons.map((x) => [x.id, x])), dbByKey = new Map(db.demons.map((x) => [`${x.name}|${x.race}|${x.level}`, x]));
    /** The Demon Database's entry for one of the engine's demons (its id, or the one the database merged it into). */
    const dbOf = (d) => dbById.get(d.id) || dbByKey.get(`${d.name}|${d.raceName}|${d.lvl}`) || null;
    const isEvent = (text) => /\bevent\b/i.test(text);
    /** How to get a demon without fusing it, or null: { kind: 'have' | 'contract' | 'pg', cost, ... }. */
    function sourceOf(d) {
      if (have.has(d.id)) return { kind: 'have', cost: 0.1 };
      const x = dbOf(d);
      if (!x) return null;
      let best = null;
      const take = (s) => { if (!best || s.cost < best.cost) best = s; };
      // a contract: the open world before a dungeon, a low level before a high one, any moon before a moon phase, and
      // the plain demon before a version of it (Accomplished, Deformed, Inexperienced ...: its own, rarer spawn)
      x.contract.forEach((c) => { if (!c.season || o.events) take({ kind: 'contract', cost: (c.field ? 1 : 1.5) + c.min / 200 + (c.moon ? 0.25 : 0) + (x.variant ? 0.4 : 0) + (c.season ? 1 : 0), place: c, places: x.contract }); });
      // a Summon PG (or voucher) an NPC trades: all year unless the line names an event
      (x.get || []).filter((g) => g.kind === 'Exchange item' && g.from && g.from.length).forEach((g) => {
        const npc = g.from.filter((f) => /^NPC exchange/.test(f) && !isEvent(f));
        if (npc.length && o.pg !== false) take({ kind: 'pg', cost: 1.5, text: g.text, from: npc, note: g.note || null });
        else if (o.events) take({ kind: 'pg', cost: 2.5, text: g.text, from: g.from, note: g.note || null, event: true });
      });
      return best;
    }

    const all = E.pickable, way = new Map(), costOf = (d) => { const w = way.get(d.id); return w ? w.cost : Infinity; };
    const src = new Map();
    all.forEach((d) => { const s = sourceOf(d); if (s) { src.set(d.id, s); way.set(d.id, { kind: s.kind, cost: s.cost, parts: [], src: s }); } });
    // the demons of each race that can be had so far, easiest first (rebuilt each round)
    let byRace = new Map(), byBase = new Map();
    function index() {
      byRace = new Map(); byBase = new Map();
      all.forEach((d) => {
        if (!way.has(d.id)) return;
        if (!byRace.has(d.race)) byRace.set(d.race, []); byRace.get(d.race).push(d);
        if (!byBase.has(d.base)) byBase.set(d.base, []); byBase.get(d.base).push(d);
      });
      const easy = (a, b) => costOf(a) - costOf(b) || a.lvl - b.lvl || a.id - b.id;
      byRace.forEach((l) => l.sort(easy)); byBase.forEach((l) => l.sort(easy));
    }
    const raceIds = F.raceOrder;
    const mapRace = (a, b) => { const i = raceIds.indexOf(a), j = raceIds.indexOf(b); return i < 0 || j < 0 ? null : F.raceMap[i][j]; };
    // which race pairs make each race (a pair once, unless the chart gives the two orders different results)
    const pairsFor = new Map();
    raceIds.forEach((ra, i) => raceIds.forEach((rb, j) => {
      const r = F.raceMap[i][j];
      if (!r || ra === rb || (j < i && F.raceMap[j][i] === r)) return;
      if (!pairsFor.has(r)) pairsFor.set(r, []); pairsFor.get(r).push([ra, rb]);
    }));
    const sameFor = new Map();   // elemental id -> the races two demons of which make it
    raceIds.forEach((r) => { const k = mapRace(r, r); if (k && F.elementals[k - 1]) { const id = F.elementals[k - 1]; if (!sameFor.has(id)) sameFor.set(id, []); sameFor.get(id).push(r); } });
    const specialsFor = new Map();
    F.specials.forEach((sp) => { if (!specialsFor.has(sp.result)) specialsFor.set(sp.result, []); specialsFor.get(sp.result).push(sp); });
    const pluginCost = (sp) => { if (!sp.plugin) return 0; const p = F.plugins[String(sp.plugin)]; return p && p.how ? 2 : 6; };
    /** The lowest levels, each at or above the demon's base level, that add up to `sum` (the lowest demon is raised first). */
    function levelsFor(ds, sum) {
      const l = ds.map((d) => d.lvl);
      let need = sum - l.reduce((s2, v) => s2 + v, 0);
      while (need > 0) {
        let k = -1;
        l.forEach((v, j) => { if (v < 99 && (k < 0 || v < l[k])) k = j; });
        if (k < 0) return null;
        l[k]++; need--;
      }
      return l;
    }
    const check = (parts, T, test) => { const r = E.fuse(parts, { log: false }); return r.id === T.id && test(r); };

    /**
     * The ways to make T from demons that can be had so far. `under`: only ways cheaper than this (the search stops
     * early); `every`: the best way of each kind and race pair, for the list of other ways (else only the best one);
     * `must`: only ways with a demon this says yes to (the list narrowed to a demon or race the player typed).
     */
    function waysOf(T, under, every, must) {
      const out = [];
      let cap = under === undefined ? Infinity : under;
      const keep = (w) => { out.push(w); if (!every && w.cost < cap) cap = w.cost; };
      const list = E.ranges.get(T.race), at = list ? list.indexOf(T) : -1;
      // special fusions: fixed demons (or any version of them, when the recipe allows it)
      (specialsFor.get(T.id) || []).forEach((sp) => {
        const opts = [];
        for (const [sid, variant] of sp.src) {
          const sd = E.byId.get(sid), c = variant ? (sd ? byBase.get(sd.base) : null) : (way.has(sid) && sd && !sd.mitamaFused ? [sd] : null);
          if (!c || !c.length) return;
          opts.push(must ? [...new Set(c.filter(must).slice(0, 3).concat(c.slice(0, 3)))] : c.slice(0, 3));
        }
        // the easiest version of each demon; when the game makes something else of those, the next easiest
        let best = null;
        const tryAll = (k, picked) => {
          if (k < opts.length) { opts[k].forEach((d) => tryAll(k + 1, picked.concat(d))); return; }
          if (must && !picked.some(must)) return;
          const cost = picked.reduce((s2, d) => s2 + costOf(d), 0) + STEP + pluginCost(sp);
          if (cost >= (best ? best.cost : cap)) return;
          const parts = picked.map((d, i) => ({ d, lvl: d.lvl, exact: !sp.src[i][1] }));
          if (check(parts, T, (r) => r.special === sp)) best = { kind: 'special', cost, parts, sp };
        };
        tryAll(0, []);
        if (best) keep(best);
      });
      // two demons of the same race make an elemental
      (sameFor.get(T.id) || []).forEach((r) => {
        const c0 = byRace.get(r) || [], c = must ? [...new Set(c0.filter(must).slice(0, 3).concat(c0.slice(0, 4)))] : c0.slice(0, 4);
        let best = null;
        for (let i = 0; i < c.length; i++) for (let j = i; j < c.length; j++) {
          if (must && !must(c[i]) && !must(c[j])) continue;
          const cost = costOf(c[i]) + costOf(c[j]) + STEP;
          if (cost >= (every ? (best ? best.cost : Infinity) : cap)) continue;
          const parts = [{ d: c[i], lvl: c[i].lvl }, { d: c[j], lvl: c[j].lvl }];
          if (check(parts, T, (x) => !x.special)) { best = { kind: 'same', cost, parts }; if (!every) cap = cost; }
        }
        if (best) keep(best);
      });
      // two different elementals make a Mitama
      if (T.race === MITAMA_RACE) F.elementals.forEach((e1, i) => F.elementals.forEach((e2, j) => {
        if (j <= i || F.mitamas[F.elemMitama[i][j]] !== T.id) return;
        const a = E.byId.get(e1), b = E.byId.get(e2), cost = costOf(a) + costOf(b) + STEP;
        const parts = [{ d: a, lvl: a.lvl }, { d: b, lvl: b.lvl }];
        if (must && !must(a) && !must(b)) return;
        if (cost < cap && check(parts, T, (x) => !x.special)) keep({ kind: 'mitama', cost, parts });
      }));
      if (at < 0) return out.sort((x, y) => x.cost - y.cost);
      // an elemental moves the demon next to it in its race one rank up or down
      const ri = raceIds.indexOf(T.race);
      if (ri >= 0) F.elementals.forEach((eid, ei) => {
        const dir = F.elemAdjust[ri][ei], near = dir === 1 ? list[at - 1] : dir ? list[at + 1] : null;
        const e = (byBase.get(eid) || []).find((d) => d.race === ELEMENTAL_RACE), others = near ? (byBase.get(near.id) || []).filter((d) => d.race === T.race) : [];
        const other = (must && others.find(must)) || others[0];
        if (!e || !other || (must && !must(e) && !must(other))) return;
        const cost = costOf(e) + costOf(other) + STEP, parts = [{ d: other, lvl: other.lvl }, { d: e, lvl: e.lvl }];
        if (cost < cap && check(parts, T, (x) => !x.special && x.elemental === dir)) keep({ kind: 'rank', cost, parts, dir });
      });
      // two races the chart turns into T's race, at levels that land on T: above the demon below it in its race, and not
      // past T (the race's highest demon has no upper limit)
      const top = at === list.length - 1, minSum = at > 0 ? 2 * list[at - 1].lvl : 0, maxSum = top ? 198 : 2 * T.lvl - 1;
      if (minSum <= maxSum) (pairsFor.get(T.race) || []).forEach(([ra, rb]) => {
        const A = byRace.get(ra), B = byRace.get(rb);
        if (!A || !B) return;
        let best = null, lim = every ? Infinity : cap;
        for (const a of A) {
          const ca = costOf(a);
          if (ca + costOf(B[0]) + STEP >= lim) break;
          for (const b of B) {
            const base = ca + costOf(b) + STEP;
            if (base >= lim) break;
            if (a.lvl + b.lvl > maxSum || (must && !must(a) && !must(b))) continue;
            const sum = Math.max(a.lvl + b.lvl, minSum), cost = base + RAISE * (sum - a.lvl - b.lvl);
            if (cost >= lim) continue;
            const lv = levelsFor([a, b], sum);
            if (!lv) continue;
            const parts = [{ d: a, lvl: lv[0] }, { d: b, lvl: lv[1] }];
            if (check(parts, T, (x) => !x.special && !x.elemental)) { best = { kind: 'two', cost, parts }; lim = cost; }
          }
        }
        if (best) { keep(best); }
      });
      return out.sort((x, y) => x.cost - y.cost);
    }

    // relax until nothing gets easier: a way only uses demons that are easier than what it makes, so this ends
    const targets = all.filter((d) => E.ranges.has(d.race) && E.ranges.get(d.race).includes(d) || sameFor.has(d.id) || d.race === MITAMA_RACE || specialsFor.has(d.id));
    for (let round = 0, changed = true; changed && round < 40; round++) {
      changed = false;
      index();
      targets.forEach((T) => {
        const now = costOf(T), w = waysOf(T, now)[0];
        if (w && w.cost < now - 1e-9) { way.set(T.id, w); changed = true; }
      });
    }
    index();
    /** Every way to get T, easiest first: its own source (contract, PG, "have it") and the best fusion of each kind.
     *  With `must`, only the fusions with a demon it says yes to (the best of each kind among those). */
    function ways(T, must) {
      const s = src.get(T.id), out = waysOf(T, undefined, true, must).filter((w) => w.parts.every((p) => p.d.id !== T.id));
      if (s && !must) out.push({ kind: s.kind, cost: s.cost, parts: [], src: s });
      return out.sort((x, y) => x.cost - y.cost);
    }
    // the demons the page lets you pick: the ones some fusion makes (also when no start leads to them). A demon that can
    // only be contracted or bought as a PG is not offered: there is nothing to plan, and it still shows inside trees
    const fusable = new Set([...targets.map((d) => d.id), ...F.triDark.map((r) => r[1]), ...F.triElemental.map((r) => r[5])]);
    return { way: (d) => way.get(d.id) || null, cost: costOf, ways, source: (d) => src.get(d.id) || null, dbOf, makeable: () => all.filter((d) => way.has(d.id)),
      fusable: (d) => fusable.has(d.id), listable: () => all.filter((d) => fusable.has(d.id)) };
  }

  /** A demon picker: a search field that lists matching demons under it (arrow keys, Enter, Esc). */
  function demonCombo(E, current, placeholder, onPick, list) {
    const wrap = h('div', { class: 'fus-combo' });
    const input = h('input', { class: 'input', type: 'search', placeholder, value: current ? current.label : '', 'aria-label': placeholder, spellcheck: 'false', autocomplete: 'off', role: 'combobox', 'aria-expanded': 'false' });
    const menu = h('div', { class: 'fus-menu', role: 'listbox', hidden: true });
    // its own clear button, in place of the search field's tiny built-in one: a full-height target that takes the pick
    // away at once (mousedown is swallowed so the field keeps the focus and its blur doesn't put the name back)
    const clear = h('button', { type: 'button', class: 'fus-clear', 'aria-label': 'Clear', 'data-tip': 'Clear', hidden: current ? null : true,
      onmousedown: (ev) => ev.preventDefault(),
      onclick: () => { input.value = ''; clear.hidden = true; if (current) { close(); onPick(null); } else { at = 0; input.focus(); open(); } } }, icon('x'));
    let hits = [], at = 0;
    const all = (list || E.pickable);
    const draw = () => {
      const q = input.value.trim().toLowerCase();
      const seen = new Set();
      hits = all.filter((d) => { const key = d.label + '|' + d.race + '|' + d.lvl; if (seen.has(key)) return false; seen.add(key); return !q || d.label.toLowerCase().includes(q) || d.raceName.toLowerCase().includes(q); })
        .sort((a, b) => { const qa = a.label.toLowerCase().startsWith(q) ? 0 : 1, qb = b.label.toLowerCase().startsWith(q) ? 0 : 1; return qa - qb || a.label.localeCompare(b.label) || a.lvl - b.lvl; })
        .slice(0, 60);
      at = Math.min(at, Math.max(0, hits.length - 1));
      menu.replaceChildren(...(hits.length ? hits.map((d, i) => h('div', { class: 'fus-opt' + (i === at ? ' on' : ''), role: 'option', 'aria-selected': String(i === at),
        onmousedown: (ev) => { ev.preventDefault(); pick(d); } }, demonPic(E, d), h('span', { class: 'fus-opt-name', text: d.label }), h('span', { class: 'src-count', text: `${d.raceName} · Lv ${d.lvl}` })))
        : [h('div', { class: 'fus-opt empty', text: 'No demon matches.' })]));
      const on = menu.querySelector('.fus-opt.on'); if (on) on.scrollIntoView({ block: 'nearest' });
    };
    const open = () => { menu.hidden = false; input.setAttribute('aria-expanded', 'true'); draw(); };
    const close = () => { menu.hidden = true; input.setAttribute('aria-expanded', 'false'); };
    const pick = (d) => { close(); input.value = d.label; clear.hidden = false; onPick(d); };
    input.addEventListener('focus', () => { input.select(); open(); });
    input.addEventListener('input', () => { at = 0; clear.hidden = !input.value; open(); });
    input.addEventListener('blur', () => { close(); if (!input.value.trim() && current) onPick(null); else if (current) input.value = current.label; });
    input.addEventListener('keydown', (ev) => {
      if (ev.key === 'ArrowDown') { ev.preventDefault(); if (menu.hidden) open(); else { at = Math.min(at + 1, hits.length - 1); draw(); } }
      else if (ev.key === 'ArrowUp') { ev.preventDefault(); at = Math.max(at - 1, 0); draw(); }
      else if (ev.key === 'Enter') { if (!menu.hidden && hits[at]) { ev.preventDefault(); pick(hits[at]); } }
      else if (ev.key === 'Escape') { if (!menu.hidden) { ev.stopPropagation(); close(); } }
    });
    wrap.append(h('div', { class: 'search has-clear' }, icon('search'), input, clear), menu);
    return wrap;
  }
  function demonPic(E, d, big) {
    const box = h('span', { class: 'dcm-icon' + (big ? ' fus-pic-big' : ''), 'aria-hidden': 'true' });
    const src = (d && d.icon) || E.F.noIcon;
    if (src) box.append(h('img', { src, alt: '', loading: 'lazy', onerror: (ev) => ev.target.remove() }));
    return box;
  }
  /** One demon as a small chip: picture, name, race and level; a click plans it (reverse view). */
  function demonChip(E, d, lvl, onclick, tag) {
    const kids = [demonPic(E, d), h('span', { class: 'fus-chip-text' }, h('span', { class: 'fus-chip-name', text: d.label }), h('span', { class: 'fus-chip-sub', text: `${d.raceName} · Lv ${lvl === undefined ? d.lvl : lvl}` })), tag || null];
    return onclick ? h('button', { type: 'button', class: 'fus-chip', 'data-tip': `How to make ${d.label}`, onclick: () => onclick(d) }, kids) : h('span', { class: 'fus-chip' }, kids);
  }
  /** Open the Quest Database on the quests with this title (searched for, and opened). */
  function openInQuestDb(title) {
    const D = S.quest, found = D.data ? D.data.quests.filter((q) => q.title === title) : [];
    D.q = title; D.type = found.length && found.every((q) => q.type === 'Inactive') ? 'Inactive' : 'available'; D.open = new Set(found.map((q) => q.id));
    go('quests');
  }
  /** A line that names a quest (Quest “Act 13: …”): the line, and a button that opens that quest in the Quest Database. */
  function questLine(text) {
    const m = /Quest “([^”]+)”/.exec(text || '');
    if (!m) return text;
    loadQuests();   // so the quest can be opened, not only searched for, when the button is clicked
    const title = m[1];
    return [text, h('button', { type: 'button', class: 'btn btn-sm q-link', onclick: () => openInQuestDb(title) }, 'Open in Quest Database')];
  }
  function pluginLine(E, sp) {
    if (!sp || !sp.plugin) return null;
    const p = E.F.plugins[String(sp.plugin)];
    // the game's key item names it ("Plugin: Uriel", "Enhanced Plugin: Skadi"); its ways to get it are joined with " · "
    const ways = p && p.how ? p.how.split(' · ') : ['The data does not say where to get it.'];
    // several ways are alternatives: say so, or they read as steps that all have to be done
    return h('div', { class: 'fus-plugin' }, icon('info'), h('div', null, h('b', { text: p && p.name ? `Needs the key item “${p.name}”` : 'Needs a fusion plugin' }),
      ways.length > 1 ? [h('div', { class: 'any-one', text: `Any one of these ${ways.length} ways gives it:` }), h('ul', null, ways.map((w) => h('li', null, questLine(w))))] : h('div', null, questLine(ways[0]))));
  }
  /** The same, short, for a recipe card: the key item's name, and its ways to get it in a tip (cards keep one size). */
  function pluginNote(E, sp) {
    if (!sp || !sp.plugin) return null;
    const p = E.F.plugins[String(sp.plugin)];
    const ways = p && p.how ? p.how.split(' · ') : ['The data does not say where to get it.'];
    return withTip(h('div', { class: 'fus-plugin fus-plugin-sm', tabindex: '0' }, icon('info'), h('span', { class: 'fus-plugin-name', text: p && p.name ? `Needs “${p.name}”` : 'Needs a fusion plugin' })),
      () => h('div', null, h('b', { text: p && p.name ? `How to get “${p.name}”${ways.length > 1 ? ' (any one of these)' : ''}` : 'Fusion plugin' }), h('ul', { class: 'fus-tip-ways' }, ways.map((w) => h('li', { text: w })))));
  }

  function pageFusion() {
    const U = S.fus, page = h('div', { class: 'page page-fusion' });
    loadFusion();
    page.append(pageHead('Pick two or three demons to see what they make, or pick the demon you want and get the whole tree to it from demons you can contract, with every other way to make it.',
      h('div', { class: 'seg', role: 'group', 'aria-label': 'Planner' },
        [['forward', 'Fuse demons'], ['tree', 'Fusion tree']].map(([id, label]) =>
        h('button', { type: 'button', 'aria-pressed': String(U.view === id), onclick: () => { U.view = id; store.set('fus-view', id); render(); } }, label)))));
    if (U.state !== 'ready') {
      page.append(h('p', { class: 'note', text: U.state === 'failed' ? 'The fusion data could not be loaded (fusion-data.json is missing beside the page).' : 'Loading the fusion data…' }));
      return page;
    }
    page.append(U.view === 'tree' ? fusionTree(U.eng) : fusionForward(U.eng));
    page.append(h('p', { class: 'note fus-foot', text: 'Demons, levels, races and special fusions: the IMAGINE client. The race chart and the tri-fusion and elemental rules: the server’s fusion code, which IMAGINE may have changed. Checked in game: two two-demon fusions (one the community chart lacks), a tri-fusion and the refusal of Mitama-fused demons, all as the planner says. Special fusions and elemental rank changes are not checked yet.' }));
    return page;
  }

  function fusionForward(E) {
    const U = S.fus, box = h('div', { class: 'fus-body' });
    const slots = h('div', { class: 'fus-slots' }), result = h('div', { class: 'fus-result-wrap' });
    const drawResult = () => {
      const xs = U.slots.filter((x) => x && x.d);
      result.replaceChildren();
      if (U.slots.filter(Boolean).length < 2 || xs.length < 2) { result.append(h('p', { class: 'note', text: 'Pick at least two demons. A third makes it a tri-fusion.' })); return; }
      const r = E.fuse(xs.map((x) => ({ d: x.d, lvl: x.lvl })));
      const card = h('section', { class: 'card fus-result' });
      const d = r.id && E.byId.get(r.id);
      card.append(h('div', { class: 'card-head' }, h('h3', { class: 'card-title', text: xs.length === 3 ? 'Tri-fusion result' : 'Fusion result' }),
        r.special ? h('span', { class: 'tag tag-digi', text: 'Special fusion' }) : r.elemental ? h('span', { class: 'tag ' + (r.elemental > 0 ? 'fus-up' : 'fus-down'), text: r.elemental > 0 ? 'Rank up' : 'Rank down' }) : null));
      if (d) card.append(h('div', { class: 'fus-out' }, demonPic(E, d, true), h('div', null, h('div', { class: 'fus-out-name', text: d.label }), h('div', { class: 'card-sub', text: `${d.raceName} · base level ${d.lvl}` })),
        d.mitamaFused ? null : h('button', { type: 'button', class: 'btn btn-sm', style: 'margin-left:auto', onclick: () => { fusOpenTree(d); render(); } }, 'Other ways to make it')));
      else card.append(h('p', { class: 'fus-none', text: 'Nothing: these demons can’t be fused together.' }));
      if (r.how.length) card.append(h('ul', { class: 'fus-how' }, r.how.map((t) => h('li', { text: t }))));
      if (r.special) {
        card.append(pluginLine(E, r.special));
        if (r.special.plugin) {
          const alt = E.fuse(xs.map((x) => ({ d: x.d, lvl: x.lvl })), { plugins: false }), ad = alt.id && E.byId.get(alt.id);
          card.append(h('p', { class: 'note', text: ad ? `Without that plugin they make ${ad.label} (${ad.raceName}, level ${ad.lvl}).` : 'Without that plugin they make nothing.' }));
        }
      }
      result.append(card);
    };
    const drawSlots = () => {
      slots.replaceChildren(...[0, 1, 2].map((i) => {
        const x = U.slots[i];
        if (i === 2 && !x && !U.third) return h('button', { type: 'button', class: 'fus-slot fus-add', onclick: () => { U.third = true; drawSlots(); const inp = slots.querySelectorAll('input[type=search]')[2]; if (inp) inp.focus(); } },
          icon('plus'), h('span', { text: 'Add a third demon (tri-fusion)' }));
        const slot = h('div', { class: 'fus-slot card' });
        slot.append(h('div', { class: 'card-head' }, h('span', { class: 'card-title', text: `Demon ${i + 1}` }),
          i === 2 ? h('button', { type: 'button', class: 'btn btn-ghost btn-sm', onclick: () => { U.slots[2] = null; U.third = false; drawSlots(); drawResult(); } }, 'Remove') : null));
        slot.append(demonCombo(E, x && x.d, 'Search a demon or race', (d) => { U.slots[i] = d ? { d, lvl: d.lvl } : null; drawSlots(); drawResult(); }));
        if (x && x.d) {
          const lvl = h('input', { class: 'input fus-lvl', type: 'number', min: 1, max: 99, value: x.lvl, 'aria-label': `Demon ${i + 1}’s level`,
            oninput: () => { const v = Math.max(1, Math.min(99, parseInt(lvl.value, 10) || x.d.lvl)); x.lvl = v; drawResult(); } });
          slot.append(h('div', { class: 'fus-picked' }, demonPic(E, x.d, true), h('div', { class: 'fus-picked-text' }, h('div', { class: 'fus-out-name', text: x.d.label }), h('div', { class: 'card-sub', text: `${x.d.raceName} · base level ${x.d.lvl}` })),
            h('label', { class: 'fus-lvl-box', 'data-tip': 'The demon’s level now. The result depends on it; a fresh demon is at its base level.' }, h('span', { text: 'Level' }), lvl)));
        }
        return slot;
      }));
    };
    drawSlots(); drawResult();
    box.append(slots, result);
    return box;
  }

  // ---- Fusion tree: the whole way to a demon from scratch (fusionPlanner), as a pyramid ----
  const FT_KIND = { two: 'Fuse two', tri: 'Tri-fusion', special: 'Special fusion', same: 'Same race', mitama: 'Two elementals', contract: 'Contract', pg: 'Summon PG', have: 'You have it' };
  const ftKind = (w) => (w.kind === 'rank' ? (w.dir === 1 ? 'Rank up' : 'Rank down') : FT_KIND[w.kind] || '');
  const ftSig = (w) => (w ? `${w.kind}:${w.parts.map((p) => `${p.d.id}@${p.lvl}`).join('+')}` : '');
  /** Where a start comes from, in a line: the contract place, the PG, or "you have it". */
  const ftSource = (s) => (s.kind === 'have' ? 'You already have it' : s.kind === 'contract' ? `Contract in ${s.place.zone} · ${ddbLv(s.place)}` : s.text.replace(/^Use /, ''));
  const ftPlural = (n, one, many) => `${fmt(n)} ${n === 1 ? one : many || one + 's'}`;

  /** The Fusion tree's two plans, worked out once per choice of starts: P with the demons the player says they have, P0
   *  without them (a demon they have covers one place in the tree; another one of it is planned from scratch). */
  function fusPlans(E) {
    const U = S.fus, V = U.tree, key = `${V.pg}|${V.events}|${[...V.have].sort((a, b) => a - b).join(',')}`;
    if (U.planKey !== key) {
      U.plan0 = fusionPlanner(E, S.ddb.data, { pg: V.pg, events: V.events });
      U.plan = V.have.size ? fusionPlanner(E, S.ddb.data, { pg: V.pg, events: V.events, have: V.have }) : U.plan0;
      U.planKey = key;
    }
    return { P: U.plan, P0: U.plan0 };
  }

  /** Open the Fusion tree on a demon with its details open: where the Demon Database and "Fuse demons" send you for
   *  every way to make it (the old "Find a recipe"). */
  function fusOpenTree(d) {
    const U = S.fus;
    Object.assign(U.tree, { target: d, sel: '0', shownFor: d.id, open: new Set(), wq: '', wmode: 'best', wst: { pg: 0 } });
    store.set('fus-tree-target', String(d.id)); U.view = 'tree'; store.set('fus-view', 'tree');
  }

  function fusionTree(E) {
    const U = S.fus, V = U.tree, box = h('div', { class: 'fus-body' });
    loadDemonDb();
    if (S.ddb.state !== 'ready') {
      box.append(h('p', { class: 'note', text: S.ddb.state === 'failed' ? 'The demon list could not be loaded (demon-db.json is missing beside the page): the tree needs it to know where demons can be contracted.' : 'Loading the demons…' }));
      return box;
    }
    if (V.target === undefined) { const id = num(store.get('fus-tree-target')); V.target = (id && E.byId.get(id)) || null; }
    const plan = () => fusPlans(E).P;
    let centred = false;
    const draw = () => {
      const old = box.querySelector('.ftree'), sx = old ? old.scrollLeft : 0, sy = old ? old.scrollTop : 0;
      const P = plan(), P0 = U.plan0, kids = [];
      if (V.target && !P.fusable(V.target)) V.target = null;   // a saved demon that no fusion makes is no longer offered
      const T = V.target;
      const chip = (on, label, tip, fn) => h('button', { type: 'button', class: 'chip-btn', 'aria-pressed': String(on), 'data-tip': tip, onclick: fn }, label);
      kids.push(h('div', { class: 'toolbar' },
        demonCombo(E, T, 'Which demon do you want?', (d) => { V.target = d; V.sel = null; V.open = new Set(); V.wq = ''; V.wmode = 'best'; V.wst.pg = 0; centred = false; store.set('fus-tree-target', d ? String(d.id) : ''); draw(); }, P.listable()),
        h('span', { class: 'q-key', text: 'Start from' }),
        h('div', { class: 'chips', role: 'group', 'aria-label': 'Demons the tree may start from' },
          h('span', { class: 'chip-btn ft-fixed', 'data-tip': 'Demons a spawn lets you contract all year: always used', text: 'Contracts' }),
          chip(V.pg, 'Summon PG demons', 'Demons a Summon PG gives, when an NPC trades that PG outside events', () => { V.pg = !V.pg; store.set('fus-tree-pg', V.pg ? '1' : '0'); draw(); }),
          chip(V.events, 'Event demons', 'Demons only an event gives: seasonal spawns, and PGs from event rewards or event exchanges', () => { V.events = !V.events; store.set('fus-tree-events', V.events ? '1' : '0'); draw(); }))));
      const done = () => {
        box.replaceChildren(...kids);
        const el = box.querySelector('.ftree');
        if (!el) return;
        el.scrollLeft = sx; el.scrollTop = sy;
        if (!centred) requestAnimationFrame(() => { if (el.scrollWidth > el.clientWidth) el.scrollLeft = (el.scrollWidth - el.clientWidth) / 2; centred = el.isConnected; });
        // drag the tree to move around in it (a drag is not a click on the demon under it)
        let from = null, moved = false;
        el.addEventListener('pointerdown', (ev) => { if (ev.button === 0) { from = { x: ev.clientX, y: ev.clientY, l: el.scrollLeft, t: el.scrollTop }; moved = false; } });
        el.addEventListener('pointermove', (ev) => {
          if (!from || !(ev.buttons & 1)) { from = null; return; }
          const dx = ev.clientX - from.x, dy = ev.clientY - from.y;
          if (!moved && Math.abs(dx) + Math.abs(dy) < 6) return;
          moved = true; el.classList.add('dragging'); el.scrollLeft = from.l - dx; el.scrollTop = from.t - dy;
        });
        const end = () => { from = null; el.classList.remove('dragging'); };
        el.addEventListener('pointerup', end); el.addEventListener('pointerleave', end);
        el.addEventListener('click', (ev) => { if (moved) { ev.stopPropagation(); ev.preventDefault(); moved = false; } }, true);
      };
      if (!T) {
        kids.push(h('p', { class: 'note', text: 'Pick a demon to get the whole way to it from scratch: which demons to contract (or get from a Summon PG), what to fuse in which order, and the levels to raise them to. At every step the tree takes the demons that are easiest to get.' }));
        return done();
      }
      // a way picked here (V.over, by demon) holds while its demons can still be had; one that no longer does is dropped
      const usable = (Q, d, w) => (w.parts.length ? w.parts.every((p) => Q.way(p.d)) && w : (() => { const s = Q.source(d); return s && s.kind === w.kind ? { kind: s.kind, cost: s.cost, parts: [], src: s } : null; })());

      // A demon's details: how it is made in the tree, and every way to get it, as two lists. "Easiest of each kind" is
      // the planner's (the best pair of each two races with levels raised where that helps, each special fusion,
      // getting it directly). "Every recipe" is the engine's sweep at base levels: every special fusion, two-demon
      // fusion and tri-fusion (worked out once, about a second). Both can be narrowed to a demon or race; a typed
      // demon gets its own best ways, not only the ones that happened to be best. This replaced the "Find a recipe" view.
      const panelFor = (sel, alone) => {
        const d = sel.d, w = sel.w, Q = sel.Q, has = V.have.has(d.id), own = Q.way(d);
        const total = new Map(), fusionsOf = (x) => { if (!total.has(x.id)) { const y = Q.way(x); total.set(x.id, y && y.parts.length ? 1 + y.parts.reduce((s2, p) => s2 + fusionsOf(p.d), 0) : 0); } return total.get(x.id); };
        const pick = (way) => { if (ftSig(way) === ftSig(own)) V.over.delete(d.id); else V.over.set(d.id, way); V.open = new Set(); draw(); };
        const panel = h('section', { class: 'card fus-sec ft-panel' });
        panel.append(h('div', { class: 'card-head' },
          h('div', { class: 'fus-out' }, demonPic(E, d, true), h('div', null, h('div', { class: 'fus-out-name', text: d.label }), h('div', { class: 'card-sub', text: `${d.raceName} · base level ${d.lvl}` }))),
          h('div', { class: 'toolbar' },
            V.over.has(d.id) ? h('button', { type: 'button', class: 'btn btn-sm', onclick: () => { V.over.delete(d.id); V.open = new Set(); draw(); } }, 'Back to the easiest way') : null,
            h('button', { type: 'button', class: 'btn btn-sm', 'aria-pressed': String(has), 'data-tip': has ? 'Plan it again from scratch' : 'The tree stops here for one of them, and uses it where that makes things easier',
              onclick: () => { if (has) V.have.delete(d.id); else { V.have.add(d.id); V.over.delete(d.id); } store.set('fus-have', JSON.stringify([...V.have])); V.open = new Set(); draw(); } }, has ? 'I don’t have it after all' : 'I already have it'),
            alone ? null : h('button', { type: 'button', class: 'btn btn-ghost btn-sm', 'aria-label': 'Close', onclick: () => { V.sel = null; draw(); } }, 'Close'))));
        if (w) {
          const lines = [];
          if (w.kind === 'have') lines.push('You marked it as a demon you already have, so the tree stops here.');
          else if (w.kind === 'contract') lines.push(`Contract it: ${w.src.places.filter((c) => !c.season || V.events).map((c) => `${c.zone} (${ddbLv(c)}${c.moon ? `, ${ddbMoon(c.moon)}` : ''}${c.season ? `, ${c.season} event` : ''})`).join(' · ')}.`);
          else if (w.kind === 'pg') { lines.push(`${w.src.text}.`); lines.push(`${w.src.from.length > 1 ? 'The item comes from any one of these: ' : 'The item: '}${w.src.from.join(' · ')}.`); if (w.src.note) lines.push(w.src.note); }
          else { lines.push(`In this tree: ${w.parts.map((p) => `${p.d.label} (level ${p.lvl})`).join(' + ')}.`); E.fuse(w.parts.map((p) => ({ d: p.d, lvl: p.lvl }))).how.forEach((t) => lines.push(t)); }
          if (sel.need) lines.push(`It comes at level ${d.lvl}: raise it to level ${sel.need} before the fusion above it (Magical Golden Apples).`);
          panel.append(h('ul', { class: 'fus-how' }, lines.map((t) => h('li', { text: t }))));
          if (w.sp) panel.append(pluginLine(E, w.sp));
        }

        // every recipe at base levels as ways the tree can take, easiest ingredients first; a special fusion is shown
        // with the easiest version of each demon when the planner found one
        let every = null;
        const allRecipes = () => {
          if (every) return every;
          const R = E.recipes(d), at = (x) => ({ d: x, lvl: x.lvl }), best = Q.ways(d);
          const kindOf = (r) => { const f = E.fuse(r.map(at), { log: false, plugins: false }); return f.elemental ? { kind: 'rank', dir: f.elemental } : { kind: d.race === 57 ? 'mitama' : E.isElemental(d) ? 'same' : 'two' }; };
          const cost = (x) => x.parts.reduce((s2, p) => s2 + Q.cost(p.d), 0), lv = (x) => x.parts.reduce((s2, p) => s2 + p.d.lvl, 0);
          every = [
            ...R.special.map((sp) => best.find((x) => x.sp === sp) || { kind: 'special', sp, parts: sp.src.map(([id, variant]) => ({ d: E.byId.get(id), exact: !variant })).filter((p) => p.d).map((p) => Object.assign(p, { lvl: p.d.lvl })) }),
            ...R.two.map((r) => Object.assign(kindOf(r), { parts: r.map(at) })),
            ...R.three.map((r) => ({ kind: 'tri', parts: r.map(at) })),
          ].filter((x) => x.parts.every((p) => p.d.id !== d.id));
          every.forEach((x) => { x.ease = cost(x); x.lv = lv(x); });
          every.sort((x, y) => (x.ease === y.ease ? 0 : x.ease - y.ease) || x.lv - y.lv);
          return every;
        };
        const startTag = (x) => { const y = Q.way(x); return y && !y.parts.length ? h('span', { class: 'tag tag-digi', 'data-tip': ftSource(y.src), text: y.kind === 'pg' ? 'PG' : ftKind(y) }) : null; };
        const wayCard = (way) => {
          const cur = !!w && ftSig(way) === ftSig(w), lost = way.parts.filter((p) => !Q.way(p.d));
          const n = way.parts.length && !lost.length ? 1 + way.parts.reduce((s2, p) => s2 + fusionsOf(p.d), 0) : 0;
          return h('li', { class: 'fus-rcard' + (cur ? ' ft-cur' : '') },
            h('div', { class: 'fus-rcard-head' }, h('span', { class: 'tag' + (way.parts.length ? '' : ' tag-digi'), text: ftKind(way) }),
              lost.length ? null : h('span', { class: 'src-count fus-sum', 'data-tip': 'Fusions in all, counting the ones that make its demons', text: n ? ftPlural(n, 'fusion') : 'No fusion' })),
            way.parts.length ? way.parts.map((p, i) => [i ? h('span', { class: 'fus-plus', 'aria-hidden': 'true', text: '+' }) : null, demonChip(E, p.d, p.lvl, null, startTag(p.d))])
              : h('div', { class: 'ft-way-src', text: `${ftSource(way.src)}${way.src.kind === 'pg' && way.src.from[0] ? ` · ${way.src.from[0]}` : ''}` }),
            pluginNote(E, way.sp),
            cur ? h('span', { class: 'ft-inuse' }, icon('check'), 'In the tree')
              : lost.length ? h('span', { class: 'ft-lost', tabindex: '0', 'data-tip': `Nothing you can contract or get from a Summon PG leads to ${lost.map((p) => p.d.label).join(' or ')} with the starts chosen above, so the tree can’t take this recipe.`, text: `No way to ${lost[0].d.label}` })
                : h('button', { type: 'button', class: 'btn btn-sm ft-use', onclick: () => pick(way) }, 'Use this way'));
        };
        // a demon nothing leads to has no "easiest" list: its recipes are what there is to show
        if (V.wmode !== 'all' && !Q.ways(d).length) V.wmode = 'all';
        const list = h('div', { class: 'fus-body' }), pager = h('nav', { class: 'pager', 'aria-label': 'Pages' }), count = h('span', { class: 'src-count' });
        const drawWays = () => {
          const q = (V.wq || '').trim().toLowerCase(), must = q ? (x) => x.label.toLowerCase().includes(q) || x.raceName.toLowerCase().includes(q) : null;
          let ways;
          if (V.wmode === 'all') {
            if (!E.built()) {
              list.replaceChildren(h('p', { class: 'note', role: 'status' }, h('span', { class: 'spin ink', style: 'margin-right:8px;vertical-align:-2px' }), 'Working out every fusion once (a second or two)…'));
              pager.replaceChildren(); count.textContent = '';
              setTimeout(() => { E.build(); if (list.isConnected) drawWays(); }, 30);
              return;
            }
            ways = allRecipes();
            if (must) ways = ways.filter((x) => x.parts.some((p) => must(p.d)));
          } else ways = Q.ways(d, must);
          count.textContent = fmt(ways.length);
          const pg = pageOf(V.wst, ways);
          list.replaceChildren(ways.length ? h('ul', { class: 'fus-recipes' }, pg.rows.map(wayCard))
            : h('p', { class: 'note', text: q ? `None with “${V.wq.trim()}”.` : V.wmode === 'all' ? `No fusion makes ${d.label}.` : 'No other way.' }));
          if (ways.length > pg.per) drawPager(pager, V.wst, ways.length, pg, drawWays, () => list, V.wmode === 'all' ? 'recipes' : 'ways'); else pager.replaceChildren();
        };
        const modes = [['best', 'Easiest of each kind', 'The best pair of each two races, with levels raised where that helps, each special fusion, and getting it directly'],
          ['all', 'Every recipe', 'Every special fusion, two-demon fusion and tri-fusion that makes it at base levels, the ones from demons that are easy to get first']];
        panel.append(h('div', { class: 'card-head' }, h('h3', { class: 'card-title' }, 'Every way to get it', count),
            h('span', { class: 'card-sub', text: V.wmode === 'all' ? 'At base levels, tri-fusions included; easiest demons first.' : 'Easiest first, with levels raised where that helps.' })),
          h('div', { class: 'toolbar' }, searchBox(V.wq || '', 'Only ways with this demon or race', (v) => { V.wq = v; V.wst.pg = 0; drawWays(); }),
            h('div', { class: 'chips', role: 'group', 'aria-label': 'Which ways' }, modes.map(([id, label, tip]) =>
              h('button', { type: 'button', class: 'chip-btn', 'aria-pressed': String(V.wmode === id), 'data-tip': tip, onclick: () => { V.wmode = id; V.wst.pg = 0; draw(); } }, label)))),
          list, pager);
        drawWays();
        return panel;
      };

      // no start leads to it (and no way picked here does): say so, and still show what fuses into it
      const rootPick = V.over.get(T.id);
      if (!P.way(T) && !(rootPick && usable(P, T, rootPick))) {
        kids.push(h('p', { class: 'notice' }, icon('info'), h('span', { text: `Nothing you can contract or get from a Summon PG leads to ${T.label} with these choices. Switch on “Summon PG demons” or “Event demons” above, or it only comes from a quest, an event or a gift. Its recipes are below.` })));
        kids.push(panelFor({ d: T, w: null, Q: P, need: 0 }, true));
        return done();
      }

      // the tree: every demon made the planner's way, or the way picked for it here. A demon met again inside its own
      // tree is made the planner's way from there down (a picked way could loop)
      const byPath = new Map(), used = new Set();
      let count = 0;
      const build = (Q, d, need, path, anc, free) => {
        const ov = !free && V.over.get(d.id);
        let w = (ov && usable(Q, d, ov)) || Q.way(d);
        // the one the player has is used once; another one is made from scratch, when it can be
        if (w.kind === 'have') { if (used.has(d.id) && P0.way(d)) { Q = P0; w = (ov && usable(Q, d, ov)) || Q.way(d); } else used.add(d.id); }
        const n = { d, need: need > d.lvl ? need : 0, w, path, Q, kids: [], picked: ftSig(w) !== ftSig(Q.way(d)) };
        byPath.set(path, n);
        if (w.parts.length && ++count < 500) { const a2 = new Set(anc).add(d.id); n.kids = w.parts.map((p, i) => build(Q, p.d, p.lvl, `${path}.${i}`, a2, free || a2.has(p.d.id))); }
        return n;
      };
      const root = build(P, T, 0, '0', new Set(), false);
      // a demon you simply go and get has no tree: open its details (where to get it, and the ways to fuse it instead)
      if (!root.kids.length && V.shownFor !== T.id) V.sel = '0';
      V.shownFor = T.id;

      // what the whole tree needs: the starts (by demon and level to raise it to), the fusions in order, the plugins
      const starts = new Map(), steps = new Map(), plugins = new Map();
      let fusions = 0;
      (function walk(n) {
        n.kids.forEach(walk);
        if (!n.kids.length) { const k = `${n.d.id}|${n.need}`, s = starts.get(k); if (s) s.n++; else starts.set(k, { n: 1, node: n }); return; }
        fusions++;
        const k = `${n.d.id}|${ftSig(n.w)}`, s = steps.get(k);
        if (s) s.n++; else steps.set(k, { n: 1, node: n });
        if (n.w.sp && n.w.sp.plugin) plugins.set(n.w.sp.plugin, n.w.sp);
      })(root);
      const startList = [...starts.values()].sort((a, b) => a.node.w.kind.localeCompare(b.node.w.kind) || a.node.d.label.localeCompare(b.node.d.label) || a.node.need - b.node.need);
      const toGet = startList.reduce((s2, x) => s2 + (x.node.w.kind === 'have' ? 0 : x.n), 0), owned = startList.reduce((s2, x) => s2 + (x.node.w.kind === 'have' ? x.n : 0), 0);

      const mini = (d, lvl) => h('span', { class: 'ft-mini' }, demonPic(E, d), h('span', { class: 'ft-mini-name', text: d.label }), lvl ? h('span', { class: 'ft-mini-lv', text: `Lv ${lvl}` }) : null);
      kids.push(h('div', { class: 'ft-top' },
        h('div', { class: 'fus-out' }, demonPic(E, T, true), h('div', null, h('div', { class: 'fus-out-name', text: T.label }), h('div', { class: 'card-sub', text: `${T.raceName} · base level ${T.lvl}` }))),
        h('div', { class: 'ft-counts' },
          fusions ? h('span', { class: 'ft-count' }, h('b', { text: fmt(fusions) }), fusions === 1 ? ' fusion' : ' fusions') : null,
          toGet ? h('span', { class: 'ft-count' }, h('b', { text: fmt(toGet) }), toGet === 1 ? ' demon to get' : ' demons to get') : null,
          owned ? h('span', { class: 'ft-count' }, h('b', { text: fmt(owned) }), ' you have') : null,
          plugins.size ? h('span', { class: 'ft-count' }, h('b', { text: fmt(plugins.size) }), plugins.size === 1 ? ' plugin' : ' plugins') : null)));

      // what to get first: made here, shown last (a big tree's list is long, and the tree is what the page is for)
      const sourceTip = (s) => () => h('div', null,
        s.kind === 'contract' ? [h('b', { text: 'Where it can be contracted' }), h('ul', { class: 'fus-tip-ways' }, s.places.filter((c) => !c.season || V.events).map((c) => h('li', { text: `${c.zone} · ${ddbLv(c)}${c.moon ? ` · ${ddbMoon(c.moon)}` : ''}${c.season ? ` · ${c.season} event` : ''}` })))]
          : s.kind === 'pg' ? [h('b', { text: s.from.length > 1 ? 'The item comes from any one of these' : 'Where the item comes from' }), h('ul', { class: 'fus-tip-ways' }, s.from.map((f) => h('li', { text: f }))), s.note ? h('div', { text: s.note }) : null]
            : h('span', { text: 'You marked it as a demon you already have. Click it in the tree to change that.' }));
      const gather = h('ul', { class: 'ft-gather' }, startList.map(({ n, node }) => {
        const s = node.w.src;
        return withTip(h('li', { class: 'ft-get' + (s.kind === 'have' ? ' have' : ''), tabindex: '0' }, demonPic(E, node.d),
          h('span', { class: 'fus-chip-text' }, h('span', { class: 'fus-chip-name', text: `${n > 1 ? `${n} × ` : ''}${node.d.label}` }), h('span', { class: 'fus-chip-sub', text: s.kind === 'have' && n > 1 ? `You have one: the tree needs ${n}` : `${ftSource(s)}${s.kind === 'pg' && s.from[0] ? ` · ${s.from[0]}` : ''}` })),
          node.need ? h('span', { class: 'tag ft-raise', 'data-tip': `It comes at level ${node.d.lvl}: raise it to ${node.need} before fusing it (Magical Golden Apples)`, text: `Raise to Lv ${node.need}` }) : null), sourceTip(s));
      }));
      const gatherCard = root.kids.length ? h('section', { class: 'card fus-sec' },
        h('div', { class: 'card-head' }, h('h3', { class: 'card-title' }, 'Get these first', h('span', { class: 'src-count', text: String(startList.reduce((s2, x) => s2 + x.n, 0)) })), h('span', { class: 'card-sub', text: 'The easiest demons that lead there. Hover one for every place.' })),
        gather, [...plugins.values()].map((sp) => pluginLine(E, sp))) : null;

      // 1. the pyramid. A demon made the same way as one already drawn is folded: its ingredients are the same, so it
      // only gets a small card with their pictures (the one highest up, then leftmost, is the one drawn in full)
      const drawn = new Set(), foldedAt = new Set(), openedAt = new Set();
      for (let row = [root]; row.length;) {
        const next = [];
        row.forEach((n) => {
          if (!n.kids.length) return;
          if (drawn.has(n.d.id) && !V.open.has(n.path)) { foldedAt.add(n.path); return; }
          if (drawn.has(n.d.id)) openedAt.add(n.path);
          drawn.add(n.d.id); next.push(...n.kids);
        });
        row = next;
      }
      const nodeEl = (n) => {
        const w = n.w, start = !w.parts.length, folded = foldedAt.has(n.path);
        const btn = h('button', { type: 'button', class: 'ft-node' + (start ? ' start' : '') + (w.kind === 'have' ? ' have' : '') + (V.sel === n.path ? ' on' : ''), 'aria-pressed': String(V.sel === n.path),
          'data-tip': `${n.d.label}: ${start ? ftSource(w.src) : w.parts.map((p) => `${p.d.label} (Lv ${p.lvl})`).join(' + ')}. Click for other ways.`,
          onclick: () => { V.sel = V.sel === n.path ? null : n.path; V.wq = ''; V.wmode = 'best'; V.wst.pg = 0; draw(); if (V.sel) { const p = box.querySelector('.ft-panel'); if (p) p.scrollIntoView({ block: 'nearest', behavior: 'smooth' }); } } },
          demonPic(E, n.d), h('span', { class: 'fus-chip-text' }, h('span', { class: 'fus-chip-name', text: n.d.label }), h('span', { class: 'fus-chip-sub', text: `Lv ${n.d.lvl} · ${ftKind(w)}` })));
        const marks = [n.need ? h('span', { class: 'tag ft-raise', 'data-tip': `Raise it to level ${n.need} before this fusion (it starts at ${n.d.lvl})`, text: `Raise to Lv ${n.need}` }) : null,
          w.sp && w.sp.plugin ? h('span', { class: 'tag tag-warn', 'data-tip': 'This special fusion needs a plugin (a key item): see “Get these first”', text: 'Plugin' }) : null,
          n.picked ? h('span', { class: 'tag tag-off', 'data-tip': 'You picked this way; the tree’s own choice was another', text: 'Your pick' }) : null,
          openedAt.has(n.path) ? h('button', { type: 'button', class: 'tag ft-refold', 'data-tip': `Made the same way as the other ${n.d.label}: fold this one again`, onclick: () => { V.open.delete(n.path); draw(); } }, 'Fold') : null].filter(Boolean);
        const li = h('li', null, btn, marks.length ? h('span', { class: 'ft-marks' }, marks) : null);
        // folded: a compact card with only the pictures of what it is fused from (a purple ring = a demon to get)
        if (folded) li.append(h('button', { type: 'button', class: 'ft-fold', 'aria-label': `Same as the other ${n.d.label}: ${n.kids.map((k) => k.d.label).join(' + ')}. Show them here too.`,
          'data-tip': `Same as the other ${n.d.label}: ${n.kids.map((k) => `${k.d.label} (Lv ${k.need || k.d.lvl})`).join(' + ')}. Click to show them here too.`, onclick: () => { V.open.add(n.path); draw(); } },
          n.kids.map((k, i) => [i ? h('span', { class: 'fus-plus', 'aria-hidden': 'true', text: '+' }) : null, h('span', { class: 'ft-orb' + (k.kids.length ? '' : ' start') }, demonPic(E, k.d))])));
        else if (n.kids.length) li.append(h('ul', null, n.kids.map(nodeEl)));
        return li;
      };
      kids.push(h('section', { class: 'card fus-sec' },
        h('div', { class: 'card-head' }, h('h3', { class: 'card-title', text: 'Fusion tree' }), h('span', { class: 'card-sub' }, h('span', { class: 'ft-key' }), 'fuse', h('span', { class: 'ft-key start' }), 'get it')),
        h('div', { class: 'ftree', tabindex: '0', role: 'group', 'aria-label': `How to make ${T.label}` }, h('div', { class: 'ftree-in' }, h('ul', { style: `zoom:${V.zoom}` }, nodeEl(root)))),
        h('div', { class: 'toolbar ft-zoom' }, h('span', { class: 'q-key', text: 'Size' }),
          h('div', { class: 'seg', role: 'group', 'aria-label': 'Size of the tree' }, [1, 0.75, 0.5].map((z) => h('button', { type: 'button', 'aria-pressed': String(V.zoom === z), onclick: () => { V.zoom = z; store.set('fus-tree-zoom', String(z)); centred = false; draw(); } }, `${z * 100}%`))),
          h('span', { class: 'note', text: 'Drag the tree to move around in it.' }))));

      // the clicked demon: how it is made here, and every way to get it
      const sel = V.sel && byPath.get(V.sel);
      if (sel) kids.push(panelFor(sel));

      // 2. the fusions, in the order to do them (ingredients first)
      if (fusions) kids.push(h('section', { class: 'card fus-sec' },
        h('div', { class: 'card-head' }, h('h3', { class: 'card-title' }, 'Fusions in order', h('span', { class: 'src-count', text: String(fusions) })), h('span', { class: 'card-sub', text: 'Each demon is at the level shown when it goes in' })),
        h('ol', { class: 'ft-steps' }, [...steps.values()].map(({ n, node }) => h('li', null,
          node.w.parts.map((p, i) => [i ? h('span', { class: 'fus-plus', 'aria-hidden': 'true', text: '+' }) : null, mini(p.d, p.lvl)]),
          h('span', { class: 'ft-arrow', 'aria-label': 'makes', text: '→' }), mini(node.d),
          n > 1 ? h('span', { class: 'tag', 'data-tip': `The tree needs ${n} of them: do this fusion ${n} times`, text: `× ${n}` }) : null,
          node.w.kind !== 'two' ? h('span', { class: 'tag' + (node.w.kind === 'special' ? ' tag-digi' : ''), text: ftKind(node.w) }) : null,
          node.w.sp && node.w.sp.plugin ? h('span', { class: 'tag tag-warn', text: (E.F.plugins[String(node.w.sp.plugin)] || {}).name || 'Plugin' }) : null)))));
      // 3. what to get first, and the plugins
      if (gatherCard) kids.push(gatherCard);
      kids.push(h('p', { class: 'note', text: 'How the tree chooses: a demon you can contract in the open world is the easiest start, then one in a dungeon, a Summon PG an NPC trades, and a rarer version of a demon (Accomplished, Deformed, Inexperienced …); low levels before high ones. Fewer fusions win, and a plugin counts against a special fusion. Levels are free: it assumes you raise demons with Magical Golden Apples, and shows the lowest levels that work. A contracted or fused demon is taken at its base level. Normal tri-fusions are not used (two demons can make every race); special fusions of three are. Contract places and PG trades are the public server data, which NewMoon may have changed.' }));
      done();
    };
    draw();
    return box;
  }

  // ---------- Tools: Rebirth Planner ----------
  // rebirth-planner.json is made by "imagine-reader rebirthplanner" (crates\imagine\src\build\rebirth_planner.rs) from the
  // server's growth, rebirth and Mitama tables. The rules followed here are the server's (comp_hack):
  //  - a rebirth (ReunionDemon) raises one rune's rank by one and sends the demon to level 1. Rank r needs level
  //    r x 10 + 10 (rank 9: 99), costs rank x 500 x level macca and ONE of the growth type's cost items. Entering another
  //    rune is a rebirth into its rank 1 (no level, no item); a demon still on its own default growth can go straight to
  //    rank 2 of that rune. Each rank above 1 adds the rune's bonus (AdjustDemonBaseStats).
  //  - levels come from Magical Golden Apples at Vivian: 10,000 XP each (the public server data), times the demon's
  //    XP bonus; a Mitama-fused demon gets 60% less XP (its rank bonus says so).
  //  - Mitama fusion needs 48 rank points (each rune counts up to 8; the default rune counts 1 while it is 0); the demon
  //    becomes its Mitama form, its ranks reset, and its Mitama rank is (points - 48) / 4. Each rune then holds as many
  //    Mitamas as its new rank (8 at most); each kind at most 12 + Mitama rank per demon (MitamaReunion.cpp).
  //  - a slot's bonus is random (one of the kind's 18); a set is on when the rune holds enough of each kind; the Mitama
  //    rank bonus scales with level: round(level / 99 x ceil(1.25 b^2 + b)). Stats: GetDemonBaseStats on the growth type
  //    the demon ENDS on, rank bonuses, Mitama rank and slot base stats, the HP/MP formula and dependent stats, then the
  //    other Mitama parts and the sets (AdjustMitamaStats modes 1 and 2, as calc\digitalize.rs applies them).
  const RBP_STAT_IDX = { 0: 0, 1: 1, 2: 2, 3: 3, 4: 4, 5: 5, 6: 6, 7: 7, 12: 8, 13: 9, 14: 10, 15: 11, 19: 12, 20: 13 };   // CorrectTbl id -> demon-db stats index
  const RBP_PRESETS = [4, 6, 8];
  function loadRebirth() {
    if (S.rbp.state === 'idle') {
      S.rbp.state = 'loading';
      fetch('rebirth-planner.json', { cache: 'no-store' }).then((r) => (r.ok ? r.json() : null)).catch(() => null).then((d) => {
        S.rbp.P = d; S.rbp.state = d ? 'ready' : 'failed';
        if (d) { d.runeOf = new Map(); d.runes.forEach((r, i) => { r.own.forEach((g) => d.runeOf.set(g, i)); r.ranks.forEach((k) => { if (k) d.runeOf.set(k.growth, i); }); }); }
        if (S.page === 'rebirth') render();
      });
    }
    loadDemonDb(); loadFusion(); loadDcm();   // the DCM list: which demons a DCM gives (ddbCanOwn)
    loadSkillDb();   // the Skills step: every skill a demon can get
    if (!S.rbp.fxState) {   // what each passive, feature and part does (rbp-effects.json; the app's copy is made with the sealed data)
      S.rbp.fxState = 'loading';
      fetch('rbp-effects.json', { cache: 'no-store' }).then((r) => (r.ok ? r.json() : null)).catch(() => null).then((d) => {
        S.rbp.fx = d && d.skills ? d.skills : null; S.rbp.fxState = S.rbp.fx ? 'ready' : 'failed';
        if (S.page === 'rebirth') render();
      });
    }
    if (S.rbp.compState === 'idle' && !S.rbp.comp) {   // the COMP demons as last seen (no look at the game: the Overview's picker does that)
      S.rbp.compState = 'loading';
      host.call('compDemons', { cached: true }).then((list) => { S.rbp.comp = Array.isArray(list) ? list : []; S.rbp.compState = 'ready'; if (S.page === 'rebirth') render(); })
        .catch(() => { S.rbp.comp = []; S.rbp.compState = 'ready'; if (S.page === 'rebirth') render(); });
    }
  }
  /** The plan as kept between visits: targets, the end path, the Mitama fusion and the slots. One plan, whatever the demon. */
  function rbpPlan() {
    if (!S.rbp.plan) {
      let p = null;
      try { p = JSON.parse(store.get('rbp-plan') || 'null'); } catch (e) { p = null; }
      S.rbp.plan = rbpPlanFrom(p);
    }
    return S.rbp.plan;
  }
  /** A plan made safe to use from anything kept or shared (a build, the unsaved plan, a pasted code): every field checked, the rest dropped. */
  function rbpPlanFrom(p) {
    if (!p || typeof p !== 'object') p = null;
    {
      const ranks = (v) => (Array.isArray(v) && v.length === 12 ? v.map((x) => Math.max(0, Math.min(9, n0(x)))) : Array(12).fill(0));
      const plan = { level: p && p.levelSet && p.level ? Math.max(1, Math.min(99, n0(p.level))) : 93, levelSet: !!(p && p.levelSet), xpBonus: p ? n0(p.xpBonus) : 0, pre: ranks(p && p.pre), last: p ? n0(p.last) : 0,
        fuse: !!(p && p.fuse), kind: p && p.kind >= 1 && p.kind <= 4 ? p.kind : 1, boost: !!(p && p.boost), post: ranks(p && p.post), postLast: p ? n0(p.postLast) : 0,
        slots: {}, view: p && p.view === 'boosts' ? 'boosts' : 'runes', picks: p && Array.isArray(p.picks) ? p.picks.map(n0).filter((v) => v >= 0 && v < 48) : [], kindAuto: !(p && p.kindAuto === false),
        boostPre: ranks(p && p.boostPre), boostPost: ranks(p && p.boostPost), itemKind: p && RBP_ITEM_KINDS.some(([k]) => k === p.itemKind) ? p.itemKind : 'rune',
        skills: p && p.skills && typeof p.skills === 'object' && Array.isArray(p.skills.slots) ? { key: String(p.skills.key || ''), slots: p.skills.slots.slice(0, 8).map(n0), touched: !!p.skills.touched } : null,
        equip: p && p.equip && typeof p.equip === 'object' && Array.isArray(p.equip.slots) ? { key: String(p.equip.key || ''), slots: p.equip.slots.slice(0, 4).map(n0), touched: !!p.equip.touched } : null,
        force: p && p.force && typeof p.force === 'object' && Array.isArray(p.force.slots) ? { key: String(p.force.key || ''), slots: p.force.slots.slice(0, 8).map((v) => Math.max(0, n0(v))), touched: !!p.force.touched } : null };   // ranks the player raised by hand in the boosts view, over what the boosts need; the skill bar, the equipment and the Demon Force bar (for which demon)
      // the slots per rune: counts per kind (an older plan's set + extras becomes counts)
      if (p && p.slots && typeof p.slots === 'object') Object.entries(p.slots).forEach(([i, sl]) => { if (/^(?:[0-9]|1[01])$/.test(i) && sl && Array.isArray(sl.counts)) plan.slots[i] = { counts: sl.counts.slice(0, 4).map((v) => Math.max(0, Math.min(8, n0(v)))) }; });
      plan.last = Math.max(0, Math.min(11, plan.last)); plan.postLast = Math.max(0, Math.min(11, plan.postLast));
      return plan;
    }
  }
  /**
   * Keeps what is in the planner. The unsaved plan (and the demon it is for) goes to the page's storage; a saved build
   * keeps every change in its own entry of "rbp-builds", as it is made.
   */
  function rbpSave() {
    const U = S.rbp;
    if (U.id) {
      const b = rbpBuilds().find((x) => x.id === U.id);
      if (b) { Object.assign(b, rbpBuildOf(), { saved_at: new Date().toISOString() }); rbpStoreBuilds(); U.saveState = 'saved'; rbpStatus(); }
      return;
    }
    store.set('rbp-plan', JSON.stringify(rbpPlan()));
    store.set('rbp-src', U.src);
    store.set('rbp-demon', U.demonId === null || U.demonId === undefined ? '' : String(U.demonId));
    store.set('rbp-comp', U.compKey || '');
    store.set('rbp-snap', U.compSnap ? JSON.stringify(U.compSnap) : '');
  }
  /** The chosen demon: its Demon Database entry, whether it is already Mitama-fused, and its state (level, ranks, Mitama). */
  function rbpDemon() {
    const U = S.rbp;
    return rbpDemonOf(U.src, U.src === 'comp' ? U.compSnap || (U.comp || []).find((q) => q.key === U.compKey) : null, U.demonId);   // an imported demon is kept as it was when imported
  }
  /** The same for any build: where the demon is from ('comp' or 'db'), its COMP record, its Demon Database id. */
  function rbpDemonOf(src, c, demonId) {
    const d = S.ddb.data;
    if (!d) return null;
    if (src === 'comp') {
      if (!c) return null;
      const r = rbpResolve(c);
      if (!r) return { missing: c };
      const { x, fused } = r;
      const ranks = Array.isArray(c.reunion) && c.reunion.length === 12 ? c.reunion.map((v) => Math.max(0, n0(v))) : Array(12).fill(0);
      return { x, comp: c, name: c.name, level: c.level, fused: fused || n0(c.mitamaType) > 0, kind: n0(c.mitamaType), mitamaRank: n0(c.mitamaRank), ranks,
        growth: n0(c.growthType) || (fused && x.mt ? x.mt.growth : x.growth), slots: Array.isArray(c.mitamaSlots) ? c.mitamaSlots.map(n0) : Array(96).fill(0), known: c.loaded !== false };
    }
    const x = d.demons.find((q) => q.id === demonId && rbpPlannable(q));   // a pick kept from before the list was narrowed may no longer be one
    return x ? { x, name: x.name, level: x.level, fused: false, kind: 0, mitamaRank: 0, ranks: Array(12).fill(0), growth: x.growth, slots: Array(96).fill(0), known: true } : null;
  }
  // ---------- Demon Planner builds ----------
  // A demon build is a plan and the demon it is for: a Demon Database demon (`demonId`), or a COMP demon as it was
  // imported (`comp`: its rebirth ranks, Mitama slots and skills as last read, kept with the build). The saved builds
  // are in the page's storage ("rbp-builds"), as the unsaved plan is; the Build Planner's are files because they hang
  // on a character, these hang on no one. The page has two views: the gallery ('home') and the planner ('edit').
  function rbpBuilds() {
    const U = S.rbp;
    if (!U.builds) {
      let list = [];
      try { const v = JSON.parse(store.get('rbp-builds') || '[]'); if (Array.isArray(v)) list = v.filter((b) => b && typeof b === 'object' && typeof b.id === 'string' && b.plan && typeof b.plan === 'object'); } catch (e) { list = []; }
      U.builds = list.map((b) => ({ ...b, name: String(b.name || 'Demon build').slice(0, 60) }));
    }
    return U.builds;
  }
  const rbpStoreBuilds = () => store.set('rbp-builds', JSON.stringify(rbpBuilds()));
  const rbpNewId = () => 'd' + Date.now().toString(36) + Math.random().toString(36).slice(2, 6);
  const rbpClone = (v) => JSON.parse(JSON.stringify(v));
  /** A COMP demon as a build keeps it: only what the planner reads of it. */
  function rbpSnap(c) {
    if (!c || typeof c !== 'object') return null;
    const list = (v, len) => (Array.isArray(v) ? v.slice(0, len).map(n0) : null);
    const icon = typeof c.icon === 'string' && c.icon.length < 400 && !c.icon.startsWith('data:') ? c.icon : null;
    return { key: String(c.key || '').slice(0, 80), type: n0(c.type), name: String(c.name || '').slice(0, 60), level: Math.max(0, Math.min(99, n0(c.level))), race: String(c.race || '').slice(0, 40), mitama: String(c.mitama || '').slice(0, 20), icon,
      growthType: n0(c.growthType), reunion: list(c.reunion, 12), mitamaType: n0(c.mitamaType), mitamaRank: n0(c.mitamaRank), mitamaSlots: list(c.mitamaSlots, 96),
      skills: list(c.skills, 8), equipment: list(Array.isArray(c.equipment) ? c.equipment : c.equipmentIds, 4), forceStackIds: list(c.forceStackIds, 8), loaded: c.loaded !== false };
  }
  /** What the planner holds now, as a build is written (the name and the plan, and the demon it is for). */
  function rbpBuildOf() {
    const U = S.rbp, live = U.src === 'comp' ? U.compSnap || (U.comp || []).find((q) => q.key === U.compKey) : null;
    return { name: U.name, src: U.src, demonId: U.demonId === null || U.demonId === undefined ? null : U.demonId, comp: live ? rbpSnap(live) : null, plan: rbpClone(rbpPlan()) };
  }
  /**
   * A saved build as the demon it ends as, for the Character Planner to digitalize with: its form, level, growth,
   * rebirth ranks, Mitama and slots, skill bar, equipment and Demon Force, worked out as the planner's stats card does
   * (both views). A planned Mitama has only its kind (the game rolls its bonus when it goes in): `planned` marks those
   * slots, which count for the sets and add no bonus of their own. { record, D, fused } (record: what the reader's
   * `planned_demon` reads), or null when the build can't be worked out (its demon isn't in the Demon Database, or the
   * tables are still loading).
   */
  function rbpRecord(b) {
    const P = S.rbp.state === 'ready' ? S.rbp.P : null;
    if (!P || S.ddb.state !== 'ready' || !b) return null;
    const D = rbpDemonOf(b.src, b.src === 'comp' ? b.comp : null, b.demonId);
    if (!D || D.missing) return null;
    const plan = rbpPlanFrom(b.plan);
    const defaultRune = P.runeOf.has(D.x.growth) ? P.runeOf.get(D.x.growth) : -1, onRune = P.runeOf.has(D.growth) ? P.runeOf.get(D.growth) : -1;
    const xpMult = 1 + Math.max(-99, plan.xpBonus) / 100;
    let pre = null, post = null, fused = D.fused, kind = D.kind, mitamaRank = D.mitamaRank, M = null;
    if (plan.view === 'boosts') ({ pre, post, fused, kind, mitamaRank, M } = rbpDerive(P, plan, D, { defaultRune, onRune, xpMult }));
    else {
      let postCur = D.ranks, postOn = onRune, postLevel = D.level;
      if (!D.fused) {   // as pageRebirth: the rebirths before the fusion, then (fused) the ranks start again on the Mitama form's growth
        pre = rbpSteps(P, D.ranks, onRune, D.level, plan.pre, plan.last, xpMult);
        const points = rbpPoints(pre.ranks, defaultRune);
        mitamaRank = Math.max(0, Math.floor((points - 48) / 4));
        fused = plan.fuse && points >= 48 && !!D.x.mt; kind = plan.kind;
        postCur = Array(12).fill(0); postOn = D.x.mt && P.runeOf.has(D.x.mt.growth) ? P.runeOf.get(D.x.mt.growth) : -1; postLevel = 1;
      }
      if (fused) { post = rbpSteps(P, postCur, postOn, postLevel, plan.post, plan.postLast, xpMult * 0.4); M = rbpSlotsModel(P, plan, D, post.ranks, mitamaRank); }
    }
    const res = post || pre, ranks = post ? post.ranks : pre ? pre.ranks : D.ranks;
    const slots = Array(96).fill(0), planned = Array(96).fill(false);
    if (fused) {
      if (D.fused) D.slots.slice(0, 96).forEach((id, i) => { slots[i] = n0(id); });   // the ones it holds, with their bonus
      // the planned ones in the rune's empty slots: an id of their kind (kind = id / 32; 0 is an empty slot, so Ara is 1)
      if (M) M.rows.forEach((r) => r.added.forEach((n, k) => {
        for (let j = r.i * 8, left = n; left > 0 && j < r.i * 8 + 8; j++) if (!slots[j]) { slots[j] = k ? k * 32 : 1; planned[j] = true; left--; }
      }));
    }
    const type = rbpFormType(D, fused), count = P.force ? rbpForceCount(P, type) : 8;
    return { D, fused, preRanks: pre && fused && !D.fused ? pre.ranks.slice(0, 12) : null, record: {
      name: D.name, type, level: plan.level, growthType: res && res.growth ? res.growth.growth : D.growth, reunion: ranks.slice(0, 12),
      mitamaType: fused ? kind : 0, mitamaRank: fused ? mitamaRank : 0, mitamaSlots: slots, planned,
      skills: rbpSkills(plan, D).slice(0, RBP_SKILL_SLOTS), equipment: rbpEquip(plan, D, fused).slice(0, RBP_EQ_SLOTS),
      forceStack: rbpForce(plan, D).map((v, i) => (i < count ? v : 0)) } };
  }
  /** Puts a saved build in the planner (null: the unsaved plan, as it was kept). */
  function rbpLoad(b) {
    const U = S.rbp;
    if (b) {
      U.plan = rbpPlanFrom(b.plan); U.src = b.src === 'comp' ? 'comp' : 'db'; U.demonId = num(b.demonId);
      U.compSnap = b.comp && typeof b.comp === 'object' ? b.comp : null; U.compKey = U.compSnap ? U.compSnap.key : null;
      U.id = b.id; U.name = b.name; U.saveState = 'saved';
    } else {
      U.plan = null; U.src = store.get('rbp-src') === 'comp' ? 'comp' : 'db'; U.demonId = num(store.get('rbp-demon')); U.compKey = store.get('rbp-comp') || null;
      try { const v = JSON.parse(store.get('rbp-snap') || 'null'); U.compSnap = v && typeof v === 'object' ? v : null; } catch (e) { U.compSnap = null; }
      U.id = null; U.name = ''; U.saveState = '';
    }
    Object.assign(U, { naming: false, confirmDel: false });
  }
  const rbpStatus = () => { const el = S.rbp.statusEl; if (el) el.textContent = { saved: 'Saved', failed: 'Not saved' }[S.rbp.saveState] || ''; };
  const rbpShow = () => { if (S.page !== 'rebirth') go('rebirth'); else { render(); $('#main').scrollTop = 0; } };
  /** Opens a build in the planner (no id: the unsaved plan). */
  function rbpOpen(id) {
    const U = S.rbp, b = id ? rbpBuilds().find((x) => x.id === id) : null;
    if (id && !b) { toast('err', 'That build can’t be opened', 'It is not among your saved demon builds.'); return; }
    const same = U.view === 'edit' && U.id === (id || null);
    if (S.page === 'rebirth' && !same) remember();   // (from another page, go() does it)
    if (!same) rbpLoad(b);
    U.view = 'edit';
    rbpShow();
  }
  /** The page of builds. */
  function rbpGoHome() {
    const U = S.rbp;
    if (S.page === 'rebirth' && U.view === 'edit') remember();
    U.view = 'home';
    rbpShow();
  }
  /** The saved builds under "Demon Planner" in the left bar; the one in the planner is marked instead of the entry itself. */
  function drawRbpNav() {
    const U = S.rbp, list = rbpBuilds(), open = U.navOpen && list.length > 0;
    const editing = S.page === 'rebirth' && U.view === 'edit' && U.id ? U.id : null;
    rbpNavToggle.hidden = !list.length;
    rbpNavToggle.setAttribute('aria-expanded', String(open));
    rbpNavToggle.dataset.tip = open ? 'Hide your saved demon builds' : `Show your ${list.length} saved demon build${list.length === 1 ? '' : 's'}`;
    rbpNavBox.hidden = !open;
    rbpNavBox.replaceChildren(...list.map((b) => h('button', { type: 'button', class: 'nav-subitem', 'aria-current': b.id === editing ? 'page' : null, 'data-tip': b.name, onclick: () => rbpOpen(b.id) }, b.name)));
    const entry = nav.querySelector('.nav-item[data-page="rebirth"]');
    if (entry && editing && open) entry.removeAttribute('aria-current');
  }
  const rbpNewName = (base) => {
    const names = new Set(rbpBuilds().map((b) => b.name));
    if (base) {
      const fit = (ending) => String(base).slice(0, 60 - ending.length).trimEnd() + ending;
      let n = fit(' copy'), i = 2;
      while (names.has(n)) n = fit(` copy ${i++}`);
      return n;
    }
    let i = names.size + 1; while (names.has(`Demon build ${i}`)) i++;
    return `Demon build ${i}`;
  };
  /** The unsaved plan becomes a saved build with this name; "New build" then starts from nothing again. */
  function rbpSaveDraft(name) {
    const U = S.rbp;
    U.name = name;
    const id = rbpNewId();
    rbpBuilds().push({ id, ...rbpBuildOf(), saved_at: new Date().toISOString() });
    rbpStoreBuilds();
    ['rbp-plan', 'rbp-src', 'rbp-demon', 'rbp-comp', 'rbp-snap'].forEach((k) => store.set(k, ''));
    Object.assign(U, { id, saveState: 'saved', naming: false });
    drawRbpNav();
  }
  /** A copy of a build (b: a saved one, or { name, ...rbpBuildOf() }); `open`: into the planner. */
  function rbpDuplicate(b, open) {
    const id = rbpNewId();
    rbpBuilds().push({ id, name: rbpNewName(b.name), src: b.src, demonId: b.demonId, comp: b.comp ? rbpClone(b.comp) : null, plan: rbpClone(b.plan), saved_at: new Date().toISOString() });
    rbpStoreBuilds(); drawRbpNav();
    if (open) rbpOpen(id); else render();
  }
  function rbpDelete(id) {
    const U = S.rbp;
    U.builds = rbpBuilds().filter((b) => b.id !== id);
    rbpStoreBuilds();
    if (U.id === id) { U.view = 'home'; rbpLoad(null); }
    drawRbpNav(); render();
  }
  // --- sharing: a text code ("BABELD1-…") that another player pastes into "Import build". It holds the plan and the demon
  //     it is for (a Demon Database demon's id, or a COMP demon's ranks and Mitama slots); no build name. A pasted code
  //     is a stranger's input: it is only ever read as data (JSON of numbers), every field is checked and cut to size
  //     (rbpPlanFrom, rbpSnap), and a demon the Demon Database doesn't have is refused. Importing adds a new build. ---
  const RBP_CODE = 'BABELD1-';
  function rbpCode(b) {
    const bytes = new TextEncoder().encode(JSON.stringify({ s: b.src, d: b.demonId, c: b.src === 'comp' ? b.comp : null, p: b.plan }));
    let bin = ''; bytes.forEach((v) => { bin += String.fromCharCode(v); });
    return RBP_CODE + btoa(bin).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '');
  }
  /** The build a code holds ({ src, demonId, comp, plan }); throws a plain-words reason when it holds none. */
  function rbpReadCode(text) {
    const t = String(text || '').replace(/\s+/g, '');
    if (!t.startsWith(RBP_CODE)) throw new Error(t.startsWith('BABEL1-') ? 'That is a Build Planner code. Paste it in the Build Planner’s “Import build”.' : 'That is not a demon build code: it starts with “BABELD1-”.');
    if (t.length > 12000) throw new Error('That code is too long to be a demon build.');
    let o;
    try {
      const bin = atob(t.slice(RBP_CODE.length).replace(/-/g, '+').replace(/_/g, '/'));
      o = JSON.parse(new TextDecoder('utf-8', { fatal: true }).decode(Uint8Array.from(bin, (c) => c.charCodeAt(0))));
    } catch (e) { throw new Error('The code is damaged or cut short: copy it again, all of it.'); }
    if (!o || typeof o !== 'object' || (o.s !== 'db' && o.s !== 'comp')) throw new Error('The code holds no demon build.');
    const d = S.ddb.data;
    if (!d) throw new Error('The demon list is still loading: try again in a moment.');
    const b = { src: o.s, demonId: null, comp: null, plan: rbpPlanFrom(o.p) };
    if (o.s === 'comp') {
      b.comp = rbpSnap(o.c);
      if (!b.comp || !rbpResolve(b.comp)) throw new Error('The demon in the code is not in the Demon Database, so it can’t be planned here.');
    } else {
      b.demonId = n0(o.d);
      const x = d.demons.find((q) => q.id === b.demonId);
      if (!x) throw new Error('The demon in the code is not in the Demon Database, so it can’t be planned here.');
    }
    return b;
  }
  /** What a build is, in a few words: its demon and what is planned. { x, name, level, tags } (x: the database entry, or null). */
  function rbpSummary(b) {
    const d = S.ddb.data;
    if (!d) return null;
    let x = null, name = '', level = 0, fused = false;
    if (b.src === 'comp' && b.comp) { const r = rbpResolve(b.comp); if (r) x = r.x; name = b.comp.name || (x ? x.name : ''); level = n0(b.comp.level); fused = n0(b.comp.mitamaType) > 0 || !!(r && r.fused); }
    else { x = d.demons.find((q) => q.id === n0(b.demonId)) || null; if (x) { name = x.name; level = x.level; } }
    const plan = rbpPlanFrom(b.plan);
    const runes = plan.pre.reduce((n, v, i) => n + (v > 0 || plan.post[i] > 0 ? 1 : 0), 0);
    const tags = [];
    if (plan.view === 'boosts') tags.push(plan.picks.length ? `${plan.picks.length} boost${plan.picks.length === 1 ? '' : 's'} picked` : 'Boosts view');
    else if (runes) tags.push(`${runes} rune${runes === 1 ? '' : 's'} planned`);
    // the Mitama it has, or the one the plan fuses it with: { kind 1-4, fused, rank, text } (a card draws it as the planner does)
    let mt = null;
    if (b.src === 'comp' && n0(b.comp && b.comp.mitamaType) > 0) mt = { kind: n0(b.comp.mitamaType), fused: true, rank: n0(b.comp.mitamaRank), text: `${RBP_KINDS[n0(b.comp.mitamaType) - 1] || ''} Mitama`.trim() };
    else if (plan.fuse) mt = { kind: plan.kind, fused: false, rank: 0, text: `Fusing with ${RBP_KINDS[plan.kind - 1]}` };
    if (mt) tags.push(mt.text);
    if (b.src === 'comp') tags.push('From your COMP');
    return { x, name: name || 'Unknown demon', level, tags, mt };
  }
  const rbpCodeDlg = () => $('#sim-code');   // the Build Planner's share / import dialog, with this planner's words in it
  /** "Share": the code of a build ({ src, demonId, comp, plan }), to copy. */
  async function rbpShare(b, name) {
    const dlg = rbpCodeDlg(), code = rbpCode(b);
    ++simCodeSeq;
    $('#sim-code-title').textContent = 'Share demon build';
    const box = h('textarea', { class: 'input sim-code-box', readonly: true, rows: 4, spellcheck: 'false', 'aria-label': 'Build code' });
    box.value = code;
    box.addEventListener('focus', () => box.select());
    const copy = h('button', { type: 'button', class: 'btn btn-primary', onclick: async () => {
      if (await copyText(code)) flashLabel(copy, 'Copied');
      else { box.focus(); box.select(); toast('info', 'Copy it by hand', 'The code is selected: press Ctrl+C.'); }
    } }, 'Copy code');
    simFill($('#sim-code-body'),
      h('p', { class: 'field-help', text: `The code of ${name ? `“${name}”` : 'this build'}. Send it to another player: they paste it into “Import build” in their Demon Planner.` }),
      box,
      h('div', { class: 'field-row' }, copy, h('span', { class: 'field-help sim-code-count', text: `${code.length} characters` })),
      h('p', { class: 'dlg-note' }, icon('shield'), h('span', { text: 'The code holds the plan and the demon it is for (for one of your COMP demons: its rebirth ranks and Mitama slots as last read). It does not hold your character’s name or the build’s name.' })));
    if (!dlg.open) dlg.showModal();
    copy.focus();
  }
  /** "Import build": paste a code, see what it holds, and save it as a new build. */
  function rbpImportOpen() {
    const dlg = rbpCodeDlg(), mine = ++simCodeSeq;
    let build = null;
    $('#sim-code-title').textContent = 'Import demon build';
    const box = h('textarea', { class: 'input sim-code-box', rows: 4, spellcheck: 'false', autocomplete: 'off', maxlength: 12000, placeholder: 'BABELD1-…', 'aria-label': 'Build code' });
    const status = h('p', { class: 'field-help sim-code-status', role: 'status' });
    const preview = h('div', { class: 'sim-code-preview' });
    const names = new Set(rbpBuilds().map((b) => b.name));
    let fresh = 'Imported demon build', i = 2;
    while (names.has(fresh)) fresh = `Imported demon build ${i++}`;
    const nameIn = h('input', { class: 'input sim-name', type: 'text', maxlength: 60, value: fresh, 'aria-label': 'Name of the build', spellcheck: 'false' });
    const saveBtn = h('button', { type: 'button', class: 'btn btn-primary', disabled: true, onclick: () => save() }, 'Save as build');
    const foot = h('div', { class: 'sim-code-foot', hidden: true }, h('label', { class: 'field' }, h('span', { class: 'field-label', text: 'Name' }), nameIn), saveBtn);
    const say = (text, bad) => { status.textContent = text || ''; status.classList.toggle('bad', !!bad); };
    function read() {
      const text = box.value.trim();
      build = null; saveBtn.disabled = true; foot.hidden = true; simFill(preview);
      if (!text) { say(''); return; }
      try { build = rbpReadCode(text); } catch (e) { say(String((e && e.message) || e), true); return; }
      say('');
      const s = rbpSummary(build);
      simFill(preview, h('div', { class: 'sim-code-list' },
        h('div', { class: 'sim-code-row' }, s && s.x ? ddbImg(s.x.icon || (S.ddb.data && S.ddb.data.noIcon), 32, 'dm-icon rbp-dm-icon') : h('span'),
          h('span', { class: 'sim-code-piece' }, h('b', { text: s ? s.name : '' }), h('span', { class: 'dgt-line', text: s && s.x ? `Lv ${s.level} · ${s.x.race}` : '' }))),
        s && s.tags.length ? h('div', { class: 'sim-code-ranks' }, s.tags.map((t) => h('span', { class: 'tag', text: t }))) : h('p', { class: 'note', text: 'Nothing is planned in it yet.' })));
      foot.hidden = false; saveBtn.disabled = false;
    }
    function save() {
      if (!build || mine !== simCodeSeq) return;
      const name = nameIn.value.replace(/\s+/g, ' ').trim() || fresh, id = rbpNewId();
      rbpBuilds().push({ id, name, src: build.src, demonId: build.demonId, comp: build.comp, plan: build.plan, saved_at: new Date().toISOString() });
      rbpStoreBuilds(); drawRbpNav();
      dlg.close();
      toast('ok', `Imported “${name}”`, 'It is a new build of yours. Nothing else was changed.', 4000);
      rbpOpen(id);
    }
    let timer = null;
    box.addEventListener('input', () => { clearTimeout(timer); timer = setTimeout(read, 200); });
    nameIn.addEventListener('keydown', (ev) => { if (ev.key === 'Enter') save(); });
    simFill($('#sim-code-body'),
      h('p', { class: 'field-help', text: 'Paste a demon build code another player shared with you. You see what it holds before anything is saved.' }),
      box, status, preview, foot,
      h('p', { class: 'dlg-note' }, icon('shield'), h('span', { text: 'A code is only numbers: a plan and the demon it is for, each checked against the game data. Importing adds a new build and changes nothing else.' })));
    if (!dlg.open) dlg.showModal();
    box.focus();
  }
  /**
   * What a build holds, as a card shows it under the demon's line: the skill bar, the Demon Force bar, the features
   * and the rune ranks, icons and numbers only, a tip on each (the planner's bars, compact). Worked out on a copy of
   * the build's plan, so nothing of the planner or the build is touched. Left out until its data has loaded (the page
   * draws again when it has).
   */
  function rbpMiniBars(b, live) {   // live: a demon as it is now (the Overview's partner card), so the ranks are what it has, not a plan
    const P = S.rbp.state === 'ready' ? S.rbp.P : null;
    if (!P || S.ddb.state !== 'ready') return null;
    const D = rbpDemonOf(b.src, b.src === 'comp' ? b.comp : null, b.demonId);
    if (!D || D.missing) return null;
    const plan = rbpPlanFrom(b.plan), out = [];
    const bar = (title, cells, tip) => h('span', { class: 'rbp-minibar', role: 'group', 'aria-label': title }, h('span', { class: 'rbp-minibar-title', 'data-tip': tip || null, text: title }), h('span', { class: 'rbp-minislots' }, cells));
    // the game-style tooltip of the expertise icons (expSkillTip): picture and name, what it is, then the game's own text.
    // The plain data-tip stays for the keyboard (focus shows it), the rich one is what the pointer shows.
    const tt = (iconEl, title, sub, lines, ...more) => h('div', { class: 'tt' }, tipHead(iconEl, title, sub), ...lines, ...more);
    const note = (text, why) => h('div', { class: 'tt-line tt-gap' + (why ? ' tt-why' : ''), text });
    const skills = rbpSkills(plan, D);
    out.push(bar('Skills', skills.map((id, i) => {
      const s = id ? rbpSkill(id) : null;
      const el = h('span', { class: 'rbp-mini' + (s ? '' : ' rbp-mini-blank'), tabindex: 0, 'data-tip': s ? `${s.name}${s.desc ? '\n' + s.desc : ''}` : `Slot ${i + 1}: empty`, 'aria-label': s ? s.name : `Slot ${i + 1}: empty` }, s ? ddbImg(s.icon, 24, 'rbp-mini-pic') : null);
      return s ? withTip(el, () => {
        const from = rbpSkillFrom(id, D), kind = rbpSkillTypeName(s) ? `${rbpSkillTypeName(s)} skill` : 'Skill';
        return tt(ddbImg(s.icon, 24, 'skill-ico'), s.name, [kind, s.el && s.el !== 'Support' ? s.el : null, `Slot ${i + 1}`].filter(Boolean).join(' · '),
          tipLines(s.desc), note(from.how === 'own' ? `${D.name} ${from.text}` : from.text));
      }) : el;
    })));
    const endFused = rbpEndFused(P, plan, D), byId = rbpForceById(P);
    if (byId) {
      const force = rbpForce(plan, D), count = rbpForceCount(P, rbpFormType(D, endFused)), before = rbpForceCount(P, D.x.id);
      out.push(bar('Demon Force', force.map((id, i) => {
        const e = id ? rbpForceEffect(P, byId, id) : null, locked = i >= count, fz = endFused && i >= before && i < count;
        if (locked && !id) return h('span', { class: 'rbp-mini rbp-mini-locked', 'aria-label': `Slot ${i + 1}: not open`, tabindex: 0, 'data-tip': `${D.name} opens ${count} slots: slot ${i + 1} can’t be filled` });
        const tip = e ? `${e.name}${e.texts.length ? '\n' + e.texts.join('\n') : ''}${locked ? `\n\n${D.name} opens ${count} slots: this one can’t be filled` : ''}${fz ? '\n\nOpened by the Mitama fusion' : ''}` : `Slot ${i + 1}: empty${fz ? '\n\nOpened by the Mitama fusion' : ''}`;
        const el = h('span', { class: 'rbp-mini' + (e ? '' : ' rbp-mini-blank') + (fz ? ' rbp-mini-fused' : '') + (locked ? ' rbp-mini-bad' : ''), tabindex: 0, 'data-tip': tip, 'aria-label': e ? e.name : `Slot ${i + 1}: empty` },
          e ? (rbpForcePic(e.item, 24, 'rbp-mini-pic') || h('span', { class: 'rbp-mini-pic rbp-mini-q', text: e.item ? '?' : '✦' })) : null);
        return e ? withTip(el, () => tt(h('span', { class: 'skill-ico' }, rbpForcePic(e.item, 24) || h('span', { class: 'rbp-mini-q', text: e.item ? '?' : '✦' })), e.name, `Demon Force · Slot ${i + 1}`,
          e.texts.length ? e.texts.map((x) => h('div', { class: 'tt-line', text: x })) : [h('div', { class: 'tt-line', text: 'The game has no text for it' })],
          fz ? note('Opened by the Mitama fusion') : null, locked ? note(`${D.name} opens ${count} slots: this one can’t be filled`, true) : null)) : el;
      })));
    }
    // the Epitaph Parts worn (only the equipment: not the features they stand in for), one square per slot
    const eq = rbpEqById(P);
    if (eq) {
      const traits = rbpTraitSlots(P, D, endFused), equip = rbpEquip(plan, D, endFused);
      out.push(bar('Equipment', equip.slice(0, RBP_EQ_SLOTS).map((id, i) => {
        const it = id ? eq.get(id) : null, T = traits[i] ? rbpTrait(traits[i]) : null;
        // the partner card: a slot with no part shows its feature (a part always wins the slot)
        if (!it && live && T) {
          const known = !!(S.ddb.data && S.ddb.data.skills[T.id]), name = known ? T.name : 'A feature the game data does not name';
          // its own picture when the game has one, else the features' thumbnail of the digitalize demon's card (the Features boost's picture, tinted)
          const pic = (cls) => { if (T.icon) { const own = ddbImg(T.icon, 24, cls); if (own) return own; } const ic = skillIcon({ icon: 'digitalize_featureicon' }); ic.classList.add('as-feature'); if (cls !== 'skill-ico') ic.classList.add(cls); return ic; };
          const el = h('span', { class: 'rbp-mini rbp-mini-feature', tabindex: 0, 'aria-label': `${name}, feature`, 'data-tip': `${name}${T.desc ? '\n' + T.desc : ''}` }, pic('rbp-mini-pic'));
          return withTip(el, () => tt(pic('skill-ico'), name, `Feature · Slot ${i + 1}`, T.desc ? tipLines(T.desc) : [h('div', { class: 'tt-line', text: 'The game has no text for it' })], note('No demon equipment in this slot')));
        }
        if (!it) return h('span', { class: 'rbp-mini rbp-mini-blank', tabindex: 0, 'aria-label': `Slot ${i + 1}: no part`, 'data-tip': `Slot ${i + 1}: no Epitaph Part` });
        const letter = () => h('span', { class: 'rbp-mini-pic rbp-mini-q', text: String(it.name || '?').charAt(0) });
        const el = h('span', { class: 'rbp-mini', tabindex: 0, 'aria-label': it.name, 'data-tip': `${it.name}
${rbpEqText(it)}${T ? `
In place of ${T.name}` : ''}` }, rbpEqPic(it.id, 24) || letter());
        return withTip(el, () => {
          const lines = tipLines(it.desc);
          return tt(h('span', { class: 'skill-ico' }, rbpEqPic(it.id, 24) || h('span', { class: 'rbp-mini-q', text: String(it.name || '?').charAt(0) })), it.name, `Demon equipment · Slot ${i + 1}`,
            lines.length ? lines : [h('div', { class: 'tt-line', text: it.gnames.join(', ') || it.skillName })],
            T ? note(`In place of the feature ${T.name}`) : note('In a slot with no feature'));
        });
      }), 'The Epitaph Parts the demon wears, one per feature slot'));
    }
    // the rebirth ranks planned: a radar of the 12 runes, before the Mitama fusion and after it (a fused demon: only its ranks, as "after")
    const rows = rbpMiniRanks(P, plan, D), sum = (r) => (r ? r.ranks.reduce((s, n) => s + n, 0) : 0);
    const preRow = D.fused ? null : rows.find((r) => r.title === 'Ranks'), postRow = rows.find((r) => (D.fused ? r.title === 'Ranks' : r.title === 'After fusion'));
    const before = preRow ? sum(preRow) : null, after = postRow ? sum(postRow) : 0;
    const key = (cls, label, n) => h('span', { class: 'rbp-radar-key', 'data-tip': label }, h('i', { class: cls }), h('span', { text: label }), h('b', { text: String(n) }));
    // (live, the Overview's partner card: the same graph, colours and labels as a build card, the user's call; only the tip drops "as planned")
    const rankTip = (D.fused ? `${after} rebirth rank${after === 1 ? '' : 's'} on the runes since the Mitama fusion`
      : `${before} rebirth rank${before === 1 ? '' : 's'} before the Mitama fusion, then ${after} after it`) + (live ? '' : ', as planned');
    // the radar in a column of its own, to the right of the rows (a narrow card puts it under them)
    const ranksBox = h('span', { class: 'rbp-miniranks', role: 'group', 'aria-label': 'Ranks' }, h('span', { class: 'rbp-minibar-title', 'data-tip': 'The rebirth rank of each rune (the centre is 0, the rim 9): before the Mitama fusion and after it', text: 'Ranks' }),
      h('span', { class: 'rbp-radar-wrap', tabindex: 0, 'data-tip': rankTip, 'aria-label': rankTip },
        rbpRanksRadar(P, preRow ? preRow.ranks : null, postRow ? postRow.ranks : null),
        h('span', { class: 'rbp-radar-keys' }, preRow ? key('rbp-radar-pre', 'Pre-Mitama', before)
          // The partner card, a fused demon: the same two lines as a build card. The fusion cleared the 12 ranks from
          // before it and kept only their total as the Mitama rank = (total - 48) / 4 (comp_hack MitamaDemon;
          // newmoon-kb rebirth-mitama.md), so "before" is that total (a range of 4) and has no outline on the radar.
          : live && D.fused ? (() => { const lo = 48 + 4 * D.mitamaRank, hi = Math.min(96, lo + 3); const el = key('rbp-radar-pre', 'Pre-Mitama', lo === hi ? lo : `${lo}–${hi}`);
            el.dataset.tip = `Worked out from its Mitama rank ${D.mitamaRank}: the fusion clears each rune’s rank and keeps only their total, so the radar can’t draw them`; el.tabIndex = 0; return el; })() : null,
          key('rbp-radar-post', D.fused && !live ? 'On the runes now' : 'Post-Mitama', after))));
    return h('span', { class: 'rbp-minibars' }, h('span', { class: 'rbp-minirows' }, out), ranksBox);
  }
  /**
   * The 12 runes' rebirth ranks as a radar: one spoke per rune (the rim is rank 9), the ranks before the Mitama fusion
   * and after it as two outlines (pre / post: null leaves one out). Small, for a gallery card; each spoke has its numbers in its tip.
   */
  function rbpRanksRadar(P, pre, post) {
    const W = 124, cx = W / 2, cy = W / 2, R = 54, N = 12, MAX = 9;   // (the pictures sit on the rim, so they reach R + 7: the box is that and a 1 unit edge, no empty margin)
    const svg = s('svg', { class: 'rbp-radar', viewBox: `0 0 ${W} ${W}`, width: W, height: W, role: 'img', 'aria-hidden': 'true' });
    const ang = (i) => -Math.PI / 2 + (i * 2 * Math.PI) / N;
    const at = (i, v) => { const r = R * Math.max(0, Math.min(1, v / MAX)); return [cx + r * Math.cos(ang(i)), cy + r * Math.sin(ang(i))]; };
    const path = (vals) => vals.map((v, i) => { const [x, y] = at(i, v); return `${i ? 'L' : 'M'}${x.toFixed(1)},${y.toFixed(1)}`; }).join('') + 'Z';
    const idx = [...Array(N).keys()];
    idx.forEach((i) => { const [x, y] = at(i, MAX); svg.append(s('line', { class: 'rbp-radar-grid', x1: cx, y1: cy, x2: x, y2: y })); });
    [3, 6, 9].forEach((v) => svg.append(s('path', { class: 'rbp-radar-grid', d: path(idx.map(() => v)) })));
    [['rbp-radar-pre', pre], ['rbp-radar-post', post]].forEach(([cls, vals]) => { if (vals && vals.some(Boolean)) svg.append(s('g', { class: cls }, s('path', { class: 'rbp-radar-area', d: path(vals) }), s('path', { class: 'rbp-radar-line', d: path(vals) }))); });
    // the runes' pictures on the rim, and a wide invisible spoke over each for its tip
    const I = (S.data && S.data.icons) || {};
    idx.forEach((i) => {
      const Ru = P.runes[i], [x, y] = at(i, MAX + 2.6), [hx, hy] = at(i, MAX);
      const a = pre ? pre[i] : null, b = post ? post[i] : 0;
      const tip = `${Ru.name}: ${a === null ? `rank ${b}` : `rank ${a} before the fusion, ${b} after it`}`;
      svg.append(s('g', { 'data-tip': tip },
        s('line', { x1: cx, y1: cy, x2: hx, y2: hy, stroke: 'transparent', 'stroke-width': 14 }),
        I[`rune${i}`] ? s('image', { class: 'rbp-radar-ico', href: I[`rune${i}`], x: (x - 7).toFixed(1), y: (y - 7).toFixed(1), width: 14, height: 14 })
          : Object.assign(s('text', { class: 'rbp-radar-name', x: x.toFixed(1), y: (y + 3.5).toFixed(1), 'text-anchor': 'middle' }), { textContent: Ru.name.charAt(0) })));
    });
    return svg;
  }
  /** The rune ranks a build plans: [{ title, ranks[12], cur[12] (what the demon has), tip }], before the fusion and after it. */
  function rbpMiniRanks(P, plan, D) {
    const zeros = Array(12).fill(0), rows = [];
    const defaultRune = P.runeOf.has(D.x.growth) ? P.runeOf.get(D.x.growth) : -1, onRune = P.runeOf.has(D.growth) ? P.runeOf.get(D.growth) : -1;
    const mtRune = D.x.mt && P.runeOf.has(D.x.mt.growth) ? P.runeOf.get(D.x.mt.growth) : -1;
    let pre = null, post = null;
    if (plan.view === 'boosts') {
      const R = rbpDerive(P, plan, D, { defaultRune, onRune, xpMult: 1 });
      if (plan.picks.length) { pre = R.pre ? R.pre.ranks : null; post = R.post ? R.post.ranks : null; }
      else if (!D.fused) pre = D.ranks;
    } else if (!D.fused) {
      const r = rbpSteps(P, D.ranks, onRune, D.level, plan.pre, plan.last, 1);
      pre = r.ranks;
      if (plan.fuse && D.x.mt && rbpPoints(r.ranks, defaultRune) >= 48) post = rbpSteps(P, zeros, mtRune, 1, plan.post, plan.postLast, 1).ranks;
    } else post = rbpSteps(P, D.ranks, onRune, D.level, plan.post, plan.postLast, 1).ranks;
    if (D.fused) { if (post) rows.push({ title: 'Ranks', ranks: post, cur: D.ranks, tip: 'The rebirth rank of each rune, as planned' }); }
    else {
      if (pre) rows.push({ title: 'Ranks', ranks: pre, cur: D.ranks, tip: 'The rebirth rank of each rune before the Mitama fusion, as planned' });
      if (post) rows.push({ title: 'After fusion', ranks: post, cur: zeros, tip: 'The rebirth rank of each rune after the Mitama fusion (they start again from 0)' });
    }
    return rows;
  }
  /** One saved build as a card in the gallery. */
  function rbpCard(b) {
    const s = rbpSummary(b);
    const acts = h('span', { class: 'sim-build-acts' });
    // (the path, not ev.target.closest: a button of the actions is replaced by the click itself, and then has no parent to find)
    const el = h('div', { class: 'dg-tile sim-build rbp-build', onclick: (ev) => { if (!ev.composedPath().includes(acts)) rbpOpen(b.id); } });
    const drawActs = (asking) => simFill(acts, asking
      ? [h('span', { class: 'dgt-line', text: 'Delete it?' }), h('button', { type: 'button', class: 'btn btn-sm', onclick: () => rbpDelete(b.id) }, 'Delete'),
        h('button', { type: 'button', class: 'btn btn-ghost btn-sm', onclick: () => drawActs(false) }, 'Keep')]
      : [h('button', { type: 'button', class: 'btn btn-ghost btn-square btn-sm', 'aria-label': `Share ${b.name}`, 'data-tip': 'Share: a code another player can import', onclick: () => rbpShare(b, b.name) }, icon('share')),
        h('button', { type: 'button', class: 'btn btn-ghost btn-square btn-sm', 'aria-label': `Duplicate ${b.name}`, 'data-tip': 'Duplicate', onclick: () => rbpDuplicate(b, false) }, icon('copy')),
        h('button', { type: 'button', class: 'btn btn-ghost btn-square btn-sm', 'aria-label': `Delete ${b.name}`, 'data-tip': 'Delete', onclick: () => drawActs(true) }, icon('trash'))]);
    drawActs(false);
    const pic = h('span', { class: 'rbp-build-pic' + (s && s.x ? '' : ' none') }, s && s.x ? ddbImg(b.comp && b.comp.icon ? b.comp.icon : s.x.icon || (S.ddb.data && S.ddb.data.noIcon), 56, 'rbp-build-img') : null);
    if (s && s.x && s.mt && s.mt.fused) mitamaMark(pic, true);   // as on the planner's demon picture
    // the Mitama chip as the planner's demon card draws it: the kind's picture and colour; a demon already fused shows the tag, one to be fused the "Fusing with" chip
    const P = S.rbp.state === 'ready' ? S.rbp.P : null, K = s && s.mt && P ? P.mitama.kinds[s.mt.kind - 1] : null;
    const mtChip = !s || !s.mt ? null : s.mt.fused
      ? h('span', { class: 'tag rbp-kindtag', style: `--k:var(--mt-${s.mt.kind})`, 'data-tip': 'M-Type: the Mitama it was fused with, and its Mitama rank' }, rbpKindIcon(s.mt.kind - 1), `${RBP_KINDS[s.mt.kind - 1]} Mitama · rank ${s.mt.rank}`)
      : h('span', { class: 'rbp-fusewith' }, h('span', { class: 'q-key', text: 'Fusing with' }),
        h('span', { class: 'rbp-kindchip', 'aria-pressed': 'true', style: `--k:var(--mt-${s.mt.kind})`, 'data-tip': K ? `${K.demonName}: the Mitama rank bonus raises ${(K.ranks[12] || []).map(([, l]) => l).join(', ')}` : null }, rbpKindIcon(s.mt.kind - 1), RBP_KINDS[s.mt.kind - 1]));
    const tags = s ? s.tags.filter((t) => !s.mt || t !== s.mt.text) : [];
    const bars = rbpMiniBars(b);
    simFill(el,
      h('span', { class: 'rbp-build-top' }, pic, h('span', { class: 'sim-build-body' },
        h('span', { class: 'sim-build-head' },
          h('span', { class: 'dgt-id' }, h('button', { type: 'button', class: 'sim-build-name', 'data-tip': `Open ${b.name}`, text: b.name }),
            h('span', { class: 'dgt-line', text: s ? (s.x ? `${s.name} · Lv ${s.level}` : `${s.name} is not in the Demon Database`) : 'Loading the demons…' })),
          acts),
        tags.length || mtChip ? h('span', { class: 'dgt-tags' }, tags.map((t) => h('span', { class: 'tag', text: t })), mtChip) : null)),
      bars ? h('span', { class: 'rbp-build-bars' }, bars) : null,
      b.saved_at ? h('span', { class: 'dgt-line sim-build-saved', text: `Saved ${when(b.saved_at)}` }) : null);
    return el;
  }
  /** The page of builds: "New build" (the unsaved plan, or a fresh one), "Import build", and every saved build. */
  function rbpHome(page) {
    const builds = rbpBuilds();
    // the unsaved plan as it was kept: the demon it is for, if any, and whether anything is planned
    let kept = null;
    try { kept = { src: store.get('rbp-src') === 'comp' ? 'comp' : 'db', demonId: num(store.get('rbp-demon')), comp: JSON.parse(store.get('rbp-snap') || 'null'), plan: JSON.parse(store.get('rbp-plan') || 'null') }; } catch (e) { kept = null; }
    const ks = kept && S.ddb.state === 'ready' && (kept.src === 'comp' ? kept.comp : kept.demonId !== null) ? rbpSummary(kept) : null;
    page.append(pageHead('Your demon builds. Open one to keep working on it, or start a new one. Nothing here touches the game or your demons.'));
    page.append(h('div', { class: 'sim-builds rbp-builds' },
      h('div', { class: 'dg-tile sim-build sim-new', onclick: (ev) => { if (!ev.target.closest('.sim-new-import')) rbpOpen(null); } },
        h('button', { type: 'button', class: 'sim-new-main' },
          h('span', { class: 'sim-new-plus', 'aria-hidden': 'true' }, icon('plus')),
          h('span', { class: 'dgt-id' }, h('span', { class: 'dgt-name', text: 'New build' }),
            h('span', { class: 'dgt-line', text: ks && ks.x ? `Continue where you left off: ${ks.name}, not saved yet` : 'Pick any demon, or import one from your COMP' }))),
        h('button', { type: 'button', class: 'btn btn-sm btn-ghost sim-new-import', 'data-tip': 'Paste a code another player shared with you', onclick: () => rbpImportOpen() },
          h('span', { class: 'btn-icon', 'aria-hidden': 'true' }, icon('download')), 'Import build')),
      builds.map(rbpCard)));
    page.append(h('p', { class: 'note', text: builds.length
      ? 'A saved build keeps every change you make to it. Each card says which demon it is for and what is planned.'
      : 'No saved builds yet. Start a new one and press “Save build”: it then shows up here and under Demon Planner in the left bar.' }));
    return page;
  }
  /** The planner's top line: back to the builds, the build's name, and what can be done with it. */
  function rbpTitleBar() {
    const U = S.rbp, bar = h('div', { class: 'sim-title' });
    function draw() {
      const back = h('button', { type: 'button', class: 'btn btn-ghost sim-back', 'data-tip': 'All your demon builds', onclick: rbpGoHome }, h('span', { class: 'chev' }, icon('chevron')), 'Builds');
      if (!U.id && !U.naming) {
        simFill(bar, back, h('b', { class: 'sim-name-text', text: 'New build' }), h('span', { class: 'card-sub', text: 'not saved' }),
          h('button', { type: 'button', class: 'btn btn-primary', 'data-tip': 'Keep this build: it gets a name, a card and a line under Demon Planner', onclick: () => { U.naming = true; draw(); } }, 'Save build…'),
          h('button', { type: 'button', class: 'btn btn-ghost btn-square', 'aria-label': 'Share this build', 'data-tip': 'Share: a code another player can import', onclick: () => rbpShare(rbpBuildOf(), '') }, icon('share')));
        return;
      }
      if (!U.id) {
        const input = h('input', { class: 'input sim-name', type: 'text', maxlength: 60, value: rbpNewName(), 'aria-label': 'Name of the build', spellcheck: 'false' });
        const save = () => {
          const name = input.value.replace(/\s+/g, ' ').trim();
          if (!name) { input.focus(); return; }
          rbpSaveDraft(name); draw();
          toast('ok', `Saved “${U.name}”`, 'It keeps every change you make to it from now on.', 4000);
        };
        input.addEventListener('keydown', (ev) => { if (ev.key === 'Enter') save(); else if (ev.key === 'Escape') { ev.stopPropagation(); U.naming = false; draw(); } });
        simFill(bar, back, input, h('button', { type: 'button', class: 'btn btn-primary', onclick: save }, 'Save'),
          h('button', { type: 'button', class: 'btn btn-ghost', onclick: () => { U.naming = false; draw(); } }, 'Cancel'));
        input.focus(); input.select();
        return;
      }
      const name = h('input', { class: 'input sim-name', type: 'text', maxlength: 60, value: U.name, 'aria-label': 'Name of the build', 'data-tip': 'Click to rename', spellcheck: 'false' });
      name.addEventListener('input', () => {
        const v = name.value.replace(/\s+/g, ' ').trim();
        if (!v) return;
        U.name = v; rbpSave(); drawRbpNav();
      });
      name.addEventListener('change', () => { if (!name.value.trim()) name.value = U.name; });
      name.addEventListener('keydown', (ev) => { if (ev.key === 'Enter') name.blur(); });
      U.statusEl = h('span', { class: 'card-sub', role: 'status', 'data-tip': 'A saved build keeps every change as you make it' });
      simFill(bar, back, name, U.statusEl,
        U.confirmDel
          ? [h('span', { class: 'card-sub', text: 'Delete this build?' }), h('button', { type: 'button', class: 'btn btn-sm', onclick: () => rbpDelete(U.id) }, 'Delete'),
            h('button', { type: 'button', class: 'btn btn-ghost btn-sm', onclick: () => { U.confirmDel = false; draw(); } }, 'Keep')]
          : [h('button', { type: 'button', class: 'btn btn-ghost btn-square', 'aria-label': 'Share this build', 'data-tip': 'Share: a code another player can import', onclick: () => rbpShare(rbpBuildOf(), U.name) }, icon('share')),
            h('button', { type: 'button', class: 'btn btn-ghost btn-square', 'aria-label': 'Duplicate this build', 'data-tip': 'Duplicate: a copy to change freely', onclick: () => rbpDuplicate(rbpBuildOf(), true) }, icon('copy')),
            h('button', { type: 'button', class: 'btn btn-ghost btn-square', 'aria-label': 'Delete this build', 'data-tip': 'Delete this build', onclick: () => { U.confirmDel = true; draw(); } }, icon('trash'))]);
      rbpStatus();
    }
    draw();
    return bar;
  }
  /** A COMP demon's Demon Database entry, and whether its type is a Mitama-fused one; null when the database has none. */
  function rbpResolve(c) {
    const d = S.ddb.data, F = S.fus.eng && S.fus.eng.F;
    if (!d) return null;
    let x = d.demons.find((q) => q.id === c.type), fused = false;
    if (!x && F) {   // a Mitama-fused type: the Fusion Planner's data says which demon it is the form of
      const f = F.demons.find((row) => row[8] === c.type);
      if (f) { x = d.demons.find((q) => q.id === f[0]) || d.demons.find((q) => q.name === f[1] && q.race === (F.races[f[2]] || '') && q.level === f[4]); fused = !!x; }
    }
    if (!x && c.type > 10000) {   // an event demon the Fusion Planner doesn't list (Santa Black Frost...): the server's Mitama type is its id + 10000
      x = d.demons.find((q) => q.id === c.type - 10000 && q.mt);
      fused = !!x;
    }
    return x ? { x, fused } : null;
  }
  /** Whether "Any demon" offers it: it has a Mitama form (the only demons the server Mitama-fuses) and the player can get it. */
  const rbpPlannable = (x) => !!x.mt && ddbCanOwn(x);
  /** The server's level for a rank, and the XP from level 1 to a level. */
  const rbpLevelFor = (rank) => (rank >= 9 ? 99 : rank * 10 + 10);
  function rbpXpTo(P, level) { let s = 0; for (let l = 1; l < level; l++) s += P.xp[l] || 0; return s; }
  /**
   * The rebirths that take a demon from `cur` ranks to `targets`, rune by rune (the end path last), as the server allows
   * them: entering another rune is a rank-1 rebirth, then one rebirth per rank. `on` is the rune the demon's growth is
   * on now (-1: none), `level` its level, `mult` the XP each apple is worth as a share of 10,000 (bonus, Mitama cut).
   */
  function rbpSteps(P, cur, on, level, targets, last, mult) {
    const ranks = cur.slice(), steps = [];
    let lv = level, at = on;
    // the rune the demon is on goes first (no switch, and its own growth can skip rank 1), the end path last
    const order = [...Array(12).keys()].filter((i) => i !== last && targets[i] > ranks[i]).sort((a, b) => (a === on ? -1 : b === on ? 1 : a - b));
    if (targets[last] > ranks[last]) order.push(last);
    const apples = (to) => (lv >= to ? 0 : Math.ceil((rbpXpTo(P, to) - rbpXpTo(P, lv)) / (P.apple.xp * mult)));
    for (const i of order) {
      const R = P.runes[i];
      let from = ranks[i];
      if (at !== i) {
        if (at >= 0 && ranks[at] === 0) ranks[at] = 1;   // the rune left keeps at least rank 1
        steps.push({ rune: i, rank: Math.max(1, from), enter: true, level: lv, apples: 0, macca: 500 * lv, items: [], growth: R.ranks[0] });
        if (from === 0) { from = 1; ranks[i] = 1; }
        lv = 1; at = i;
      } else if (from === 0 && targets[i] >= 2) {
        from = 1;   // a demon still on its own growth of this rune goes straight to rank 2
      } else if (from === 0) {
        steps.push({ rune: i, rank: 1, enter: true, level: lv, apples: 0, macca: 500 * lv, items: [], growth: R.ranks[0] });
        from = 1; ranks[i] = 1; lv = 1;
      }
      for (let r = from + 1; r <= targets[i]; r++) {
        const need = rbpLevelFor(r), g = R.ranks[r - 1];
        const a = apples(need), atLevel = Math.max(lv, need);
        steps.push({ rune: i, rank: r, enter: false, level: atLevel, apples: a, macca: r * 500 * atLevel, items: g ? g.items : [], growth: g });
        ranks[i] = r; lv = 1;
      }
    }
    const total = { rebirths: steps.length, apples: steps.reduce((s, x) => s + x.apples, 0), macca: steps.reduce((s, x) => s + x.macca, 0) };
    return { steps, ranks, level: lv, on: at, total, growth: steps.length ? steps[steps.length - 1].growth : null };
  }
  /** Rank points toward the Mitama fusion (GetReunionRankTotal): each rune up to 8, the default rune at least 1. */
  function rbpPoints(ranks, defaultRune) {
    let t = ranks.reduce((s, r) => s + Math.min(8, r), 0);
    if (defaultRune >= 0 && ranks[defaultRune] === 0) t++;
    return t;
  }
  /** Conversion points the ranks are worth when extracted (REUNION_RANK_POINTS, summed up to each rank). */
  function rbpConversion(P, ranks) { return ranks.reduce((s, r) => { let v = 0; for (let p = 0; p <= Math.min(r, 9); p++) v += P.rankPoints[p]; return s + v; }, 0); }
  /** Stats at a level: growth type, rebirth ranks, then the Mitama parts, as the server works them out (see the top of this section). */
  /**
   * The server's stat changes from skills (ActiveEntityState::AdjustStats): per stat, the percents of layer 1 (type 1),
   * then the flat amounts (type 0), then the percents of layer 2 (type 2), each percent `x + trunc(x × sum / 100)` and
   * -100% or less making it 0. `adj`: [CorrectTbl id, type, value, per level] (rbp-effects.json); only the ids in `only`.
   */
  function rbpAdjust(s, adj, L, only) {
    const sums = new Map();
    adj.forEach(([id, type, v, lv]) => {
      const k = RBP_STAT_IDX[id];
      if (k === undefined || !only(id) || type > 2 || (type > 0 && !v)) return;
      const e = sums.get(k) || [0, 0, 0]; e[type === 0 ? 1 : type === 1 ? 0 : 2] += lv ? v * L : v; sums.set(k, e);
    });
    const pct = (x, p) => (p <= -100 ? 0 : x + Math.trunc(x * p * 0.01));
    sums.forEach(([p1, n, p2], k) => { let x = s[k]; if (p1) x = pct(x, p1); x += Math.trunc(n); if (p2) x = pct(x, p2); s[k] = x; });
  }
  function rbpStats(P, base, growthId, L, ranks, mit, adj) {
    const s = base.slice(), g = P.growth[String(growthId)];
    if (g) {
      const boost = Math.floor((L + 3) / 4), stage = Math.floor((boost - 1) / 5);
      const add = (k, n) => { if (g[k] && n > 0) for (let i = 0; i < 6; i++) s[i] += g[k][i] * n; };
      if (stage <= 1) add(0, boost); else if (stage === 2) { add(0, 6); add(1, boost - 6); } else if (stage === 3) { add(0, 6); add(1, 5); add(2, boost - 11); } else { add(0, 6); add(1, 5); add(2, 5); add(3, boost - 16); }
    }
    let mag = 0;
    ranks.forEach((r, i) => { const b = r - 1; if (b <= 0) return; P.runes[i].bonus.forEach(([id, , v]) => { s[RBP_STAT_IDX[id]] += v * b; }); mag += P.runes[i].mag * b; });
    mag = Math.min(100, mag);
    const other = [];   // parts that aren't one of the 14 stats: [label, value, isPercent]
    const addPart = (id, label, v, pct) => { const k = RBP_STAT_IDX[id]; if (k !== undefined) s[k] += v; else other.push([label, v, pct]); };
    let mode2 = [];
    if (mit) {
      const K = P.mitama.kinds[mit.kind - 1];   // none: a fused COMP demon whose Mitama the game didn't give (only listed), so no rank bonus is counted
      if (K) (K.ranks[Math.max(0, Math.min(12, mit.rank))] || []).forEach(([id, label, b]) => { const v = Math.round(L / 99 * Math.ceil(1.25 * b * b + b)); if (v && id <= 5) s[id] += v; else if (v) mode2.push([id, label, v]); });
      mit.slots.forEach((id) => { const sb = P.mitama.kinds.flatMap((k) => k.slots).find((q) => q[0] === id); if (!sb) return; const [, st, label, v] = sb; if (st <= 5) s[st] += v; else mode2.push([st, label, v]); });
    }
    // the skills' changes to STR ... LUCK come before the stats that depend on them (RecalculateDemonStats: base mode first)
    if (adj && adj.length) rbpAdjust(s, adj, L, (id) => id <= 5);
    const [str, mag_, vit, int_] = s, hp = s[6], mp = s[7];
    const out = s.slice();
    out[6] = hp + Math.ceil(hp * 0.03 * L) + Math.ceil(str * 0.3) + Math.ceil((hp * 0.01 + 0.5) * vit);
    out[7] = mp + Math.ceil(mp * 0.03 * L) + Math.ceil(mag_ * 0.3) + Math.ceil((mp * 0.01 + 0.5) * int_);
    const dep = (b, stat, k) => b + Math.round(stat * k + 0.5 + L * 0.1);
    out[8] = dep(s[8], str, 0.5); out[9] = dep(s[9], s[4], 0.5); out[10] = dep(s[10], mag_, 0.5); out[11] = dep(s[11], int_, 0.5);
    out[12] = dep(s[12], vit, 0.1); out[13] = dep(s[13], int_, 0.1);
    const s2 = out;
    const addTo = (id, label, v) => { const k = RBP_STAT_IDX[id]; if (k !== undefined) s2[k] += v; else other.push([label, v, id >= 78]); };
    mode2.forEach(([id, label, v]) => addTo(id, label, v));
    const setLines = [];
    if (mit) mit.sets.forEach((set) => {
      const parts = mit.boost ? set.bonusEx : set.bonus;
      parts.forEach(([id, label, v]) => { if (id === -2) mag = Math.min(100, mag + v); else if (id >= 0) addTo(id, label, v); });
      setLines.push({ set, text: rbpBoostText(mit.boost ? set.exText : set.text) });
    });
    void addPart;
    // then the rest (Max HP/MP, CLS-R, LNG-R, Spell, Support, P.Def, M.Def) on the worked-out stats, after the Mitama's
    if (adj && adj.length) rbpAdjust(s2, adj, L, (id) => id > 5);
    return { stats: s2, mag, other, setLines };
  }
  const RBP_KINDS = ['Ara', 'Nigi', 'Kusi', 'Saki'];
  const rbpMacca = (v) => `${fmt(v)} macca`;
  /** The kinds of item a rebirth can take, as the cost card's selector offers them (the Overview's affinity selector). */
  const RBP_ITEM_KINDS = [['rune', 'Runestones'], ['incense', 'Incense'], ['soul', 'Soul stones'], ['relic', 'Relics'], ['primordial', 'Primordial']];
  const rbpItemKind = (name) => (/^Primordial Runestone/.test(name) ? 'primordial' : /Runestone/.test(name) ? 'rune' : /^Incense/.test(name) ? 'incense' : /^Soul Stone/.test(name) ? 'soul' : 'relic');
  /** The item a rank's choices give under the chosen kind, else the first choice (runestones go to rank 9; incense from 5; soul stones and relics at 8 and 9; the Primordial Runestone always). */
  const rbpItemFor = (items, kind) => (items && items.length ? items.find((it) => rbpItemKind(it[1]) === kind) || items[0] : null);
  /** The rune's glyph (icons.json rune<i>, cut from the game's reunion_icon00.tga), or nothing. */
  const rbpRuneIcon = (i) => { const src = S.data && S.data.icons && S.data.icons[`rune${i}`]; return src ? h('img', { class: 'rbp-runeicon', src, alt: '', 'aria-hidden': 'true' }) : null; };
  /** A Mitama demon's face (icons.json face<k>, the client's 32 x 32 icon of Ara / Nigi / Kusi / Saki Mitama), else its small counter picture, or nothing. */
  const rbpKindFace = (k) => { const I = (S.data && S.data.icons) || {}; const src = I[`face${k}`] || I[`kind${k}`]; return src ? h('img', { class: 'rbp-kindface', src, alt: '', 'aria-hidden': 'true' }) : null; };
  /** A Mitama kind's picture (icons.json kind<k>, the game's Mitama Enhancement counters), or nothing. */
  const rbpKindIcon = (k) => { const src = S.data && S.data.icons && S.data.icons[`kind${k}`]; return src ? h('img', { class: 'rbp-kindicon', src, alt: '', 'aria-hidden': 'true' }) : null; };
  const rbpKindOf = (slotId) => Math.min(3, Math.floor(slotId / 32));
  /** The Mitama rank bonus's value at a level: round(L / 99 x ceil(1.25 b^2 + b)). */
  const rbpRankBonus = (b, L) => Math.round(L / 99 * Math.ceil(1.25 * b * b + b));
  /** A row's counts per kind: what the plan asks for, over what the demon's rune already holds. */
  const rbpCounts = (plan, i) => { const sl = plan.slots[i] || (plan.slots[i] = { counts: [0, 0, 0, 0] }); if (!Array.isArray(sl.counts)) sl.counts = [0, 0, 0, 0]; return sl.counts; };
  /** A set bonus as the page shows it: the game's text with its short names written out (LB -> Limit Break, Spd ->
   *  Speed, Crit Def -> Critical Defense, ...), a space before each number and a comma between bonuses (user,
   *  2026-11-27: no small versions of the names; searching by them still works, the search also reads the raw text). */
  function rbpBoostText(t, short) {
    const words = [[/\bLB Cap\b/g, 'Limit Break Cap'], [/\bLB\b/g, 'Limit Break'], [/\bSpd\b/g, 'Speed'], [/\bInt\b/g, 'Intelligence'], [/\bVit\b/g, 'Vitality'], [/\bLck\b/g, 'Luck'],
      [/\bStr\b/g, 'Strength'], [/\bMag\b/g, 'Magic'], [/\bCrit Def\b/g, 'Critical Defense'], [/\bCrit\b/g, 'Critical'], [/\bElec\b/g, 'Electric'], [/\bAilment Res\b/g, 'Ailment Resistance'],
      [/\bres(ist)?\b/g, 'Resistance'], [/\bCD\/Incant\b/g, 'Cooldown / Incantation'], [/\bHP\/MP\b/g, 'HP / MP'], [/\bSP\b/g, 'Soul Points'], [/\bXP\b/g, 'EXP'], [/\bregen\b/g, 'Regen'],
      [/\bPower\/Chance\b/g, 'Power / Chance'], [/Physical\/ ?Magical/g, 'Physical / Magical'], [/Curative\/Received Curative/g, 'Curative / Received Curative']];
    let s = String(t || '');
    if (!short) words.forEach(([re, to]) => { s = s.replace(re, to); });
    s = s.replace(/([A-Za-z)])([+\-−]\d)/g, '$1 $2').replace(/\s*,\s*/g, ', ').replace(/(\d%?) (?=[A-Z])/g, '$1, ');
    return s;
  }
  // ---------- Rebirth Planner: the demon's skills ----------
  // A demon holds 8 learned skills (comp_hack demon.xml LearnedSkills[8]). The bar starts with the demon's own skills (a
  // COMP demon: the 8 as last read from the game; a Demon Database demon: what it knows and learns up to the stats
  // level, in that order) and any slot can take any skill a demon can obtain in the game: one some demon learns (fusion
  // inheritance) or one an NPC barters to a demon (the Demonic Researcher's Soul Points, Skill Orb and Life Orb lists, Rag's gems...;
  // skill-db.json `barter` with for = demon). User, 2026-11-27. The picks are kept with the plan (`plan.skills`: the
  // demon they were made for, and the 8 ids); another demon starts from its own skills again.
  const RBP_SKILL_SLOTS = 8;
  const rbpSkillKey = (D) => (D.comp ? `comp:${D.comp.key}` : `db:${D.x.id}:${D.fused ? 'mt' : ''}`);
  /** The demon's own list: [id, level learned] (level 0 = known from the start); the Mitama form's when fused and it differs. */
  const rbpOwnSkills = (D) => (D.fused && D.x.mt && D.x.mt.skills ? D.x.mt.skills : D.x.skills) || [];
  /** The 8 slots the demon starts with: the game's reading for a COMP demon, else its own skills up to the stats level. */
  function rbpDefaultSkills(D, plan) {
    const read = D.comp && Array.isArray(D.comp.skills) ? D.comp.skills.map(n0).filter(Boolean) : [];
    const own = read.length ? read : rbpOwnSkills(D).filter(([, lv]) => lv <= plan.level).map(([id]) => id);
    const out = own.slice(0, RBP_SKILL_SLOTS);
    while (out.length < RBP_SKILL_SLOTS) out.push(0);
    return out;
  }
  /** The plan's 8 slots for this demon (its own skills until the player changes them). */
  function rbpSkills(plan, D) {
    const key = rbpSkillKey(D);
    if (!plan.skills || plan.skills.key !== key || !Array.isArray(plan.skills.slots)) plan.skills = { key, slots: rbpDefaultSkills(D, plan), touched: false };
    return plan.skills.slots;
  }
  /** Every skill a demon can obtain: one some demon learns, or one bartered to a demon. Cached per data load. */
  function rbpSkillPool() {
    const D = S.skl.data, byDemon = sklDemons();
    if (!D || !byDemon) return null;
    if (S.rbp.pool && S.rbp.poolFrom === D) return S.rbp.pool;
    S.rbp.pool = D.skills.filter((x) => (byDemon.get(x.id) || []).some((r) => r.how === 'skill') || (x.barter || []).some((b) => b.for === 'demon'));
    S.rbp.pool.forEach((x) => { x.hayRbp = (x.name + ' ' + (x.el || '') + ' ' + (x.type || '')).toLowerCase(); });
    S.rbp.poolFrom = D;
    return S.rbp.pool;
  }
  /** A skill as the planner names it: the Skill Database's entry, else the Demon Database's name and picture. */
  function rbpSkill(id) {
    const s = sklById(id);
    if (s) return s;
    const d = S.ddb.data && S.ddb.data.skills[id];
    return d ? { id, name: d.name, desc: d.desc, icon: d.icon, type: '' } : { id, name: `Skill ${id}`, desc: '', icon: null, type: '' };
  }
  /** Where a skill comes from for this demon: its own list (with the level), else the NPC barters, else fusion (the demons that learn it). */
  function rbpSkillFrom(id, D) {
    const own = rbpOwnSkills(D).find(([x]) => x === id);
    if (own) return { how: 'own', text: own[1] ? `learns it at Lv ${own[1]}` : 'knows it from the start', lv: own[1] };
    const s = sklById(id), barters = s && s.barter ? s.barter.filter((b) => b.for === 'demon') : [];
    const learners = (sklDemons() && sklDemons().get(id) || []).filter((r) => r.how === 'skill').map((r) => r.d.name);
    const who = [...new Set(learners)];
    const parts = [];
    if (barters.length) parts.push(barters.map((b) => b.cost).join(' or '));
    if (who.length) parts.push(`fusion: ${who.slice(0, 3).join(', ')}${who.length > 3 ? ` and ${who.length - 3} more` : ''} learn it`);
    return { how: barters.length ? 'barter' : who.length ? 'fusion' : 'none', text: parts.join(' · ') || 'no known way to get it', barters, who };
  }
  const rbpSkillTypeName = (s) => { const T = S.skl.data && S.skl.data.types.find((t) => t.key === s.type); return T ? T.name : s.type === 'passive' ? 'Passive' : s.type === 'switch' ? 'Switch' : ''; };
  /** The Skills step: the bar of 8 slots, what each holds and where it comes from, and the demon's other skills. */
  function rbpSkillsCard(P, plan, D, step) {
    const card = h('div', { class: 'card rbp-skills' });
    const ready = S.skl.state === 'ready' && !!S.skl.data;
    const slots = rbpSkills(plan, D), own = rbpOwnSkills(D);
    const changed = plan.skills.touched;
    card.append(h('div', { class: 'card-head' }, h('span', { class: 'rbp-title' }, h('i', { class: 'rbp-step', text: step }), h('span', { class: 'card-title', text: 'Skills' })),
      changed ? h('button', { type: 'button', class: 'btn btn-sm', 'data-tip': 'Back to the skills the demon has on its own', onclick: () => { plan.skills = null; rbpSave(); render(); } }, 'Back to its own skills') : null));
    card.append(h('p', { class: 'note rbp-how', text: `A demon holds ${RBP_SKILL_SLOTS} skills. ${D.comp && Array.isArray(D.comp.skills) && D.comp.skills.some(Boolean) ? 'The bar starts with the skills BABEL last read on it' : `The bar starts with what ${D.name} knows and learns up to level ${plan.level}`}; click a slot to put any skill a demon can get there: one another demon learns (fusion), or one bought for a demon (Soul Points, Sacrifice Orbs, gems...).` }));
    if (!ready) { card.append(h('p', { class: 'note', text: S.skl.state === 'failed' ? 'The skill list could not be loaded (skill-db.json is missing beside the page).' : 'Loading the skills…' })); return card; }
    // the bar, as the game's: eight squares
    const bar = h('div', { class: 'rbp-skillbar', role: 'group', 'aria-label': 'Skill slots' }, slots.map((id, i) => {
      const s = id ? rbpSkill(id) : null, from = id ? rbpSkillFrom(id, D) : null;
      return h('button', { type: 'button', class: 'rbp-skillslot' + (s ? '' : ' empty') + (from && from.how !== 'own' ? ' swapped' : ''), popovertarget: 'rbp-skills', 'aria-haspopup': 'menu',
        'data-tip': s ? `${s.name}${s.desc ? '\n' + s.desc : ''}` : `Slot ${i + 1}: empty. Click to pick a skill`, 'aria-label': s ? `Slot ${i + 1}: ${s.name}` : `Slot ${i + 1}: empty`,
        onclick: (ev) => { rbpSkillsFor = ev.currentTarget; rbpSkillsSlot = i; } },
        s ? ddbImg(s.icon, 40, 'rbp-skillpic') : h('span', { class: 'rbp-skillpic rbp-skillnone rbp-plus', 'aria-hidden': 'true' }));   // no slot number on the square, nor in the list (user, 2026-11-27); the plus is drawn, so it sits in the middle
    }));
    card.append(bar);
    // what each slot holds and where it comes from
    const rows = slots.map((id, i) => {
      if (!id) return h('li', { class: 'rbp-skillrow empty' }, h('span', { class: 'rbp-rowblank' }), h('span', { class: 'src-count', text: 'empty' }));
      const s = rbpSkill(id), from = rbpSkillFrom(id, D);
      const bits = [rbpSkillTypeName(s), s.el && s.el !== 'Support' ? s.el : null, s.cost || null, s.power ? `power ${s.power}` : null].filter(Boolean).join(' · ');
      return h('li', { class: 'rbp-skillrow' + (from.how !== 'own' ? ' swapped' : '') }, ddbImg(s.icon, 24, 'skill-ico'),
        h('span', { class: 'rbp-skilltext' }, sklName(s, 'b'), bits ? h('span', { class: 'src-count', text: bits }) : null, h('small', { class: 'rbp-skillfrom' + (from.how === 'own' ? '' : ' got'), text: from.text })));
    });
    card.append(h('ul', { class: 'gdb-list rbp-skilllist' }, rows));
    // the demon's own skills that are not on the bar: a click puts one in the first empty slot (or the last one)
    const rest = own.filter(([id]) => !slots.includes(id));
    if (rest.length) card.append(h('div', { class: 'rbp-skillrest' }, h('span', { class: 'q-key', text: `${D.name}’s other skills` }),
      h('div', { class: 'chips' }, rest.map(([id, lv]) => { const s = rbpSkill(id); return h('button', { type: 'button', class: 'chip-btn rbp-skillchip', 'data-tip': `${s.desc || s.name}\n\nClick to put it on the bar`,
        onclick: () => { const at = slots.indexOf(0); slots[at < 0 ? RBP_SKILL_SLOTS - 1 : at] = id; plan.skills.touched = true; rbpSave(); render(); } }, ddbImg(s.icon, 18, 'rbp-chip-ico'), s.name, h('span', { class: 'count', text: lv ? `Lv ${lv}` : 'start' })); }))));
    return card;
  }
  // the skill picker: a popover under the slot clicked, with a search over every skill a demon can get
  const rbpSkillsMenu = $('#rbp-skills');
  let rbpSkillsFor = null, rbpSkillsSlot = 0;
  function drawRbpSkills() {
    const list = $('#rbp-skills-list'), q = ($('#rbp-skills-search input').value || '').trim().toLowerCase(), words = q.split(/\s+/).filter(Boolean);
    const plan = rbpPlan(), D = rbpDemon();
    if (!D || D.missing) { list.replaceChildren(); return; }
    const slots = rbpSkills(plan, D), pool = rbpSkillPool() || [];
    const pick = (id) => { slots[rbpSkillsSlot] = id; plan.skills.touched = true; rbpSave(); rbpSkillsMenu.hidePopover(); render(); };
    const row = (s, line, tag) => h('button', { type: 'button', class: 'char', 'aria-current': slots[rbpSkillsSlot] === s.id ? 'true' : null, 'data-tip': s.desc || null, onclick: () => pick(s.id) },
      ddbImg(s.icon, 32, 'dm-icon rbp-dm-icon'), h('span', { class: 'char-text' }, h('span', { class: 'char-name' + (s.ja ? ' tl-name' : ''), 'data-tip': s.ja ? SKL_TL_TIP(s) : null, text: s.name }), h('span', { class: 'char-line', text: line })), tag ? h('span', { class: 'tag', text: tag }) : null);
    const matches = (s) => words.every((w) => (s.hayRbp || s.name.toLowerCase()).includes(w));
    const own = rbpOwnSkills(D).map(([id, lv]) => ({ s: rbpSkill(id), lv })).filter(({ s }) => matches(s));
    const ownIds = new Set(rbpOwnSkills(D).map(([id]) => id));
    const others = pool.filter((s) => !ownIds.has(s.id) && matches(s));
    others.sort((a, b) => (q && a.name.toLowerCase().startsWith(q) ? 0 : 1) - (q && b.name.toLowerCase().startsWith(q) ? 0 : 1) || a.name.localeCompare(b.name));
    const shown = others.slice(0, 60);
    const items = [];
    if (slots[rbpSkillsSlot]) items.push(h('button', { type: 'button', class: 'char', onclick: () => pick(0) }, h('span', { class: 'dm-icon dm-none dm-off', text: '–' }), h('span', { class: 'char-text' }, h('span', { class: 'char-name', text: 'Empty the slot' }))));
    if (own.length) items.push(h('div', { class: 'list-head', text: `${D.name}’s own` }), ...own.map(({ s, lv }) => row(s, `${rbpSkillTypeName(s)}${lv ? ` · learns it at Lv ${lv}` : ' · knows it from the start'}`, slots.includes(s.id) ? 'on the bar' : null)));
    if (shown.length) items.push(h('div', { class: 'list-head', text: 'Any demon skill' }), ...shown.map((s) => { const f = rbpSkillFrom(s.id, D); return row(s, `${rbpSkillTypeName(s)} · ${f.text}`, slots.includes(s.id) ? 'on the bar' : null); }));
    list.replaceChildren(...items);
    $('#rbp-skills-note').textContent = !pool.length ? 'Loading the skills…' : !own.length && !others.length ? 'No skill matches.' : others.length > shown.length ? `${shown.length} of ${others.length} shown: type more to narrow it.` : '';
    simPlace(rbpSkillsMenu, rbpSkillsFor);
  }
  rbpSkillsMenu.addEventListener('toggle', (ev) => {
    if (ev.newState !== 'open') return;
    const inp = $('#rbp-skills-search input'); inp.value = ''; drawRbpSkills(); inp.focus();
  });
  $('#rbp-skills-search input').addEventListener('input', () => drawRbpSkills());
  $('#rbp-skills-search input').addEventListener('keydown', (ev) => { if (ev.key === 'Enter') { const b = $('#rbp-skills-list .char'); if (b) b.click(); ev.preventDefault(); } else if (ev.key === 'ArrowDown') { const b = $('#rbp-skills-list .char'); if (b) { b.focus(); ev.preventDefault(); } } });
  rbpSkillsMenu.addEventListener('keydown', (ev) => {
    if (ev.key !== 'ArrowDown' && ev.key !== 'ArrowUp') return;
    const rows = [...rbpSkillsMenu.querySelectorAll('.char')], i = rows.indexOf(document.activeElement);
    const next = rows[ev.key === 'ArrowDown' ? Math.min(rows.length - 1, i + 1) : Math.max(0, i - 1)];
    if (next) { next.focus(); ev.preventDefault(); }
  });
  $('#main').addEventListener('scroll', () => { if (rbpSkillsMenu.matches(':popover-open')) rbpSkillsMenu.hidePopover(); });

  // ---------- Rebirth Planner: Demon Force (user, 2026-11-27: right after the Mitama fusion) ----------
  // A demon carries up to 8 Demon Force effects, its force stack (comp_hack demon.xml ForceStack[8]). Using a Demon Force
  // item on the summoned demon can put the item's effect in a slot the player picks, over what is there
  // (packets\game\DemonForce.cpp: the item's DevilBoostData extraID). An empty slot past the demon type's count
  // (DevilData growth.forceStack) can't be filled; every Mitama form opens all 8 while many base forms open fewer, so the
  // step plans the form the demon ends as, and the stack is kept through the fusion (MitamaDemon leaves it). No such item
  // has a level, rune or alignment rule, and the server refuses no repeat (whether the client does is not known). The
  // Demon Database has no own name for a Mitama form, so the step names the demon ("Once Mitama-fused, Seraph opens 8").
  // benefit gauge's random effects (DevilBoostLotData) can't be planned: a COMP demon's are shown as BABEL last read them.
  // rebirth-planner.json `force`. The picks are kept with the plan (`plan.force`: the demon they were made for and the 8
  // stack ids); a COMP demon starts from its stack as last read.
  const RBP_FORCE_SLOTS = 8;
  let rbpForceItems = null, rbpForceFrom = null, rbpForceEnd = false;
  /** The item effects by stack id: { stack, item, name, texts, hay }. */
  function rbpForceById(P) {
    const F = P && P.force;
    if (!F) return null;
    if (rbpForceFrom !== F) {
      rbpForceItems = new Map(F.items.map(([item, stack, name, texts]) => [stack, { stack, item, name, texts, hay: [name, ...texts].join(' ').toLowerCase() }]));
      rbpForceFrom = F;
    }
    return rbpForceItems;
  }
  /** A slot's effect: the item's, else a random one (no item) with the client's text. */
  const rbpForceEffect = (P, byId, stack) => byId.get(stack) || { stack, item: 0, name: 'Random effect', texts: (P.force.random && P.force.random[String(stack)]) || [], hay: '' };
  /** The demon type it ends as: the Mitama form's (the Fusion Planner's row, column 8) when it will be fused. */
  function rbpFormType(D, endFused) {
    if (D.comp && (D.fused || !endFused)) return D.comp.type;
    if (!endFused || D.fused) return D.x.id;
    return rbpMtType(D);   // the equipment step's: the Fusion Planner's row, else the usual + 10000
  }
  /** How many slots a demon type opens (force.slots lists the types under 8; an unknown type: 8). */
  const rbpForceCount = (P, type) => { const n = type ? P.force.slots[String(type)] : undefined; return typeof n === 'number' ? n : P.force.max; };
  /** The stack as BABEL last read it on a COMP demon (8 ids, 0 = empty), else 8 empty slots. */
  function rbpForceRead(D) {
    const read = D.comp && Array.isArray(D.comp.forceStackIds) ? D.comp.forceStackIds.slice(0, RBP_FORCE_SLOTS).map((v) => Math.max(0, n0(v))) : [];
    while (read.length < RBP_FORCE_SLOTS) read.push(0);
    return read;
  }
  /** The plan's 8 slots for this demon (what it has until the player changes them). */
  function rbpForce(plan, D) {
    const key = rbpSkillKey(D);
    if (!plan.force || plan.force.key !== key || !Array.isArray(plan.force.slots)) plan.force = { key, slots: rbpForceRead(D), touched: false };
    return plan.force.slots;
  }
  /** An item's picture (icons.json item<id>, beside the skill icons), or nothing. */
  const rbpForcePic = (id, size, cls) => { const name = id && rbpIcons()[`item${id}`]; return name && S.meta.skillIcons ? h('img', { class: cls || 'rbp-forceimg', src: S.meta.skillIcons + name + '.png', alt: '', width: size, height: size, loading: 'lazy', onerror: (ev) => ev.target.remove() }) : null; };
  /** Where an item comes from in the game: the Item Database's first lines (it loads with the first item planned). */
  function rbpForceWhere(id) {
    if (S.gdb.state !== 'ready') return S.gdb.state === 'failed' ? '' : 'Looking up where it comes from…';
    const it = S.gdb.data.items.find((x) => x.id === id && !x.keyItem);
    const ways = it ? [...new Set([...(it.get || []).filter((g) => !g.d).map((g) => `${g.k ? `${g.k}: ` : ''}${g.t}${g.v ? ` (${g.v})` : ''}${g.s ? ` · ${g.s}` : ''}`), ...(it.quests || []).map((q) => `Quest: ${q}`)])] : [];
    if (!ways.length) return 'The game data does not say where it comes from';
    return ways.length > 2 ? `${ways.slice(0, 2).join('; ')}; or ${ways.length - 2} more ways (Item Database)` : ways.join('; or ');
  }
  /** The Demon Force step: the bar of 8 slots (the ones the form doesn't open greyed out), what each holds, and where each item comes from. */
  function rbpForceCard(P, plan, D, endFused, step) {
    rbpForceEnd = endFused;
    const card = h('div', { class: 'card rbp-force' });
    const byId = rbpForceById(P);
    const head = (btn) => h('div', { class: 'card-head' }, h('span', { class: 'rbp-title' }, h('i', { class: 'rbp-step', text: step }), h('span', { class: 'card-title', text: 'Demon Force' })), btn);
    if (!byId) { card.append(head(null), h('p', { class: 'note', text: 'The planner data has no Demon Force tables: rebuild rebirth-planner.json.' })); return card; }
    const slots = rbpForce(plan, D), read = rbpForceRead(D);
    // the slots the Mitama fusion opens: past what the demon's own (not fused) type opens, up to the form's count; the
    // server opens slots from the first, so they are the rightmost (user, 2026-11-27: highlight them, right to left)
    const count = rbpForceCount(P, rbpFormType(D, endFused)), before = rbpForceCount(P, D.x.id);
    const formName = D.name, fusing = endFused && !D.fused;
    const byFusion = (i) => endFused && i >= before && i < count;
    const opened = count - before, fusedTip = 'Opened by the Mitama fusion';
    card.append(head(plan.force.touched ? h('button', { type: 'button', class: 'btn btn-sm', 'data-tip': D.comp ? 'Back to the effects BABEL last read on the demon' : 'Back to no effects', onclick: () => { plan.force = null; rbpSave(); render(); } }, D.comp ? 'Back to what it has' : 'Clear') : null));
    if (slots.some((s, i) => s && s !== read[i]) && S.gdb.state === 'idle') loadGearDb();
    card.append(h('p', { class: 'note rbp-how', text: `A demon carries up to ${RBP_FORCE_SLOTS} Demon Force effects. Using a Demon Force item on the summoned demon puts its effect in the slot you pick, over what is there. ${fusing ? 'Once Mitama-fused, ' : ''}${formName} opens ${count} of the ${RBP_FORCE_SLOTS} slots${endFused && opened > 0 ? `: ${before} of its own and the ${opened} highlighted ones the Mitama fusion ${fusing ? 'opens' : 'opened'}${fusing ? ' (the effects are kept through the fusion)' : ''}` : ''}. Using the items also fills a gauge that now and then offers a random effect; those can’t be planned.` }));
    const bar = h('div', { class: 'rbp-skillbar', role: 'group', 'aria-label': 'Demon Force slots' }, slots.map((id, i) => {
      const e = id ? rbpForceEffect(P, byId, id) : null, locked = i >= count, isNew = !!id && id !== read[i];
      const label = e ? `${e.name}${e.texts.length ? '\n' + e.texts.join('\n') : ''}` : '';
      if (locked && !id) return h('span', { class: 'rbp-skillslot rbp-forceslot locked', role: 'img', 'aria-label': `Slot ${i + 1}: not open`, 'data-tip': `${formName} opens ${count} slots: slot ${i + 1} can’t be filled`, tabindex: 0 });
      const fz = byFusion(i);
      return h('button', { type: 'button', class: 'rbp-skillslot rbp-forceslot' + (e ? '' : ' empty') + (isNew ? ' swapped' : '') + (locked && isNew ? ' bad' : '') + (fz ? ' fused' : ''), popovertarget: 'rbp-force-menu', 'aria-haspopup': 'menu',
        'aria-label': `Slot ${i + 1}: ${e ? e.name : 'empty'}${fz ? ` (${fusedTip.toLowerCase()})` : ''}`,
        'data-tip': (e ? label + (locked && isNew ? `\n\n${formName} opens ${count} slots: this one can’t be filled` : '') : `Slot ${i + 1}: empty. Click to pick a Demon Force item`) + (fz ? `\n\n${fusedTip}` : ''),
        onclick: (ev) => { rbpForceFor = ev.currentTarget; rbpForceSlot = i; } },
        e ? (rbpForcePic(e.item, 40, 'rbp-skillpic') || h('span', { class: 'rbp-skillpic rbp-skillnone', text: e.item ? '?' : '✦' })) : h('span', { class: 'rbp-skillpic rbp-skillnone rbp-plus', 'aria-hidden': 'true' }));
    }));
    card.append(bar);
    const rows = slots.map((id, i) => {
      // no slot number (user, 2026-11-27: the numbers add little); the rows keep the bar's order, an empty one a dashed square
      const fz = byFusion(i), fzTag = fz ? h('small', { class: 'rbp-fusedtag', text: fusedTip }) : null;
      if (!id) return h('li', { class: 'rbp-skillrow empty' }, h('span', { class: 'rbp-rowblank' + (i >= count ? ' locked' : '') + (fz ? ' fused' : '') }), h('span', { class: 'src-count', text: i >= count ? `not open: ${formName} opens ${count} slots` : 'empty' }), fzTag);
      const e = rbpForceEffect(P, byId, id), isNew = id !== read[i];
      const from = !e.item ? 'a random effect from the gauge, as BABEL last read it' : isNew ? rbpForceWhere(e.item) : 'on the demon now, as BABEL last read it';
      return h('li', { class: 'rbp-skillrow' + (isNew ? ' swapped' : '') }, h('span', { class: 'rbp-forceico' + (fz ? ' fused' : '') }, rbpForcePic(e.item, 24)),
        h('span', { class: 'rbp-skilltext' }, h('b', { text: e.name }), fzTag, h('span', { class: 'src-count', text: e.texts.join(' · ') || 'the game has no text for it' }),
          i >= count && isNew ? h('small', { class: 'rbp-forcebad', text: `${formName} opens ${count} slots: this one can’t be filled` }) : null,
          h('small', { class: 'rbp-skillfrom' + (isNew ? ' got' : ''), text: from })));
    });
    card.append(h('ul', { class: 'gdb-list rbp-skilllist' }, rows));
    return card;
  }
  // the item picker: a popover beside the slot clicked, with a search over every item that puts an effect in a slot
  const rbpForceMenu = $('#rbp-force-menu');
  let rbpForceFor = null, rbpForceSlot = 0;
  function drawRbpForce() {
    const list = $('#rbp-force-list'), q = ($('#rbp-force-search input').value || '').trim().toLowerCase(), words = q.split(/\s+/).filter(Boolean);
    const P = S.rbp.P, plan = rbpPlan(), D = rbpDemon(), byId = rbpForceById(P);
    if (!D || D.missing || !byId) { list.replaceChildren(); return; }
    const slots = rbpForce(plan, D);
    const pick = (id) => { slots[rbpForceSlot] = id; plan.force.touched = true; rbpSave(); rbpForceMenu.hidePopover(); render(); };
    const all = [...byId.values()].filter((e) => words.every((w) => e.hay.includes(w)));
    const rank = (e) => (q && e.name.toLowerCase().startsWith(q) ? 0 : 1);
    all.sort((a, b) => rank(a) - rank(b) || a.name.localeCompare(b.name));
    const shown = all.slice(0, 80);
    const items = [];
    if (slots[rbpForceSlot]) items.push(h('button', { type: 'button', class: 'char', onclick: () => pick(0) }, h('span', { class: 'dm-icon dm-none dm-off', text: '–' }), h('span', { class: 'char-text' }, h('span', { class: 'char-name', text: 'Empty the slot' }))));
    items.push(h('div', { class: 'list-head', text: `Slot ${rbpForceSlot + 1}` }));
    shown.forEach((e) => { const at = slots.indexOf(e.stack); items.push(h('button', { type: 'button', class: 'char', 'aria-current': slots[rbpForceSlot] === e.stack ? 'true' : null, 'data-tip': e.texts.join('\n') || null, onclick: () => pick(e.stack) },
      h('span', { class: 'dm-icon rbp-dm-icon' }, rbpForcePic(e.item, 32)), h('span', { class: 'char-text' }, h('span', { class: 'char-name', text: e.name }), h('span', { class: 'char-line', text: e.texts.join(' · ') || 'the game has no text for it' })),
      at >= 0 && at !== rbpForceSlot ? h('span', { class: 'tag', text: `slot ${at + 1}` }) : null)); });
    list.replaceChildren(...items);
    $('#rbp-force-note').textContent = !all.length ? 'No item matches.' : all.length > shown.length ? `${shown.length} of ${all.length} shown: type more to narrow it.` : `${all.length} ${all.length === 1 ? 'item' : 'items'}.`;
    simPlace(rbpForceMenu, rbpForceFor);
  }
  rbpForceMenu.addEventListener('toggle', (ev) => {
    if (ev.newState !== 'open') return;
    const inp = $('#rbp-force-search input'); inp.value = ''; drawRbpForce(); inp.focus();
  });
  $('#rbp-force-search input').addEventListener('input', () => drawRbpForce());
  $('#rbp-force-search input').addEventListener('keydown', (ev) => { if (ev.key === 'Enter') { const b = $('#rbp-force-list .char'); if (b) b.click(); ev.preventDefault(); } else if (ev.key === 'ArrowDown') { const b = $('#rbp-force-list .char'); if (b) { b.focus(); ev.preventDefault(); } } });
  rbpForceMenu.addEventListener('keydown', (ev) => {
    if (ev.key !== 'ArrowDown' && ev.key !== 'ArrowUp') return;
    const rows = [...rbpForceMenu.querySelectorAll('.char')], i = rows.indexOf(document.activeElement);
    const next = rows[ev.key === 'ArrowDown' ? Math.min(rows.length - 1, i + 1) : Math.max(0, i - 1)];
    if (next) { next.focus(); ev.preventDefault(); }
  });
  $('#main').addEventListener('scroll', () => { if (rbpForceMenu.matches(':popover-open')) rbpForceMenu.hidePopover(); });

  // ---- the Demon equipment step (user, 2026-11-27: under the skills) ----
  // A demon wears up to 4 Epitaph Parts, one per feature (trait) slot: a part worn in a slot replaces that slot's
  // feature with the part's own skill (comp_hack CharacterManager::GetTraitSkills). The server refuses a part on a
  // feature marked fixed, and a part that shares an exclusion group with what the other three slots hold (their
  // features, or the parts worn there): packets\game\DemonEquip.cpp. rebirth-planner.json `equipment` carries the
  // tables. The step plans the demon it ends as: the Mitama form's features when the fusion is planned. The picks are
  // kept with the plan (`plan.equip`: the demon they were made for and the 4 item ids); a COMP demon starts from the
  // parts BABEL last read on it.
  const RBP_EQ_SLOTS = 4;
  let rbpEqItems = null, rbpEqFrom = null;
  /** The equipment items by id: { id, skill, name, skillName, desc, descJa (the game's Japanese when desc is BABEL's translation), groups, hay }. */
  function rbpEqById(P) {
    const E = P && P.equipment;
    if (!E) return null;
    if (rbpEqFrom !== E) {
      rbpEqItems = new Map(E.items.map(([id, skill, name, skillName, desc, descJa]) => {
        const rule = E.rules[String(skill)], groups = rule ? rule[1] : [];
        const gnames = groups.map((g) => E.groups[String(g)]).filter(Boolean);
        return [id, { id, skill, name, skillName, desc: desc || '', descJa, groups, gnames, hay: [name, skillName, ...gnames, desc || ''].join(' ').toLowerCase() }];
      }));
      rbpEqFrom = E;
    }
    return rbpEqItems;
  }
  /** The Mitama form's demon type, from the Fusion Planner's data (its row's column 8). */
  function rbpMtType(D) { const F = S.fus.eng && S.fus.eng.F, f = F && F.demons.find((r) => r[0] === D.x.id); return f ? f[8] : D.x.mt ? D.x.id + 10000 : 0; }   // demons the Fusion Planner doesn't list (events) follow id + 10000
  /** The 4 feature slots (skill ids, 0 = none) of the demon as it ends: the Mitama form's when it is or will be fused. */
  function rbpTraitSlots(P, D, endFused) {
    const type = endFused ? (D.fused && D.comp ? D.comp.type : rbpMtType(D)) : D.x.id;
    const gap = P.equipment && type ? P.equipment.traitSlots[String(type)] : null;
    if (gap) return gap.slice(0, RBP_EQ_SLOTS);
    const list = ((endFused && D.x.mt ? D.x.mt.traits : D.x.traits) || []).slice(0, RBP_EQ_SLOTS);
    while (list.length < RBP_EQ_SLOTS) list.push(0);
    return list;
  }
  const rbpEqKey = (D, endFused) => `${rbpSkillKey(D)}${endFused ? ':end-mt' : ''}`;
  /** The plan's 4 parts for this demon (what BABEL last read on a COMP demon until the player changes them). */
  function rbpEquip(plan, D, endFused) {
    const key = rbpEqKey(D, endFused);
    if (!plan.equip || plan.equip.key !== key || !Array.isArray(plan.equip.slots)) {
      const read = D.comp && Array.isArray(D.comp.equipment) ? D.comp.equipment.slice(0, RBP_EQ_SLOTS).map((v) => Math.max(0, n0(v))) : [];
      while (read.length < RBP_EQ_SLOTS) read.push(0);
      plan.equip = { key, slots: read, touched: false };
    }
    return plan.equip.slots;
  }
  /** A feature's name, text and picture (the Demon Database's; the game has no picture for most features). */
  function rbpTrait(id) {
    const d = S.ddb.data && S.ddb.data.skills[id];
    return d ? { id, name: d.name, desc: d.desc || '', icon: d.icon } : { id, name: `Feature ${id}`, desc: '', icon: null };
  }
  /** What slot `i` would hold with part `item` there: the part's skill, else the slot's feature. */
  const rbpSlotSkill = (byId, traits, equip, i) => (equip[i] && byId.get(equip[i]) ? byId.get(equip[i]).skill : traits[i]);
  /**
   * Whether part `item` can go in slot `at`, as the server checks it: the slot's feature is not fixed, and no other slot
   * holds a skill sharing an exclusion group with the part. Returns null when it can, else why not ({ fixed } or
   * { clash: the other slot's name, group }).
   */
  function rbpEqBlock(P, byId, traits, equip, at, item) {
    const E = P.equipment, it = byId.get(item);
    if (!it) return null;
    const own = E.rules[String(traits[at])];
    if (traits[at] && own && own[0]) return { fixed: true };
    const mine = new Set(it.groups);
    for (let i = 0; i < RBP_EQ_SLOTS; i++) {
      if (i === at) continue;
      const sk = rbpSlotSkill(byId, traits, equip, i), r = sk ? E.rules[String(sk)] : null;
      const g = r ? r[1].find((x) => mine.has(x)) : 0;
      if (g) return { clash: equip[i] && byId.get(equip[i]) ? byId.get(equip[i]).name : rbpTrait(traits[i]).name, group: E.groups[String(g)] || `group ${g}`, slot: i };
    }
    return null;
  }
  const rbpEqPic = (id, size) => { const name = rbpIcons()[`item${id}`]; return name && S.meta.skillIcons ? h('img', { class: 'rbp-eqimg', src: S.meta.skillIcons + name + '.png', alt: '', width: size, height: size, loading: 'lazy', onerror: (ev) => ev.target.remove() }) : null; };
  /** The game's text when it is in English (195 of the parts still have the client's Japanese one), else the group names. */
  const rbpEqText = (it) => (/[぀-ヿ一-鿿]/.test(it.desc) ? it.gnames.join(', ') : it.desc.replace(/\s*\n\s*/g, ' ').trim());
  /** Where a part comes from in the game: the Item Database's first lines (it loads with the first part planned). */
  function rbpEqWhere(id) {
    if (S.gdb.state !== 'ready') return S.gdb.state === 'failed' ? '' : 'Looking up where it comes from…';
    const it = S.gdb.data.items.find((x) => x.id === id && !x.keyItem);
    // the ways as the Item Database words them ("Chest: Suginami Tunnels (5%)"), the same place once, then its quests
    const ways = it ? [...new Set([...it.get.filter((g) => !g.d).map((g) => `${g.k ? `${g.k}: ` : ''}${g.t}${g.s ? ` · ${g.s}` : ''}`), ...it.quests.map((q) => `Quest: ${q}`)])] : [];
    if (!ways.length) return 'The game data does not say where it comes from';
    return ways.length > 2 ? `${ways.slice(0, 2).join('; ')}; or ${ways.length - 2} more ways (Item Database)` : ways.join('; or ');
  }
  /** The Demon equipment step: the 4 feature slots, what each holds now and the part planned there. */
  function rbpEquipCard(P, plan, D, endFused, step) {
    const card = h('div', { class: 'card rbp-equip' });
    const byId = rbpEqById(P);
    const formName = endFused && D.x.mt && D.x.mt.name ? D.x.mt.name : D.name;
    card.append(h('div', { class: 'card-head' }, h('span', { class: 'rbp-title' }, h('i', { class: 'rbp-step', text: step }), h('span', { class: 'card-title', text: 'Demon equipment' })),
      plan.equip && plan.equip.touched ? h('button', { type: 'button', class: 'btn btn-sm', 'data-tip': D.comp ? 'Back to the parts BABEL last read on the demon' : 'Back to no parts', onclick: () => { plan.equip = null; rbpSave(); render(); } }, D.comp ? 'Back to what it wears' : 'Clear') : null));
    if (!byId) { card.append(h('p', { class: 'note', text: 'The planner data has no demon equipment tables: rebuild rebirth-planner.json.' })); return card; }
    const traits = rbpTraitSlots(P, D, endFused), equip = rbpEquip(plan, D, endFused);
    if (equip.some(Boolean) && S.gdb.state === 'idle') loadGearDb();
    card.append(h('p', { class: 'note rbp-how', text: `A demon wears up to ${RBP_EQ_SLOTS} Epitaph Parts, one per feature slot: a part replaces that slot’s feature with its own. Parts with a similar effect can’t be worn together (the game’s rule), so a part that clashes with another slot is greyed out in the list.${endFused && !D.fused ? ` These are the features of ${formName}, the form it ends as after the Mitama fusion.` : ''}` }));
    const rows = traits.map((tr, i) => {
      const it = equip[i] ? byId.get(equip[i]) : null, T = tr ? rbpTrait(tr) : null;
      const block = it ? rbpEqBlock(P, byId, traits, equip, i, it.id) : null;
      const slot = h('button', { type: 'button', class: 'rbp-skillslot rbp-eqslot' + (it ? ' swapped' : ' empty') + (block ? ' bad' : ''), popovertarget: 'rbp-equip-menu', 'aria-haspopup': 'menu',
        'aria-label': it ? `Slot ${i + 1}: ${it.name}` : `Slot ${i + 1}: ${T ? T.name : 'no feature'}`, 'data-tip': it ? `${it.name}\n${rbpEqText(it)}` : 'Click to pick a part for this slot',
        onclick: (ev) => { rbpEqFor = ev.currentTarget; rbpEqSlot = i; } }, it ? (rbpEqPic(it.id, 40) || h('span', { class: 'rbp-skillpic rbp-skillnone', text: '?' })) : h('span', { class: 'rbp-skillpic rbp-skillnone rbp-plus', 'aria-hidden': 'true' }));   // the plus is drawn (not a "+" glyph, which sits low), so it is in the exact middle
      const feat = T ? h('span', { class: 'rbp-eqfeat' + (it ? ' covered' : ''), 'data-tip': T.desc || null, tabindex: T.desc ? 0 : null }, T.name) : h('span', { class: 'rbp-eqfeat none', text: 'no feature' });
      const text = it
        ? h('span', { class: 'rbp-skilltext' }, h('b', { text: it.name }), h('span', { class: 'src-count' + (it.descJa ? ' tl-name' : ''), 'data-tip': it.descJa ? TL_DESC_TIP(it.descJa) : null, text: rbpEqText(it) }),
          T ? h('small', { class: 'rbp-skillfrom' }, 'in place of ', feat) : h('small', { class: 'rbp-skillfrom', text: 'in a slot with no feature' }), block ? h('small', { class: 'rbp-eqbad', text: block.fixed ? 'This feature can’t be covered' : `Can’t be worn with ${block.clash} (${block.group})` }) : null,
          h('small', { class: 'rbp-skillfrom got', text: rbpEqWhere(it.id) }))
        // the feature as it is: its name and the game's description of it (user, 2026-11-27: not just "its own feature")
        : h('span', { class: 'rbp-skilltext' }, T ? h('b', { text: T.name }) : h('span', { class: 'src-count', text: 'No feature in this slot' }),
          T ? h('span', { class: 'src-count rbp-eqdesc', text: T.desc ? T.desc.replace(/\s*\n\s*/g, ' ').trim() : 'its own feature (the game data has no description)' }) : h('small', { class: 'rbp-skillfrom', text: 'a part here adds its skill' }));
      return h('li', { class: 'rbp-skillrow rbp-eqrow' + (it ? ' swapped' : '') }, slot, text);   // no slot number (user, 2026-11-27): the square leads the row
    });
    card.append(h('ul', { class: 'gdb-list rbp-skilllist rbp-eqlist' }, rows));
    return card;
  }
  // the part picker: a popover beside the slot clicked, with a search over every part
  const rbpEqMenu = $('#rbp-equip-menu');
  let rbpEqFor = null, rbpEqSlot = 0;
  function drawRbpEquip() {
    const list = $('#rbp-equip-list'), q = ($('#rbp-equip-search input').value || '').trim().toLowerCase(), words = q.split(/\s+/).filter(Boolean);
    const P = S.rbp.P, plan = rbpPlan(), D = rbpDemon(), byId = rbpEqById(P);
    if (!D || D.missing || !byId) { list.replaceChildren(); return; }
    const endFused = rbpEndFused(P, plan, D), traits = rbpTraitSlots(P, D, endFused), equip = rbpEquip(plan, D, endFused);
    const pick = (id) => { equip[rbpEqSlot] = id; plan.equip.touched = true; rbpSave(); rbpEqMenu.hidePopover(); render(); };
    const all = [...byId.values()].filter((it) => words.every((w) => it.hay.includes(w)));
    const rows = all.map((it) => ({ it, block: rbpEqBlock(P, byId, traits, equip, rbpEqSlot, it.id) }));
    const rank = (it) => (q && it.name.toLowerCase().replace(/^epitaph parts\s*/, '').replace(/^[[【]/, '').startsWith(q) ? 0 : 1);
    rows.sort((a, b) => (a.block ? 1 : 0) - (b.block ? 1 : 0) || rank(a.it) - rank(b.it) || a.it.name.localeCompare(b.it.name));
    const shown = rows.slice(0, 80);
    const tr = traits[rbpEqSlot], T = tr ? rbpTrait(tr) : null;
    const items = [];
    if (equip[rbpEqSlot]) items.push(h('button', { type: 'button', class: 'char', onclick: () => pick(0) }, h('span', { class: 'dm-icon dm-none dm-off', text: '–' }),
      h('span', { class: 'char-text' }, h('span', { class: 'char-name', text: T ? `Keep the feature ${T.name}` : 'Take the part off' }))));
    items.push(h('div', { class: 'list-head', text: T ? `Slot ${rbpEqSlot + 1}: in place of ${T.name}` : `Slot ${rbpEqSlot + 1}: no feature` }));
    shown.forEach(({ it, block }) => items.push(h('button', { type: 'button', class: 'char' + (block ? ' rbp-eqoff' : ''), 'aria-disabled': block ? 'true' : null, 'aria-current': equip[rbpEqSlot] === it.id ? 'true' : null,
      'data-tip': block ? (block.fixed ? 'This slot’s feature can’t be covered' : `Can’t be worn with ${block.clash} in slot ${block.slot + 1}: both are ${block.group}`) : it.desc || null,
      onclick: () => { if (!block) pick(it.id); } },
      h('span', { class: 'dm-icon rbp-dm-icon' }, rbpEqPic(it.id, 32)), h('span', { class: 'char-text' }, h('span', { class: 'char-name', text: it.name }), h('span', { class: 'char-line' + (it.descJa ? ' tl-name' : ''), text: rbpEqText(it) || it.skillName })),
      block ? h('span', { class: 'tag tag-off', text: block.fixed ? 'fixed feature' : `clashes · slot ${block.slot + 1}` }) : equip.includes(it.id) ? h('span', { class: 'tag', text: 'worn' }) : null)));
    list.replaceChildren(...items);
    const free = rows.filter((r) => !r.block).length;
    $('#rbp-equip-note').textContent = !all.length ? 'No part matches.' : `${free} of ${all.length} can go here${rows.length > shown.length ? `; ${shown.length} shown: type more to narrow it` : ''}.`;
    simPlace(rbpEqMenu, rbpEqFor);
  }
  rbpEqMenu.addEventListener('toggle', (ev) => {
    if (ev.newState !== 'open') return;
    const inp = $('#rbp-equip-search input'); inp.value = ''; drawRbpEquip(); inp.focus();
  });
  $('#rbp-equip-search input').addEventListener('input', () => drawRbpEquip());
  $('#rbp-equip-search input').addEventListener('keydown', (ev) => { if (ev.key === 'Enter') { const b = $('#rbp-equip-list .char:not([aria-disabled])'); if (b) b.click(); ev.preventDefault(); } else if (ev.key === 'ArrowDown') { const b = $('#rbp-equip-list .char'); if (b) { b.focus(); ev.preventDefault(); } } });
  rbpEqMenu.addEventListener('keydown', (ev) => {
    if (ev.key !== 'ArrowDown' && ev.key !== 'ArrowUp') return;
    const rows = [...rbpEqMenu.querySelectorAll('.char')], i = rows.indexOf(document.activeElement);
    const next = rows[ev.key === 'ArrowDown' ? Math.min(rows.length - 1, i + 1) : Math.max(0, i - 1)];
    if (next) { next.focus(); ev.preventDefault(); }
  });
  $('#main').addEventListener('scroll', () => { if (rbpEqMenu.matches(':popover-open')) rbpEqMenu.hidePopover(); });
  /** Whether the demon ends Mitama-fused: it is now, or the plan fuses it and the ranks before reach the 48 points. */
  function rbpEndFused(P, plan, D) {
    if (D.fused) return true;
    if (plan.view === 'boosts') return !!D.x.mt;   // "Pick boosts first" always plans the fusion
    if (!plan.fuse || !D.x.mt) return false;
    const onRune = P.runeOf.has(D.growth) ? P.runeOf.get(D.growth) : -1, defaultRune = P.runeOf.has(D.x.growth) ? P.runeOf.get(D.x.growth) : -1;
    const pre = rbpSteps(P, D.ranks, onRune, D.level, plan.pre, plan.last, 1);
    return rbpPoints(pre.ranks, defaultRune) >= 48;
  }
  function pageRebirth() {
    const U = S.rbp, page = h('div', { class: 'page page-rbp' });
    loadRebirth();
    if (U.view === 'home') return rbpHome(page);
    page.append(h('div', { class: 'page-head sim-head' }, rbpTitleBar()));
    page.append(pageHead('Plan a demon’s rebirths and its Mitama: set the rank of each rune, fuse it with a Mitama, fill its slots, and see what it costs and what the demon becomes.',
      h('div', { class: 'rbp-headright' },
        h('div', { class: 'seg', role: 'group', 'aria-label': 'How to plan' },
          [['runes', 'Plan the runes'], ['boosts', 'Pick boosts first']].map(([id, label]) =>
            h('button', { type: 'button', 'aria-pressed': String((rbpPlan().view || 'runes') === id), 'data-tip': id === 'runes' ? 'Set each rune’s rank, the fusion and the slots yourself' : 'Pick the Mitama set boosts you want; the ranks, Mitamas and costs follow from them (an experiment)',
              onclick: () => { rbpPlan().view = id; rbpSave(); render(); } }, label))),
        h('button', { type: 'button', class: 'btn btn-sm sim-import', 'aria-haspopup': 'dialog',
          'data-tip': 'Plan one of your COMP demons as last read from the game, with its rebirth ranks and Mitama slots. Nothing of the demon is changed',
          onclick: () => rbpImportPick() }, 'Import demon'),
        h('button', { type: 'button', class: 'btn btn-sm', 'data-tip': 'Back to no targets, no fusion, no Mitamas', onclick: () => { U.plan = rbpPlanFrom(null); rbpSave(); render(); } }, 'Reset the plan'))));
    if (U.state !== 'ready' || S.ddb.state !== 'ready' || S.fus.state === 'loading' || S.fus.state === 'idle' || S.dcm.state === 'loading' || S.dcm.state === 'idle') {
      page.append(h('p', { class: 'note', text: U.state === 'failed' ? 'The planner data could not be loaded (rebirth-planner.json is missing beside the page).' : S.ddb.state === 'failed' ? 'The demon list could not be loaded (demon-db.json is missing beside the page).' : 'Loading the rebirth tables…' }));
      return page;
    }
    const P = U.P, plan = rbpPlan(), d = S.ddb.data;
    const D = rbpDemon();
    page.append(rbpDemonCard(P, plan, d, D));
    if (!D || D.missing) {
      page.append(h('p', { class: 'note', text: D && D.missing ? `${D.missing.name} isn’t in the Demon Database, so it can’t be planned here.` : U.src === 'comp' ? 'Import one of your demons at the top right, or pick any demon above.' : 'Pick a demon above, or import one of your COMP demons at the top right, to start planning.' }));
      return page;
    }
    // ---- the phases: rebirths before the Mitama fusion (none for a demon already fused), the fusion, rebirths after it, the slots ----
    const defaultRune = P.runeOf.has(D.x.growth) ? P.runeOf.get(D.x.growth) : -1;
    const xpMult = 1 + Math.max(-99, plan.xpBonus) / 100;
    const onRune = P.runeOf.has(D.growth) ? P.runeOf.get(D.growth) : -1;
    if (plan.view === 'boosts') {
      page.append(rbpBoostsView(P, plan, D, { defaultRune, onRune, xpMult }));
      return page;
    }
    const main = h('div', { class: 'rbp-main' }), side = h('div', { class: 'rbp-side' });
    let pre = null, points, mitamaRank, fused = D.fused, kind = D.kind, postCur, postOn, postLevel, step = 1;
    main.append(rbpSkillsCard(P, plan, D, step++));   // the skills first (user, 2026-11-27), then the equipment of the demon it ends as
    main.append(rbpEquipCard(P, plan, D, rbpEndFused(P, plan, D), step++));
    if (!D.fused) {
      pre = rbpSteps(P, D.ranks, onRune, D.level, plan.pre, plan.last, xpMult);
      points = rbpPoints(pre.ranks, defaultRune);
      mitamaRank = Math.max(0, Math.floor((points - 48) / 4));
      main.append(rbpPreCard(P, plan, D, pre, { points, defaultRune, step: step++ }));
      fused = plan.fuse && points >= 48 && !!D.x.mt;
      kind = plan.kind;
      const mtRune = D.x.mt && P.runeOf.has(D.x.mt.growth) ? P.runeOf.get(D.x.mt.growth) : -1;
      postCur = Array(12).fill(0); postOn = mtRune; postLevel = 1;   // after the fusion the ranks reset and the form's own growth is current
      main.append(rbpFusionCard(P, plan, D, points, mitamaRank, step++));
    } else {
      mitamaRank = D.mitamaRank; points = 48 + mitamaRank * 4;
      postCur = D.ranks; postOn = onRune; postLevel = D.level;
    }
    main.append(rbpForceCard(P, plan, D, fused, step++));   // right after the fusion (user, 2026-11-27): the form it ends as decides how many slots open
    let post = null, slots = null;
    if (fused) {
      post = rbpSteps(P, postCur, postOn, postLevel, plan.post, plan.postLast, xpMult * 0.4);
      slots = rbpSlotsModel(P, plan, D, post.ranks, mitamaRank);
      main.append(rbpAfterCard(P, plan, D, post, slots, mitamaRank, { fusedNow: D.fused, step: step++ }));
    }
    main.append(rbpSummaryCard(P, plan, D, pre, post, fused, slots));
    side.append(rbpResultCard(P, plan, D, pre, post, fused, kind, mitamaRank, slots));
    page.append(h('div', { class: 'rbp-cols' }, main, side));
    return page;
  }
  /**
   * The other way round: from the Mitama set boosts the player wants, what the demon needs. Each picked set asks its
   * rune for Mitamas of each kind (sets on one rune combine by the highest count per kind), so the rune's rank after
   * the fusion is the slots that takes; the Mitamas of each kind over all runes set the Mitama rank the demon needs
   * (cap 12 + rank), so the rank points before the fusion (48 + 4 x rank), reached the cheap way: every rune raised
   * level by level, lowest ranks first.
   */
  function rbpDerive(P, plan, D, ctx) {
    const sets = P.mitama.sets.filter((s) => plan.picks.includes(s.id));
    if (!sets.length) return { sets, counts: [], perKind: [0, 0, 0, 0], most: 0, mitamaRank: 0, points: 48, preRanks: null, pre: null, postRanks: Array(12).fill(0), post: null, kind: D.kind || 1, M: null, warnings: [], conflicts: new Set(), fused: false, preLast: 0, postLast: 0 };
    const have = [...Array(12).keys()].map((i) => { const c = [0, 0, 0, 0]; for (let k = 0; k < 8; k++) { const id = D.slots[i * 8 + k]; if (id) c[rbpKindOf(id)]++; } return c; });
    const warnings = [], conflicts = new Set();
    const counts = [...Array(12).keys()].map((i) => {
      const mine = sets.filter((s) => s.rune === i);
      const need = RBP_KINDS.map((_, k) => Math.max(have[i][k], ...mine.map((s) => s.req[k])));
      const total = need.reduce((a, b) => a + b, 0);
      if (total > 8) { mine.forEach((s) => conflicts.add(s.id)); warnings.push(`${P.runes[i].name}: ${mine.map((s) => rbpBoostText(plan.boost ? s.exText : s.text)).join(' and ')} need ${total} Mitamas together, but a rune holds 8.`); }
      return need;
    });
    const perKind = RBP_KINDS.map((_, k) => counts.reduce((s, c) => s + c[k], 0));
    const most = Math.max(...perKind);
    let mitamaRank = Math.max(0, most - 12);
    if (D.fused) {
      if (most > 12 + D.mitamaRank) warnings.push(`${most} ${RBP_KINDS[perKind.indexOf(most)]} Mitamas need Mitama rank ${most - 12}; ${D.name} has rank ${D.mitamaRank} (${12 + D.mitamaRank} of a kind at most).`);
      mitamaRank = D.mitamaRank;
    } else if (mitamaRank > 12) warnings.push(`${most} ${RBP_KINDS[perKind.indexOf(most)]} Mitamas: 24 of a kind is the most a demon can hold (Mitama rank 12).`);
    // the picks that push a kind past the cap are marked, so they can be unpicked
    perKind.forEach((n, k) => { if (n > (D.fused ? 12 + D.mitamaRank : 24)) sets.filter((s) => s.req[k] > 0).forEach((s) => conflicts.add(s.id)); });
    mitamaRank = Math.min(12, mitamaRank);
    let points = 48 + 4 * mitamaRank;
    // ranks after the fusion: the slots each rune needs (a fused demon keeps what it has); the player can raise them
    const postFloor = counts.map((c, i) => Math.max(D.fused ? D.ranks[i] : 0, Math.min(8, c.reduce((a, b) => a + b, 0))));
    const postRanks = postFloor.map((f, i) => Math.max(f, Math.min(9, plan.boostPost[i])));
    // ranks before it: the cheap way to the points, raised further where the player asked (more points: a higher Mitama rank)
    let preRanks = null, pre = null, preFloor = null;
    if (!D.fused) {
      preFloor = D.ranks.slice();
      while (rbpPoints(preFloor, ctx.defaultRune) < points) {
        let best = -1;
        for (let i = 0; i < 12; i++) if (preFloor[i] < 8 && (best < 0 || preFloor[i] < preFloor[best])) best = i;
        if (best < 0) break;
        preFloor[best]++;
      }
      preRanks = preFloor.map((f, i) => Math.max(f, Math.min(9, plan.boostPre[i])));
      points = rbpPoints(preRanks, ctx.defaultRune);
      mitamaRank = Math.max(mitamaRank, Math.min(12, Math.floor((points - 48) / 4)));
      const raised = [...Array(12).keys()].filter((i) => preRanks[i] > D.ranks[i]);
      pre = rbpSteps(P, D.ranks, ctx.onRune, D.level, preRanks, raised.length ? raised[raised.length - 1] : 0, ctx.xpMult);
    }
    const kind = plan.kindAuto || D.fused ? (D.fused ? D.kind : Math.max(1, perKind.indexOf(most) + 1)) : plan.kind;
    const mtRune = D.x.mt && P.runeOf.has(D.x.mt.growth) ? P.runeOf.get(D.x.mt.growth) : -1;
    const postCur = D.fused ? D.ranks : Array(12).fill(0);
    const raisedPost = [...Array(12).keys()].filter((i) => postRanks[i] > postCur[i]);
    const post = D.x.mt ? rbpSteps(P, postCur, D.fused ? ctx.onRune : mtRune, D.fused ? D.level : 1, postRanks, raisedPost.length ? raisedPost[raisedPost.length - 1] : 0, ctx.xpMult * 0.4) : null;
    const fake = { slots: {}, boost: plan.boost };
    counts.forEach((c, i) => { fake.slots[i] = { counts: c.slice() }; });
    const M = post ? rbpSlotsModel(P, fake, D, post.ranks, mitamaRank) : null;
    if (!D.x.mt) warnings.push(`${D.name} has no Mitama-fused form, so it can’t take Mitamas.`);
    return { sets, have, counts, perKind, most, mitamaRank, points, preRanks, preFloor, pre, postRanks, postFloor, post, kind, M, warnings, conflicts, fused: !!D.x.mt, preLast: pre ? pre.steps.length && pre.steps[pre.steps.length - 1].rune : 0, postLast: post && post.steps.length ? post.steps[post.steps.length - 1].rune : 0 };
  }
  /** "Pick boosts first": the 48 set boosts to tick, then what they take, the costs and the stats. */
  function rbpBoostsView(P, plan, D, ctx) {
    const R = rbpDerive(P, plan, D, ctx);
    const main = h('div', { class: 'rbp-main' }), side = h('div', { class: 'rbp-side' });
    main.append(rbpSkillsCard(P, plan, D, 1));   // the skills first (user, 2026-11-27), then the equipment
    main.append(rbpEquipCard(P, plan, D, D.fused || !!D.x.mt, 2));   // this view always plans the Mitama fusion
    // ---- the picker ----
    const card = h('div', { class: 'card rbp-boosts' });
    const q = (S.rbp.boostQ || '').trim().toLowerCase();
    const search = h('input', { class: 'input', type: 'search', placeholder: 'Search a boost (fire, null, LB, regen…)', value: S.rbp.boostQ || '', 'aria-label': 'Search a boost', spellcheck: 'false',
      oninput: (ev) => { S.rbp.boostQ = ev.target.value; const words = ev.target.value.trim().toLowerCase().split(/\s+/).filter(Boolean); rows.querySelectorAll('.rbp-set').forEach((c) => { const hit = words.length && words.every((w) => c.dataset.hay.includes(w)); c.classList.toggle('hit', !!hit); c.classList.toggle('miss', !!words.length && !hit); }); } });
    card.append(h('div', { class: 'card-head' }, h('span', { class: 'rbp-title' }, h('i', { class: 'rbp-step', text: '3' }), h('span', { class: 'card-title', text: 'The boosts you want' })),
      plan.picks.length ? h('button', { type: 'button', class: 'btn btn-sm', onclick: () => { plan.picks = []; rbpSave(); render(); } }, 'Clear') : null));
    // the search, and at the end of its row the Iron Man tick (the expertise skill that makes every set stronger)
    card.append(h('div', { class: 'rbp-searchrow' }, h('div', { class: 'search' }, icon('search'), search),
      h('label', { class: 'rbp-check' }, h('input', { type: 'checkbox', checked: plan.boost ? true : null, onchange: (ev) => { plan.boost = ev.target.checked; rbpSave(); render(); } }), h('span', { 'data-tip': `From class 7 Mitama Demon Growth Science (“${P.mitama.boostName}”): every Mitama set gives its stronger bonus`, text: 'Iron Man' }))));
    const rows = h('div', { class: 'rbp-brows' });
    // a boost that would push a kind past what the demon can hold (24 of a kind at Mitama rank 12; a fused demon's
    // own cap), or its rune past 8 slots, can't be picked: the chip says why
    const maxKind = D.fused ? 12 + D.mitamaRank : 24;
    const blocked = (s) => {
      const picked = P.mitama.sets.filter((x) => plan.picks.includes(x.id)).concat(s);
      const haveOf = (i) => { const c = [0, 0, 0, 0]; for (let k = 0; k < 8; k++) { const id = D.slots[i * 8 + k]; if (id) c[rbpKindOf(id)]++; } return c; };
      const countsOf = (i) => RBP_KINDS.map((_, k) => Math.max(haveOf(i)[k], ...picked.filter((x) => x.rune === i).map((x) => x.req[k])));
      const rune = countsOf(s.rune).reduce((a, b) => a + b, 0);
      if (rune > 8) return `With it ${P.runes[s.rune].name} would need ${rune} Mitamas, but a rune holds 8`;
      const totals = RBP_KINDS.map((_, k) => P.runes.reduce((sum, _r, i) => sum + countsOf(i)[k], 0));
      const k = totals.findIndex((t) => t > maxKind);
      return k >= 0 ? `With it the demon would need ${totals[k]} ${RBP_KINDS[k]} Mitamas, more than the ${maxKind} it can hold${D.fused ? ` at Mitama rank ${D.mitamaRank}` : ' (24 at Mitama rank 12)'}` : null;
    };
    P.runes.forEach((Ru, i) => {
      const mine = P.mitama.sets.filter((s) => s.rune === i);
      const chips = mine.map((s) => {
        const on = plan.picks.includes(s.id), full = rbpBoostText(plan.boost ? s.exText : s.text), text = full;
        const why = on ? null : blocked(s);
        const hay = `${text} ${full} ${rbpBoostText(s.text)} ${rbpBoostText(s.exText)} ${s.text} ${s.exText} ${Ru.name} ${s.req.map((r, k) => (r ? RBP_KINDS[k] : '')).join(' ')}`.toLowerCase();
        const chip = h('button', { type: 'button', class: 'rbp-set rbp-pick' + (on ? ' on' : '') + (R.conflicts.has(s.id) ? ' over' : '') + (why ? ' nofit' : ''), 'aria-pressed': String(on), 'data-hay': hay, disabled: why ? true : null,
          'data-tip': why || `${on ? 'Picked' : 'Pick it'}: ${full} (${s.req.map((r, k) => (r ? `${r} ${RBP_KINDS[k]}` : null)).filter(Boolean).join(', ')} on ${Ru.name})`,
          onclick: () => { plan.picks = on ? plan.picks.filter((id) => id !== s.id) : plan.picks.concat(s.id); rbpSave(); render(); } },
          h('span', { class: 'rbp-set-req' }, s.req.map((r, k) => (r ? h('i', { style: `--k:var(--mt-${k + 1})`, class: on ? 'met' : '', text: r }) : null))), h('span', { class: 'rbp-set-text', text }));
        if (q) { const hit = q.split(/\s+/).every((w) => hay.includes(w)); chip.classList.toggle('hit', hit); chip.classList.toggle('miss', !hit); }
        return chip;
      });
      const row = h('div', { class: 'rbp-brow' }, h('span', { class: 'rbp-rname' }, rbpRuneIcon(i), h('span', { class: 'rbp-rune', 'data-tip': Ru.text, tabindex: 0, text: Ru.name })), h('div', { class: 'rbp-sets rbp-setgrid' }, chips));
      rows.append(row);
    });
    card.append(rows);
    main.append(card);
    // ---- what it takes ----
    const take = h('div', { class: 'card rbp-take' });
    take.append(h('div', { class: 'card-head' }, h('span', { class: 'rbp-title' }, h('i', { class: 'rbp-step', text: '4' }), h('span', { class: 'card-title', text: 'What it takes' })),
      plan.picks.length && !R.warnings.length ? h('button', { type: 'button', class: 'btn btn-sm btn-primary', 'data-tip': 'Copy these ranks, the fusion and the Mitamas into the rune planner to fine-tune them',
        onclick: () => { if (R.preRanks) { plan.pre = R.preRanks.slice(); plan.last = R.preLast; } plan.fuse = true; plan.kind = R.kind; plan.post = R.postRanks.slice(); plan.postLast = R.postLast; plan.slots = {}; R.counts.forEach((c, i) => { plan.slots[i] = { counts: c.slice() }; }); plan.view = 'runes'; rbpSave(); render(); } }, 'Open in the rune planner') : null));
    if (!plan.picks.length) { take.append(h('p', { class: 'note', text: 'Tick the boosts you want above. The ranks, the Mitamas and the costs follow.' })); main.append(take); }
    else {
      R.warnings.forEach((w) => take.append(h('p', { class: 'note rbp-short', text: w })));
      // the Mitamas and the fusion
      take.append(h('div', { class: 'rbp-caps' }, RBP_KINDS.map((n, k) => { const v = R.perKind[k], cap = 12 + R.mitamaRank; return h('div', { class: 'rbp-cap' + (v > cap ? ' over' : v === cap ? ' full' : ''), style: `--k:var(--mt-${k + 1})`, 'data-tip': `${n} Mitamas the picked sets need` },
        h('div', { class: 'rbp-cap-top' }, h('div', { class: 'rbp-cap-text' }, h('span', { class: 'rbp-cap-name', text: n }), h('b', { text: `${v} / ${cap}` })), rbpKindFace(k)), h('div', { class: 'rbp-capbar' }, h('i', { style: `width:${Math.min(100, cap ? v / cap * 100 : 0)}%` }))); })));
      const K = P.mitama.kinds[R.kind - 1];
      take.append(h('div', { class: 'rbp-line' }, h('span', { class: 'q-key', text: D.fused ? 'Fused with' : 'Mitama fusion' }),
        D.fused ? h('span', { text: D.kind ? `${RBP_KINDS[D.kind - 1]} Mitama, rank ${D.mitamaRank}` : 'a Mitama, not read from the game' }) : h('span', { text: `at Mitama rank ${R.mitamaRank} (${R.points} rank points before it), with` }),
        K ? h('span', { class: 'rbp-kindtag', style: `--k:var(--mt-${R.kind})` }, rbpKindIcon(R.kind - 1), ` ${K.name}${plan.kindAuto ? ' (auto)' : ''}`) : h('span', { hidden: true }),
        !D.fused ? h('span', { class: 'src-count', text: `rank bonus at level ${plan.level}: ${(K.ranks[Math.min(12, R.mitamaRank)] || []).map(([, l, b]) => `${l} +${rbpRankBonus(b, plan.level)}`).join(', ') || 'none below Mitama rank 3 (56 rank points)'}` }) : null));
      // the ranks, before and after, as bars
      // the ranks the boosts need are the floor; a click on a pill or the stepper raises a rune past it (for its
      // rank bonus, more rank points before the fusion, more slots after it), stored in the plan
      const bars = (title, cur, floor, target, over, tip, fills) => h('div', { class: 'rbp-ranksbox' }, h('div', { class: 'q-key', text: title }), h('div', { class: 'rbp-ranks' }, P.runes.map((Ru, i) => {
        const set = (v) => { over[i] = Math.max(floor[i], Math.min(9, v)) > floor[i] ? Math.max(floor[i], Math.min(9, v)) : 0; rbpSave(); render(); };
        return h('div', { class: 'rbp-rrow rbp-rmini' + (target[i] > cur[i] ? ' up' : '') + (target[i] > floor[i] ? ' raised' : ''), 'data-tip': tip(i) + (target[i] > floor[i] ? ` · raised by you from ${floor[i]}` : '') },
          h('span', { class: 'rbp-rname' }, rbpRuneIcon(i), h('span', { class: 'rbp-rune', text: Ru.name })), rbpRankBar(cur[i], target[i], set, Ru, fills ? fills[i] : null), rbpStepper(target[i], floor[i], 9, set));
      })));
      const grid = h('div', { class: 'rbp-twobars' });
      if (R.pre) grid.append(bars(`Before the fusion: ${R.pre.total.rebirths} rebirths`, D.ranks, R.preFloor, R.preRanks, plan.boostPre, (i) => `${P.runes[i].name} to rank ${R.preRanks[i]}: the cheap way to ${R.points} rank points`));
      if (R.post) grid.append(bars(`${D.fused ? 'Ranks' : 'After the fusion'}: ${R.post.total.rebirths} rebirths`, D.fused ? D.ranks : Array(12).fill(0), R.postFloor, R.postRanks, plan.boostPost, (i) => `${P.runes[i].name} rank ${R.postRanks[i]}: ${R.counts[i].reduce((a, b) => a + b, 0)} Mitamas (${R.counts[i].map((c, k) => (c ? `${c} ${RBP_KINDS[k]}` : null)).filter(Boolean).join(', ') || 'none'})`,
        R.counts.map((c, i) => rbpSlotFills(R.have[i], c.map((v, k) => v - R.have[i][k])))));
      if (plan.boostPre.some(Boolean) || plan.boostPost.some(Boolean)) grid.append(h('button', { type: 'button', class: 'btn btn-sm rbp-unraise', onclick: () => { plan.boostPre = Array(12).fill(0); plan.boostPost = Array(12).fill(0); rbpSave(); render(); } }, 'Back to what the boosts need'));
      take.append(grid);
      if (R.M) {
        const need = R.M.pgs.map((n, k) => (n ? `${n} ${P.mitama.kinds[k].pgName || RBP_KINDS[k] + ' Mitama'}` : null)).filter(Boolean);
        take.append(h('p', { class: 'note', text: `Mitamas to place: ${need.length ? need.join(', ') : 'none'} (${rbpMacca(R.M.macca)}); ${R.M.activeSets.length} ${R.M.activeSets.length === 1 ? 'set' : 'sets'} on.` }));
      }
      main.append(take);
    }
    main.append(rbpForceCard(P, plan, D, D.fused || !!D.x.mt, 5));   // after what it takes (the fusion this view always plans)
    main.append(rbpSummaryCard(P, plan, D, R.pre, R.post, R.fused, R.M));
    side.append(rbpResultCard(P, plan, D, R.pre, R.post, R.fused, R.kind, R.mitamaRank, R.M));
    return h('div', { class: 'rbp-cols' }, main, side);
  }
  /** A small number box. */
  const rbpNum = (value, min, max, onchange, cls) => h('input', { class: 'input rbp-num ' + (cls || ''), type: 'number', min, max, step: 1, value, onchange: (ev) => { const v = Math.max(min, Math.min(max, Math.round(n0(ev.target.value)))); ev.target.value = v; onchange(v); } });
  /** [−] value [+] */
  function rbpStepper(value, min, max, onchange, opts) {
    const o = opts || {};
    const btn = (txt, dv, tip) => h('button', { type: 'button', class: 'rbp-stepbtn', disabled: (dv < 0 ? value <= min : value >= max) ? true : null, 'aria-label': tip, onclick: () => onchange(Math.max(min, Math.min(max, value + dv))) }, txt);
    return h('span', { class: 'rbp-stepper' + (o.cls ? ' ' + o.cls : ''), style: o.color ? `--k:${o.color}` : null }, btn('−', -1, 'Less'), h('b', { class: 'rbp-stepval', text: value }), btn('+', 1, 'More'));
  }
  const rbpFace = (x, mt) => { const pic = ddbFace(x); pic.classList.add('rbp-face'); if (mt && S.data) { pic.classList.add('rank-face'); mitamaMark(pic, true); } return pic; };
  /** The Mitama in each of a rune's slots, the ones it holds first (solid), then the ones to add (striped); null = empty. For the rank bar's colours. */
  function rbpSlotFills(have, added) {
    const cells = [];
    RBP_KINDS.forEach((_, k) => { for (let j = 0; j < (have[k] || 0); j++) cells.push({ k, add: false }); });
    RBP_KINDS.forEach((_, k) => { for (let j = 0; j < Math.max(0, added[k] || 0); j++) cells.push({ k, add: true }); });
    while (cells.length < 9) cells.push(null);
    return cells;
  }
  function rbpDemonCard(P, plan, d, D) {
    // The Overview's demon band: the picture, then the name as the dropdown (the planner's own menu: the COMP demons as
    // last seen, or with "Any demon" the Demon Database with a search), then what it is.
    const U = S.rbp, card = h('div', { class: 'card demon-card closed rbp-demon' });
    const x = D && !D.missing ? D.x : null;
    const pic = h('div', { class: 'demon-pic ' + (x ? alignClass(x.align) : 'none'), 'aria-hidden': 'true' }, h('span', { text: x ? String(D.name || '?').charAt(0) : '–' }));
    const src = D && D.comp && D.comp.icon ? D.comp.icon : x && x.icon ? x.icon : null;
    if (src) { const img = h('img', { src, alt: '' }); img.onerror = () => img.remove(); pic.append(img); }
    if (x && D.fused) mitamaMark(pic, true);
    const label = U.src === 'comp' && U.compState !== 'ready' ? 'Looking for your demons…' : D && !D.missing ? D.name : D && D.missing ? D.missing.name : 'Pick a demon';
    const name = h('button', { type: 'button', class: 'name-btn', popovertarget: 'rbp-demons', 'aria-haspopup': 'menu', 'data-tip': 'Pick any demon from the Demon Database (or import one of your COMP demons at the top right)',
      onclick: (ev) => { rbpDemonsFor = ev.currentTarget; } }, label, h('span', { class: 'chev down' }, icon('chevron')));
    const id = h('div', { class: 'demon-id' }, h('h3', { class: 'demon-name' }, name));
    if (x) {
      const gname = P.growthNames[String(D.growth)] || '', gja = (P.growthNamesJa || {})[String(D.growth)];
      id.append(h('span', { class: 'tag', text: `Lv ${D.level}` }), h('span', { class: 'tag', text: x.race }),
        h('span', { class: 'tag' + (gja ? ' tl-name' : ''), 'data-tip': 'G-Type: the growth type the demon is on' + (gja ? '. ' + TL_TIP(gja, 'growth type') : ''), text: gname || '—' }),
        D.fused ? (D.kind ? h('span', { class: 'tag rbp-kindtag', style: `--k:var(--mt-${D.kind})`, 'data-tip': 'M-Type: the Mitama it was fused with, and its Mitama rank', text: `${RBP_KINDS[D.kind - 1]} Mitama · rank ${D.mitamaRank}` })
          : h('span', { class: 'tag tag-warn', 'data-tip': 'It is Mitama-fused, but the game only listed it: which Mitama and its rank were not read, so their bonus is not counted. Open its status window in game and import it again.' }, icon('alert'), 'Mitama not read'))
          // not fused yet: the Mitama to fuse it with is picked here (the kind fixes which stats its Mitama rank bonus raises)
          : h('span', { class: 'rbp-fusewith', role: 'group', 'aria-label': 'Mitama to fuse with' }, h('span', { class: 'q-key', text: 'Fuse with' }),
            ...P.mitama.kinds.map((K) => h('button', { type: 'button', class: 'rbp-kindchip', style: `--k:var(--mt-${K.type})`, 'aria-pressed': String(!(plan.view === 'boosts' && plan.kindAuto) && plan.kind === K.type),
              'data-tip': `${K.demonName}: the Mitama rank bonus raises ${(K.ranks[12] || []).map(([, l]) => l).join(', ')}`, onclick: () => { plan.kind = K.type; plan.kindAuto = false; rbpSave(); render(); } }, rbpKindIcon(K.type - 1), K.name)),
            plan.view === 'boosts' ? h('button', { type: 'button', class: 'rbp-kindchip rbp-kindauto', 'aria-pressed': String(!!plan.kindAuto), 'data-tip': 'The kind the picked boosts use most', onclick: () => { plan.kindAuto = true; rbpSave(); render(); } }, 'Auto') : null),
        D.comp && D.known === false ? h('span', { class: 'tag tag-warn', 'data-tip': 'The game only listed this demon: its ranks and slots were not read. Open its status window in game and read the demons again.' }, icon('alert'), 'Ranks not read') : h('span', { hidden: true }),
        D.comp ? h('span', { class: 'tag tag-off', 'data-tip': 'Imported from your COMP: its rebirth ranks and Mitama slots are the ones last read from the game', text: 'From your COMP' }) : h('span', { hidden: true }));   // (append(null) would write the word "null")
    } else id.append(h('span', { class: 'tag tag-off', text: 'Demon Database' }));
    const inputs = h('div', { class: 'rbp-inputs' },
      h('label', { class: 'rbp-field', 'data-tip': 'The level the stats on the right are shown at' }, 'Stats at level', rbpNum(plan.level, 1, 99, (v) => { plan.level = v; plan.levelSet = true; rbpSave(); render(); })),
      h('label', { class: 'rbp-field', 'data-tip': 'Your XP bonus from gear and incense when you use the apples at Vivian: the apples’ XP goes up by it' }, 'XP bonus', rbpNum(plan.xpBonus, -90, 500, (v) => { plan.xpBonus = v; rbpSave(); render(); }), '%'));
    card.append(pic, h('div', { class: 'demon-main' }, id), inputs);
    return card;
  }
  /**
   * A rune's 9-rank bar: filled to the rank it has, outlined to the target; a click sets the target (a click on the
   * target pill lowers it by one). After the fusion a rank is also a slot: `fills` (rbpSlotFills) colours each pill
   * with the Mitama held or planned there.
   */
  function rbpRankBar(now, target, onpick, R, fills) {
    const bar = h('div', { class: 'rbp-bar', role: 'group', 'aria-label': `${R.name} rank` });
    for (let k = 1; k <= 9; k++) {
      const cls = k <= now ? 'now' : k <= target ? 'plan' : 'off';
      const g = R.ranks[k - 1];
      const f = fills && k <= Math.max(now, target) ? fills[k - 1] : null;
      const slot = f ? ` · ${RBP_KINDS[f.k]} Mitama ${f.add ? 'to add in this slot' : 'in this slot'}` : '';
      const tip = k <= now ? `Rank ${k}: has it${slot}`
        : k <= target ? `Rank ${k}: planned (level ${rbpLevelFor(k)}${g && g.items.length ? ', ' + g.items.map((it) => `${it[1]} ×${it[2]}`).join(' or ') : ''})${slot}${k === target ? ' · click to lower it' : ''}`
          : `Rank ${k}: click to plan it (level ${rbpLevelFor(k)})`;
      bar.append(h('button', { type: 'button', class: 'rbp-pill ' + cls + (f ? ' kind' + (f.add ? ' add' : '') : ''), style: f ? `--k:var(--mt-${f.k + 1})` : null, 'data-tip': tip, 'aria-label': tip, disabled: k <= now ? true : null, onclick: () => onpick(k === target ? Math.max(now, k - 1) : k) }));
    }
    return bar;
  }
  /** The rune's bonus at a target rank as a line ("CLS-R +28"), or '' below rank 2. */
  const rbpBonusAt = (R, t) => { const b = Math.max(0, t - 1); return t <= 1 ? '' : R.bonus.length ? R.bonus.map(([, label, v]) => `${label} +${v * b}`).join(', ') : R.mag ? `Summon magnetite −${Math.min(100, R.mag * b)}%` : ''; };
  /** The column titles over a phase's rows (the same grid as the rows: subgrid). [text, tip, spans two columns] */
  const rbpColHead = (cols) => h('div', { class: 'rbp-colhead', 'aria-hidden': 'true' }, ...cols.map(([text, tip, span2]) => h('span', { class: span2 ? 'span2' : null, 'data-tip': tip || null, text })));
  /** Quick fill: every rune to a rank (All to 4 is exactly the 48 points a fusion needs), or none. */
  function rbpPresets(targets, cur) {
    const set = (r) => { for (let i = 0; i < 12; i++) targets[i] = Math.max(cur[i], r); rbpSave(); render(); };
    return h('div', { class: 'rbp-quick' }, h('span', { class: 'q-key', text: 'Quick fill' }), h('div', { class: 'chips' },
      ...RBP_PRESETS.map((r) => h('button', { type: 'button', class: 'chip-btn', 'aria-pressed': String(targets.every((t, i) => t === Math.max(cur[i], r))), 'data-tip': r === 4 ? 'Every rune to rank 4: exactly the 48 rank points a Mitama fusion needs' : `Every rune to rank ${r}`, onclick: () => set(r) }, `All to ${r}`)),
      h('button', { type: 'button', class: 'chip-btn', 'data-tip': 'No rebirths in this phase', onclick: () => { for (let i = 0; i < 12; i++) targets[i] = 0; rbpSave(); render(); } }, 'Clear')));
  }
  /** "Ends on": a radio on each raised row; the demon keeps the growth type of the rune its last rebirth is on. */
  function rbpEndBtn(plan, lastKey, i, R, up, D, g) {
    if (!up) return h('span', { class: 'rbp-endph' });   // the same height as the radio, so a row never bounces when a rune is raised or cleared
    const on = plan[lastKey] === i, growth = g ? g.name : R.name;
    return h('button', { type: 'button', role: 'radio', class: 'rbp-endbtn', 'aria-checked': String(on),
      'data-tip': on ? `${D.name} ends on ${R.name}: its last rebirth is this rune’s, so its stats follow ${growth}` : `End on ${R.name}: the stats would follow ${growth}`,
      onclick: () => { plan[lastKey] = i; rbpSave(); render(); } }, h('i', { class: 'rbp-radio' }), h('span', { text: on ? 'ends here' : '' }));
  }
  /** "The N steps and items": a phase's rebirths one by one, and the runestones they take (per rune). */
  function rbpStepsDetails(P, res, key) {
    if (!res.steps.length) return null;
    const items = new Map();   // the first listed item of each rank, as the default choice, per rune
    const pay = rbpPay();   // the items follow the "pay with" choice of the cost card
    res.steps.forEach((st) => { const it = rbpStepItem(st, pay); if (!it) return; if (!items.has(st.rune)) items.set(st.rune, new Map()); const m = items.get(st.rune); m.set(it[1], (m.get(it[1]) || 0) + it[2]); });
    const open = S.rbp[key];
    const det = h('div', { class: 'rbp-details' },
      h('button', { type: 'button', class: 'btn btn-sm', 'aria-expanded': String(!!open), onclick: () => { S.rbp[key] = !open; render(); } }, open ? 'Hide the steps' : `The ${res.steps.length} steps and items`));
    if (open) {
      if (items.size) det.append(h('div', { class: 'rbp-items' }, h('span', { class: 'q-key', 'data-tip': 'The runestones of each rank (a rank also accepts incense, soul stones, relics or a Primordial Runestone instead; the steps list them).', tabindex: 0, text: 'Items' }),
        h('ul', { class: 'rbp-item-list' }, [...items].map(([rune, m]) => h('li', null, h('b', { text: P.runes[rune].name + ': ' }), [...m].map(([n, a]) => `${n} ×${a}`).join(', '))))));
      det.append(h('ol', { class: 'rbp-steps' }, res.steps.map((st) => h('li', null,
        h('b', { text: st.enter ? `Switch to ${P.runes[st.rune].name}` : `${P.runes[st.rune].name} to rank ${st.rank}` }),
        h('span', { class: 'src-count', text: st.enter ? ` · rebirth into “${st.growth ? st.growth.name : P.runes[st.rune].name}” · ${rbpMacca(st.macca)}` : ` · level ${st.level} (${fmt(st.apples)} apples) · ${rbpMacca(st.macca)}${st.items.length ? ' · one of: ' + st.items.map((it) => `${it[1]} ×${it[2]}`).join(', ') : ''}` })))));
    }
    return det;
  }
  /** A rune's name cell: its glyph, its name (the rune's text as a tip) and what follows. */
  const rbpRuneCell = (R, i, ...rest) => h('div', { class: 'rbp-rname' }, rbpRuneIcon(i), h('span', { class: 'rbp-rune', 'data-tip': R.text, tabindex: 0, text: R.name }), ...rest);
  /** The pictures of the cost numbers: a cluster of orbs (four runes, four Mitamas) or one item icon (icons.json). */
  const rbpOrbs = (srcs, cls) => { const got = srcs.filter(Boolean); return got.length ? h('span', { class: `slot-icon orbs orbs-${got.length} rbp-orbs ${cls || ''}`, 'aria-hidden': 'true' }, got.map((src) => h('img', { src, alt: '' }))) : null; };
  const rbpCostIcon = (src, cls) => (src ? h('img', { class: 'rbp-costicon ' + (cls || ''), src, alt: '', 'aria-hidden': 'true' }) : null);
  const rbpIcons = () => (S.data && S.data.icons) || {};
  /** A phase total as a tile: the picture, the number, the word, and a second line when there is one. */
  const rbpSumTile = (pic, n, label, o) => h('div', { class: 'rbp-sumtile', 'data-tip': o && o.tip ? o.tip : null }, pic || h('span', { class: 'rbp-sumblank' }), h('div', { class: 'rbp-sumtext' }, h('b', { text: fmt(n) }), h('span', null, label, o && o.sub ? h('span', { class: 'rbp-sumsub', text: ` · ${o.sub}` }) : null)));   // one line, ellipsed; the tip has the rest (user, 2026-11-27)
  /** A phase's rebirths, apples and macca as tiles; `fused`: the demon is Mitama-fused in this phase (XP −60%, 2.5 × the apples); `slotMacca`: what placing the phase's Mitamas costs, added to the macca tile with the split under it (user, 2026-11-27: one macca tile, not two). */
  const rbpPhaseStats = (P, T, fused, slotMacca) => { const I = rbpIcons(); return [
    rbpSumTile(rbpOrbs([0, 1, 11].map((i) => I[`rune${i}`]), 'rbp-orbs-runes'), T.rebirths, T.rebirths === 1 ? 'rebirth' : 'rebirths'),
    rbpSumTile(rbpCostIcon(I.apple), T.apples, 'apples', { tip: `${P.apple.name}s at Vivian, 10,000 XP each plus your XP bonus${fused ? '. XP −60% once fused: a Mitama-fused demon gets 60% less XP, so 2.5 × the apples' : ''}`, sub: fused ? '× 2.5: XP −60% once fused' : null }),
    rbpSumTile(rbpCostIcon(I.macca, 'rbp-macca'), T.macca + (slotMacca || 0), 'macca', slotMacca ? { sub: `${fmt(T.macca)} rebirths + ${fmt(slotMacca)} Mitamas`, tip: `${fmt(T.macca)} macca from the rebirths (rank × 500 × level each); ${fmt(slotMacca)} macca from the Mitamas ((slots filled on that rune + 1) × 50,000 each)` } : { tip: 'Rank × 500 × level for each rebirth' })]; };
  /** Step 1 of a demon not fused yet: the rebirths before the Mitama fusion, a row per rune, then the rank points toward it. */
  function rbpPreCard(P, plan, D, res, info) {
    const targets = plan.pre, cur = D.ranks, lastKey = 'last';
    for (let i = 0; i < 12; i++) targets[i] = Math.max(0, Math.min(9, n0(targets[i])));
    const card = h('div', { class: 'card rbp-phase' });
    card.append(h('div', { class: 'card-head' }, h('span', { class: 'rbp-title' }, h('i', { class: 'rbp-step', text: info.step }), h('span', { class: 'card-title', text: 'Rebirths before the Mitama fusion' })), rbpPresets(targets, cur)));
    card.append(h('p', { class: 'note rbp-how', text: 'Click a pill to set the rank each rune should reach. Every rank above 1 adds the rune’s bonus, and the ranks add up to the rank points the Mitama fusion needs (48). Each rebirth sends the demon back to level 1.' }));
    const raised = [...Array(12).keys()].filter((i) => targets[i] > cur[i]);
    if (!raised.includes(plan[lastKey]) && raised.length) plan[lastKey] = raised[raised.length - 1];
    const showNow = cur.some(Boolean);
    const rows = h('div', { class: 'rbp-rows' + (showNow ? ' has-now' : '') });
    rows.append(rbpColHead([['Rune', 'Hover a name for what the game says about the rune'], showNow ? ['Now', 'The rank the demon has'] : null,
      ['Rank', 'Filled pills: the rank it has. Outlined: the rank planned. Click a pill, or − and +', true], ['At that rank', 'The rune’s bonus at the planned rank (hover for the growth type)'],
      ['Ends on', 'The rune of the last rebirth: the demon keeps that growth type, and its stats follow it']].filter(Boolean)));
    P.runes.forEach((R, i) => {
      const t = targets[i], up = t > cur[i], g = t >= 1 && R.ranks[t - 1] ? R.ranks[t - 1] : null;
      const set = (v) => { targets[i] = v; rbpSave(); render(); };
      rows.append(h('div', { class: 'rbp-rrow' + (up ? ' up' : '') + (up && plan[lastKey] === i ? ' end' : '') },
        rbpRuneCell(R, i, info.defaultRune === i ? h('span', { class: 'tag rbp-own', 'data-tip': 'The demon’s own growth belongs to this rune: it can go straight to rank 2, and counts as rank 1 for the Mitama fusion', text: 'own' }) : null),
        showNow ? h('b', { class: 'rbp-rnow', 'data-tip': `Rank ${cur[i]} now`, text: cur[i] }) : null,
        rbpRankBar(cur[i], t, set, R, null),
        rbpStepper(Math.max(cur[i], t), cur[i], 9, set),
        h('div', { class: 'rbp-rbonus', 'data-tip': g ? `Rank ${t}: ${g.name}` : null, text: rbpBonusAt(R, t) }),
        rbpEndBtn(plan, lastKey, i, R, up, D, g)));
    });
    card.append(rows);
    const pts = info.points, ok = pts >= 48, mr = Math.min(12, Math.floor((pts - 48) / 4));
    card.append(h('div', { class: 'rbp-sum' }, ...rbpPhaseStats(P, res.total, false),
      rbpSumTile(null, rbpConversion(P, res.ranks), 'conversion points', { tip: 'What the ranks are worth if extracted with a Rebirth Syringe' })));
    // the rank points toward the fusion: a bar with the 48 mark
    card.append(h('div', { class: 'rbp-points' + (ok ? ' ok' : ''), 'data-tip': 'Rank points toward the Mitama fusion: each rune counts up to 8 (the demon’s own rune 1 while it is 0). 48 allow the fusion; every 4 past it add a Mitama rank, up to 12 at 96' },
      h('div', { class: 'rbp-points-text' }, h('b', { text: `${pts} / 48` }), h('span', { text: ok ? `rank points · Mitama rank ${mr} at the fusion` : `rank points · ${48 - pts} more for the Mitama fusion` })),
      h('div', { class: 'rbp-ptsbar' }, h('i', { style: `width:${Math.min(100, pts / 96 * 100)}%` }), h('i', { class: 'mark', style: 'left:50%' }))));
    const det = rbpStepsDetails(P, res, 'preOpen');
    if (det) card.append(det);
    return card;
  }
  /** Step 2: the Mitama fusion. A switch plans it once the rank points are there; before that, a one-click way to get them. */
  function rbpFusionCard(P, plan, D, points, mitamaRank, step) {
    const card = h('div', { class: 'card rbp-fusion' });
    const ok = points >= 48 && !!D.x.mt, on = ok && plan.fuse;
    const sw = h('label', { class: 'rbp-switch' + (ok ? '' : ' off'), 'data-tip': !D.x.mt ? 'This demon has no Mitama form' : !ok ? 'Needs 48 rank points first' : on ? 'Planned: the ranks after it and the Mitamas follow' : 'Plan the fusion, the ranks after it and the Mitamas' },
      h('input', { type: 'checkbox', class: 'switch', role: 'switch', checked: on ? true : null, disabled: ok ? null : true, onchange: (ev) => { plan.fuse = ev.target.checked; rbpSave(); render(); } }), h('span', { text: on ? 'Planned' : 'Plan the fusion' }));
    card.append(h('div', { class: 'card-head' }, h('span', { class: 'rbp-title' }, h('i', { class: 'rbp-step', text: step }), h('span', { class: 'card-title', text: 'Mitama fusion' })), sw));
    if (!D.x.mt) { card.append(h('p', { class: 'note', text: `${D.name} has no Mitama form: the game can’t fuse it with a Mitama.` })); return card; }
    if (points < 48) {
      card.append(h('div', { class: 'rbp-gate' },
        h('p', { class: 'note', text: `The fusion needs 48 rank points; the rebirths above reach ${points}. Raise the ranks in step ${step - 1}, or take the shortest way:` }),
        h('button', { type: 'button', class: 'btn btn-sm', 'data-tip': 'Every rune to rank 4 (48 rank points, Mitama rank 0), and the fusion planned', onclick: () => { for (let i = 0; i < 12; i++) plan.pre[i] = Math.max(D.ranks[i], 4); plan.fuse = true; rbpSave(); render(); } }, 'Every rune to rank 4, then fuse')));
      return card;
    }
    if (!on) { card.append(h('p', { class: 'note', text: `${D.name} can be fused at Mitama rank ${mitamaRank}. Turn the switch on to plan the fusion, the ranks after it and the Mitamas.` })); return card; }
    const kinds = h('div', { class: 'rbp-kinds' }, P.mitama.kinds.map((K) => {
      const rb = (K.ranks[Math.min(12, mitamaRank)] || []).map(([, label, b]) => `${label} +${rbpRankBonus(b, plan.level)}`);
      return h('button', { type: 'button', class: 'rbp-kind', style: `--k:var(--mt-${K.type})`, 'aria-pressed': String(plan.kind === K.type), 'data-tip': `Fuse with ${K.demonName}`, onclick: () => { plan.kind = K.type; plan.kindAuto = false; rbpSave(); render(); } },
        h('span', { class: 'rbp-kind-name' }, rbpKindFace(K.type - 1), h('b', { text: K.name })), h('span', { class: 'src-count', text: rb.length ? rb.join(', ') : 'no rank bonus below Mitama rank 3' }));
    }));
    card.append(h('div', { class: 'rbp-row' }, h('span', { class: 'q-key', text: `Fuse with (the Mitama rank ${mitamaRank} bonus at level ${plan.level})` })), kinds);
    card.append(h('p', { class: 'note', text: `${D.name} becomes its Mitama-fused form: its own base stats, its ranks reset, ${12 + mitamaRank} Mitamas of each kind can be placed. 50,000 macca, or 2,500 Kreuz for a sure success. XP gained drops by 60%.` }));
    card.append(h('label', { class: 'rbp-check' }, h('input', { type: 'checkbox', checked: plan.boost ? true : null, onchange: (ev) => { plan.boost = ev.target.checked; rbpSave(); render(); } }), h('span', { 'data-tip': `From class 7 Mitama Demon Growth Science (“${P.mitama.boostName}”): every Mitama set gives its stronger bonus`, text: `I have “${P.mitama.boostName}” (boosted sets)` })));
    return card;
  }
  /** The slots as planned: per rune the counts held, asked and final, the sets on, the Mitamas to add and what they cost. */
  function rbpSlotsModel(P, plan, D, ranks, mitamaRank) {
    const have = [...Array(12).keys()].map((i) => { const c = [0, 0, 0, 0]; for (let k = 0; k < 8; k++) { const id = D.slots[i * 8 + k]; if (id) c[rbpKindOf(id)]++; } return c; });
    const rows = [], warnings = [], activeSets = [];
    let macca = 0; const pgs = [0, 0, 0, 0];
    P.runes.forEach((R, i) => {
      const cap = Math.min(8, ranks[i]), counts = rbpCounts(plan, i);
      let final = counts.map((v, k) => Math.max(have[i][k], v));
      let total = final.reduce((a, b) => a + b, 0);
      // never more Mitamas than the rune has slots: planned ones past them are dropped (Saki first); held ones stay
      const held = have[i].reduce((a, b) => a + b, 0);
      if (total > Math.max(cap, held)) { rbpTrimSlots({ final, counts, have: have[i] }, Math.max(cap, held)); final = counts.map((v, k) => Math.max(have[i][k], v)); total = final.reduce((a, b) => a + b, 0); }
      const added = final.map((v, k) => v - have[i][k]);
      let filled = have[i].reduce((a, b) => a + b, 0);
      added.forEach((n, k) => { for (let j = 0; j < n; j++) { macca += (filled + 1) * 50000; filled++; pgs[k]++; } });
      const sets = P.mitama.sets.filter((s) => s.rune === i);
      const on = sets.filter((s) => s.req.every((r, k) => final[k] >= r));
      activeSets.push(...on);
      if (total > cap) warnings.push(`${R.name} holds ${total} Mitamas but has ${cap} slots (rank ${ranks[i]}).`);
      rows.push({ i, cap, have: have[i], counts, final, total, added, sets, on });
    });
    const perKind = RBP_KINDS.map((_, k) => rows.reduce((s, r) => s + r.final[k], 0));
    perKind.forEach((n, k) => { if (n > 12 + mitamaRank) warnings.push(`${n} ${RBP_KINDS[k]} Mitamas: at most ${12 + mitamaRank} of a kind at Mitama rank ${mitamaRank}.`); });
    return { rows, perKind, cap: 12 + mitamaRank, warnings, activeSets, macca, pgs };
  }
  /** Lowering a rune's rank under the Mitamas planned on it drops the planned ones (never the held ones, Saki first) to fit. */
  function rbpTrimSlots(r, cap) {
    let total = r.final.reduce((a, b) => a + b, 0);
    for (let k = 3; k >= 0 && total > cap; k--) { const cut = Math.min(Math.max(0, r.counts[k] - r.have[k]), total - cap); r.counts[k] -= cut; total -= cut; }
  }
  /**
   * After the fusion (the last step; the only one for a demon already fused): the four kind caps, then a row per rune
   * with its rank (a rank is a slot: the pills wear the Mitamas), the Mitamas of each kind on it and its four sets.
   * A set or a Mitama that needs more slots raises the rank; lowering the rank drops the planned Mitamas past it.
   */
  function rbpAfterCard(P, plan, D, res, M, mitamaRank, info) {
    const targets = plan.post, cur = info.fusedNow ? D.ranks : Array(12).fill(0), lastKey = 'postLast';
    for (let i = 0; i < 12; i++) targets[i] = Math.max(0, Math.min(9, n0(targets[i])));
    const card = h('div', { class: 'card rbp-phase rbp-after' });
    card.append(h('div', { class: 'card-head' }, h('span', { class: 'rbp-title' }, h('i', { class: 'rbp-step', text: info.step }), h('span', { class: 'card-title', text: info.fusedNow ? 'Ranks and Mitamas' : 'After the fusion: ranks and Mitamas' })), rbpPresets(targets, cur)));
    card.append(h('p', { class: 'note rbp-how', text: `${info.fusedNow ? '' : 'The fusion resets the ranks. '}A rune holds one Mitama per rank (8 at most), and a set is on when the rune holds enough of each kind: a click on a set adds the Mitamas it needs and raises the rank to fit. The demon holds ${M.cap} of each kind at Mitama rank ${mitamaRank}.` }));
    card.append(h('div', { class: 'rbp-caps' }, RBP_KINDS.map((n, k) => { const v = M.perKind[k], over = v > M.cap; return h('div', { class: 'rbp-cap' + (over ? ' over' : v === M.cap ? ' full' : ''), style: `--k:var(--mt-${k + 1})`, 'data-tip': `${n} Mitamas on the demon, of the ${M.cap} its Mitama rank allows` },
      h('div', { class: 'rbp-cap-top' }, h('div', { class: 'rbp-cap-text' }, h('span', { class: 'rbp-cap-name', text: n }), h('b', { text: `${v} / ${M.cap}` })), rbpKindFace(k)), h('div', { class: 'rbp-capbar' }, h('i', { style: `width:${Math.min(100, v / M.cap * 100)}%` }))); })));
    const raised = [...Array(12).keys()].filter((i) => targets[i] > cur[i]);
    if (!raised.includes(plan[lastKey]) && raised.length) plan[lastKey] = raised[raised.length - 1];
    const showNow = cur.some(Boolean);
    const rows = h('div', { class: 'rbp-rows' + (showNow ? ' has-now' : '') });
    rows.append(rbpColHead([['Rune', 'Hover a name for what the game says about the rune'], showNow ? ['Now', 'The rank the demon has'] : null,
      ['Rank · slots', 'Filled pills: the rank it has. Outlined: planned. A coloured pill is a Mitama on that slot (striped: to add). Click a pill, or − and +. Under the bar: the Mitamas of each kind on the rune (− and +), and its four sets', true],
      ['At that rank', 'The rune’s bonus at the planned rank'],
      ['Ends on', 'The rune of the last rebirth: the demon keeps that growth type, and its stats follow it']].filter(Boolean)));
    const room = (k) => Math.max(0, M.cap - M.perKind[k]);
    M.rows.forEach((r) => {
      const i = r.i, R = P.runes[i], t = targets[i], up = t > cur[i], g = t >= 1 && R.ranks[t - 1] ? R.ranks[t - 1] : null;
      const setRank = (v) => { targets[i] = v; rbpTrimSlots(r, Math.min(8, Math.max(cur[i], v))); rbpSave(); render(); };
      const fit = (total) => { if (total > r.cap) targets[i] = Math.min(9, Math.max(targets[i], total)); };   // a Mitama past the rune's slots raises the rank to make room
      const steppers = h('div', { class: 'rbp-ksteps' }, RBP_KINDS.map((n, k) => h('span', { class: 'rbp-kstep', 'data-tip': `${n} Mitamas on ${R.name}${r.have[k] ? ` (${r.have[k]} on it now)` : ''}` }, rbpKindIcon(k),
        rbpStepper(r.final[k], r.have[k], Math.min(r.final[k] + room(k), 8 - (r.total - r.final[k])), (v) => { r.counts[k] = v; fit(r.total - r.final[k] + v); rbpSave(); render(); }, { color: `var(--mt-${k + 1})` }))));
      const chips = h('div', { class: 'rbp-sets rbp-setgrid' }, r.sets.map((s) => {
        const met = r.on.includes(s), need = s.req.map((q, k) => Math.max(0, q - r.final[k]));
        const total = r.total + need.reduce((a, b) => a + b, 0), raise = total > r.cap ? total : 0;
        const overKind = need.findIndex((q, k) => q > room(k)), over8 = total > 8;   // a rune holds 8: two sets that need more together can’t share it
        const full = rbpBoostText(plan.boost ? s.exText : s.text);
        const needText = need.map((q, k) => (q ? `${q} ${RBP_KINDS[k]}` : null)).filter(Boolean).join(', ');
        return h('button', { type: 'button', class: 'rbp-set' + (met ? ' on' : '') + (overKind >= 0 || over8 ? ' nofit' : ''), 'aria-pressed': String(met), disabled: met || overKind >= 0 || over8 ? true : null,
          'data-tip': met ? `On: ${full}` : over8 ? `With it ${R.name} would hold ${total} Mitamas, but a rune holds 8` : overKind >= 0 ? `Would need ${need[overKind]} more ${RBP_KINDS[overKind]}, but the demon can hold ${M.cap} of a kind at Mitama rank ${mitamaRank}` : `Click to add ${needText}${raise ? ` and raise ${R.name} to rank ${raise}` : ''}`,
          onclick: () => { s.req.forEach((q, k) => { if (r.final[k] < q) r.counts[k] = q; }); fit(total); rbpSave(); render(); } },
          h('span', { class: 'rbp-set-req' }, s.req.map((q, k) => (q ? h('i', { style: `--k:var(--mt-${k + 1})`, class: r.final[k] >= q ? 'met' : '', text: q }) : null))), h('span', { class: 'rbp-set-text', text: full }));
      }));
      rows.append(h('div', { class: 'rbp-rrow' + (up ? ' up' : '') + (r.total > r.cap ? ' over' : '') + (up && plan[lastKey] === i ? ' end' : '') },
        rbpRuneCell(R, i, r.total ? h('span', { class: 'src-count', 'data-tip': `${r.total} Mitamas on ${R.name}, ${r.cap} slots at rank ${Math.max(cur[i], t)}`, text: `${r.total} / ${r.cap}` }) : null),
        showNow ? h('b', { class: 'rbp-rnow', 'data-tip': `Rank ${cur[i]} now`, text: cur[i] }) : null,
        rbpRankBar(cur[i], t, setRank, R, rbpSlotFills(r.have, r.added)),
        rbpStepper(Math.max(cur[i], t), cur[i], 9, setRank),
        h('div', { class: 'rbp-rbonus', 'data-tip': g ? `Rank ${t}: ${g.name}` : null, text: rbpBonusAt(R, t) }),
        rbpEndBtn(plan, lastKey, i, R, up, D, g),
        h('div', { class: 'rbp-mrow' }, steppers, chips)));
    });
    card.append(rows);
    M.warnings.forEach((w) => card.append(h('p', { class: 'note rbp-short', text: w })));
    const I = rbpIcons();
    card.append(h('div', { class: 'rbp-sum' }, ...rbpPhaseStats(P, res.total, true, M.macca),
      rbpSumTile(rbpOrbs([0, 1, 2, 3].map((k) => I[`kind${k}`])), M.pgs.reduce((a, b) => a + b, 0), 'Mitamas to place', { tip: 'Each one costs (slots filled on that rune + 1) × 50,000 macca, counted in the macca tile' })));
    // ---- the Mitamas to get: a row per kind with how many, where they come from and the random bonus each one rolls ----
    if (M.pgs.some(Boolean)) {
      const list = h('div', { class: 'rbp-getlist' });
      M.pgs.forEach((n, k) => {
        if (!n) return;
        const K = P.mitama.kinds[k], singles = K.trades.filter((t) => t.pgs === 1), single = singles[0], SP = P.mitama.spirit;
        // the gems for n PGs at the first single-PG barter (10 of each gem); the server table also has one at 8 of each,
        // without saying where it is offered, so it only goes in the tip
        const perPg = (t) => t.gems.map((g) => `${g[2]} ${g[1]}`).join(' + ');
        const ways = [];
        if (single) ways.push(['Rag’s gem exchange', single.gems.map((g) => `${fmt(g[2] * n)} ${g[1]}`).join(' + '), `${perPg(single)} per ${K.pgName || 'PG'}${singles.length > 1 ? `. Rag’s Shop in Home III or Shinjuku Babel; its shop in Kakyo Castle asks ${singles.slice(1).map(perPg).join(' or ')} (20% off)` : ''}`]);
        if (SP && SP.each) ways.push(['Saien', `${fmt(SP.each * n)} ${SP.currency || 'Fighting Spirit'}`, `${n} × ${SP.name} at ${SP.each} ${SP.currency || 'Fighting Spirit'} each (${SP.pack[1]} for ${SP.pack[0]}): it summons a Mitama of the kind you pick. NewMoon only (${SP.where}), read in game; not in the server’s tables`]);
        list.append(h('div', { class: 'rbp-get', style: `--k:var(--mt-${k + 1})` },
          h('div', { class: 'rbp-get-head' }, rbpKindFace(k), h('b', { text: `${n} × ${K.pgName || RBP_KINDS[k] + ' Mitama'}` })),
          h('div', { class: 'rbp-get-ways' }, ways.map(([from, what, tip]) => h('span', { class: 'rbp-get-way', 'data-tip': tip || null, tabindex: tip ? 0 : null }, h('span', { class: 'q-key', text: from }), h('span', { text: what })))),
          h('div', { class: 'rbp-get-pool' }, h('span', { class: 'q-key', 'data-tip': 'Each placed Mitama rolls one of these; not counted in the stats on the right', text: 'Random bonus, one of' }), ...rbpPoolParts(K).map((p) => h('span', { class: 'rbp-poolchip', text: p })))));
      });
      card.append(h('div', { class: 'rbp-gets' }, h('span', { class: 'q-key', text: 'Mitamas to get' }), list,
        h('p', { class: 'note', text: `Or fuse a Mitama demon yourself from two different elementals.${P.mitama.spirit ? ` ${P.mitama.spirit.name} (${P.mitama.spirit.where}) is NewMoon’s own: one PG summons a Mitama of whichever kind you pick.` : ''}` })));
    }
    if (D.fused && D.slots.some(Boolean)) card.append(h('p', { class: 'note', text: 'Mitamas already on a rune are kept and count toward the sets; the game can only clear a whole rune (30,000 macca per slot).' }));
    return card;
  }
  /** "STR +2 to +5", "VIT +1 to +4", ... for a kind's random slot bonuses. */
  function rbpPoolParts(K) {
    const by = new Map();
    K.slots.forEach(([, , label, v]) => { const r = by.get(label) || [v, v]; r[0] = Math.min(r[0], v); r[1] = Math.max(r[1], v); by.set(label, r); });
    return [...by].map(([label, [lo, hi]]) => `${label} ${lo === hi ? `+${lo}` : `+${lo} to +${hi}`}`);
  }
  const FIGHTING_SPIRIT_ITEM = 12658;   // Saien's currency (its icon is exported with the PGs')
  // ---- paying for the rebirths: each rank takes ONE of several items (reference\reunion-growth.tsv), the same shape
  // for every rune (checked 2026-11-27):
  //   ranks 2-4: Small Runestone x1/5/20, Runestone x1, Large Runestone x1
  //   ranks 5-7: Runestone x1/5/20, Large Runestone x1/1/3, Incense (Small) x5/20/100
  //   ranks 8-9: Large Runestone x5/20, Soul Stone x3/5, a relic x1
  //   any rank:  Primordial Runestone x1
  // The cost card shows a column per rank with its options as tiles (the first rune's item stands for the category),
  // the player picks one per rank (user, 2026-11-27), and every rebirth to that rank, on any rune, takes that
  // category's item of its rune. Kept between visits (rbp-pay: rank -> category).
  const RBP_TIERS = { small: 'Small Runestone', mid: 'Runestone', large: 'Large Runestone', incense: 'Incense', soul: 'Soul Stone', relic: 'Relic', primordial: 'Primordial Runestone' };
  const rbpTier = (name) => (/^Primordial Runestone/.test(name) ? 'primordial' : /^Small Runestone/.test(name) ? 'small' : /^Large Runestone/.test(name) ? 'large' : /^Runestone/.test(name) ? 'mid' : /^Incense/.test(name) ? 'incense' : /^Soul Stone/.test(name) ? 'soul' : 'relic');
  function rbpPay() {
    if (!S.rbp.pay) {
      let p = null;
      try { p = JSON.parse(store.get('rbp-pay') || 'null'); } catch (e) { p = null; }
      S.rbp.pay = {};
      for (let r = 2; r <= 9; r++) S.rbp.pay[r] = p && RBP_TIERS[p[r]] ? p[r] : 'large';   // large runestones: one item at most ranks (an older shape starts over)
    }
    return S.rbp.pay;
  }
  const rbpPaySave = () => store.set('rbp-pay', JSON.stringify(S.rbp.pay));
  /** The item a step takes: the category picked for its rank (of the step's own rune), else the option with the fewest items. */
  function rbpStepItem(st, pay) {
    if (!st.items || !st.items.length) return null;
    return st.items.find((x) => rbpTier(x[1]) === pay[st.rank]) || st.items.slice().sort((a, b) => a[2] - b[2])[0];
  }
  /**
   * The whole plan's cost, under the steps (an experiment, 2026-11-27: it was the sticky card on the right): the totals
   * as tiles, then a step-by-step table that says what each step is (the runes raised and to what rank, the fusion's
   * form and Mitama rank, the Mitamas by kind), then the items to gather (every runestone with its icon, and the
   * Mitama PGs by either route).
   */
  function rbpSummaryCard(P, plan, D, pre, post, fused, M) {
    const card = h('div', { class: 'card rbp-summary rbp-costcard' });
    const I = rbpIcons();
    const raisedText = (ranks, from) => { const r = P.runes.map((R, i) => (ranks[i] > from[i] ? `${R.name} ${from[i] ? `${from[i]} → ` : ''}${ranks[i]}` : null)).filter(Boolean); return r.length ? `${r.length} ${r.length === 1 ? 'rune' : 'runes'} raised: ${r.join(', ')}` : 'no rune raised'; };
    const lines = [];   // [step, title, what, rebirths, apples, macca]
    if (pre && pre.steps.length) lines.push([3, D.fused ? 'Rebirths' : 'Rebirths before the fusion', `${raisedText(pre.ranks, D.ranks)}; the demon ends on ${P.runes[pre.on] ? P.runes[pre.on].name : '—'}${pre.growth ? ` (${pre.growth.name})` : ''}. Each rebirth takes the demon to level 1; the apples bring it back to the next rank’s level at Vivian.`, pre.total.rebirths, pre.total.apples, pre.total.macca]);
    if (fused && !D.fused) { const K = P.mitama.kinds[plan.kind - 1]; const pts = pre ? rbpPoints(pre.ranks, P.runeOf.has(D.x.growth) ? P.runeOf.get(D.x.growth) : -1) : 48; lines.push([4, 'Mitama fusion', `${D.name} fused with ${K ? K.demonName : 'a Mitama'} at the Cathedral: it becomes ${D.x.mt && D.x.mt.name ? D.x.mt.name : 'its Mitama form'} at Mitama rank ${Math.min(12, Math.floor((pts - 48) / 4))} (${pts} rank points). 50,000 macca, or 2,500 Kreuz for a sure success.`, 0, 0, 50000]); }
    if (post && post.steps.length) lines.push([D.fused ? 4 : 6, D.fused ? 'Rebirths' : 'Rebirths after the fusion', `${raisedText(post.ranks, D.fused ? D.ranks : Array(12).fill(0))}; the demon ends on ${P.runes[post.on] ? P.runes[post.on].name : '—'}${post.growth ? ` (${post.growth.name})` : ''}. A Mitama-fused demon gets 60% less XP, so 2.5 × the apples.`, post.total.rebirths, post.total.apples, post.total.macca]);
    if (M && M.pgs.some(Boolean)) lines.push([D.fused ? 4 : 6, 'Mitama slots', `${fmt(M.pgs.reduce((a, b) => a + b, 0))} Mitamas placed (${M.pgs.map((n, k) => (n ? `${n} ${RBP_KINDS[k]}` : null)).filter(Boolean).join(', ')}) for ${M.activeSets.length} ${M.activeSets.length === 1 ? 'set' : 'sets'}; each one costs (slots already filled on that rune + 1) × 50,000 macca.`, 0, 0, M.macca]);
    const rebirths = lines.reduce((s, l) => s + l[3], 0), apples = lines.reduce((s, l) => s + l[4], 0), macca = lines.reduce((s, l) => s + l[5], 0), mitamas = M ? M.pgs.reduce((a, b) => a + b, 0) : 0;
    card.append(h('div', { class: 'card-head' }, h('span', { class: 'card-title', text: 'What it all costs' }), h('span', { class: 'src-count', text: lines.length ? 'everything above, added up' : '' })));
    const forceRead = rbpForceRead(D), byForceId = rbpForceById(P);   // the Demon Force items to get (below)
    // no rebirth planned: the note, then still the skills and the demon equipment to get (under the totals' place)
    if (!lines.length) card.append(h('p', { class: 'note', text: 'No rebirths planned yet: set a rune’s target above.' }));
    else card.append(h('div', { class: 'rbp-sum rbp-sum-big' },
      rbpSumTile(rbpOrbs([0, 1, 11].map((i) => I[`rune${i}`]), 'rbp-orbs-runes'), rebirths, rebirths === 1 ? 'rebirth' : 'rebirths', { tip: 'Every rebirth of the plan, both phases' }),
      rbpSumTile(rbpCostIcon(I.apple), apples, 'apples', { tip: P.apple.name + 's at Vivian, 10,000 XP each plus your XP bonus' }),
      rbpSumTile(rbpCostIcon(I.macca, 'rbp-macca'), macca, 'macca', { tip: 'Rebirths, the fusion and the placed Mitamas together' }),
      mitamas ? rbpSumTile(rbpOrbs([0, 1, 2, 3].map((k) => I[`kind${k}`])), mitamas, 'Mitamas', { tip: 'Summon PGs to get, or Mitama demons to fuse' }) : null));
    // step by step: what each one is, and its three numbers
    const headPic = (pic, word) => h('th', { class: 'num', 'data-tip': word, 'aria-label': word }, pic || word);
    if (lines.length) card.append(h('table', { class: 'tbl rbp-costs rbp-costs-wide' }, h('thead', null, h('tr', null, h('th', { text: 'Step' }), h('th', { text: 'What' }),
      headPic(rbpOrbs([0, 1, 11].map((i) => I[`rune${i}`]), 'rbp-orbs-runes rbp-orbs-sm'), 'Rebirths'), headPic(rbpCostIcon(I.apple, 'rbp-costicon-sm'), 'Apples'), headPic(rbpCostIcon(I.macca, 'rbp-macca rbp-costicon-sm'), 'Macca'))),
      h('tbody', null, lines.map(([step, title, what, r, a, m]) => h('tr', null, h('td', { class: 'rbp-cost-step' }, h('span', null, h('i', { class: 'rbp-step', text: step }), h('b', { text: title }))), h('td', { class: 'rbp-cost-what', text: what }), h('td', { class: 'num', text: r ? fmt(r) : '' }), h('td', { class: 'num', text: a ? fmt(a) : '' }), h('td', { class: 'num', text: fmt(m) })))),
      h('tfoot', null, h('tr', null, h('td', { text: 'Total' }), h('td'), h('td', { class: 'num', text: fmt(rebirths) }), h('td', { class: 'num', text: fmt(apples) }), h('td', { class: 'num', text: fmt(macca) })))));
    // the items to gather: every runestone of both phases (first choice of each rank), then the Mitama PGs by either route
    const pay = rbpPay(), steps = [pre, post].filter(Boolean).flatMap((ph) => ph.steps);
    const need = new Map();
    steps.forEach((st) => { const it = rbpStepItem(st, pay); if (!it) return; const e = need.get(it[0]) || { id: it[0], name: it[1], n: 0 }; e.n += it[2]; need.set(it[0], e); });
    const itemPic = (id) => { const name = I[`item${id}`]; return name && S.meta.skillIcons ? h('img', { class: 'rbp-itemicon', src: S.meta.skillIcons + name + '.png', alt: '', loading: 'lazy', onerror: (ev) => ev.target.remove() }) : h('span', { class: 'rbp-itemicon rbp-itemicon-none' }); };
    const tile = (pic, name, count, tip) => h('div', { class: 'rbp-itemtile', 'data-tip': tip || name }, pic, h('span', { class: 'rbp-need-name', text: name }), h('b', { text: `×${fmt(count)}` }));
    const gather = h('div', { class: 'rbp-gather' });
    if (need.size) {
      // a column per rank: how many rebirths reach it, then its options as tiles (the first rune's item for the picture
      // and the name, the per-rune items in the tip); a click picks what every rebirth to that rank pays with
      const perRank = new Map();
      steps.forEach((st) => { if (st.items && st.items.length) perRank.set(st.rank, (perRank.get(st.rank) || 0) + 1); });
      const cols = [];
      for (let r = 2; r <= 9; r++) {
        const n = perRank.get(r) || 0, opts = P.runes[0].ranks[r - 1] ? P.runes[0].ranks[r - 1].items : [];
        if (!opts.length) continue;
        cols.push(h('div', { class: 'rbp-rankcol' + (n ? '' : ' idle') },
          h('div', { class: 'rbp-rankcol-head' }, h('b', { text: `Rank ${r}` }), h('span', { class: 'src-count', text: n ? `${n} ${n === 1 ? 'rebirth' : 'rebirths'}` : 'none planned' })),
          ...opts.map((it) => {
            const tier = rbpTier(it[1]), on = pay[r] === tier;
            const perRune = P.runes.map((R) => { const g = R.ranks[r - 1], x = g && g.items.find((y) => rbpTier(y[1]) === tier); return x ? `${R.name}: ${x[1]}` : null; }).filter(Boolean);
            const same = new Set(perRune.map((s) => s.slice(s.indexOf(': ') + 2))).size === 1;   // the item name itself may hold a colon ("Soul Stone: The Bull")
            return h('button', { type: 'button', class: 'rbp-rankopt', 'aria-pressed': String(on), 'data-tip': `${same ? it[1] : RBP_TIERS[tier]} ×${it[2]} for each rebirth to rank ${r}${n ? ` (×${fmt(it[2] * n)} in all)` : ''}${same ? '' : '. Per rune: ' + perRune.join(', ')}`,
              onclick: () => { pay[r] = tier; rbpPaySave(); render(); } },
              itemPic(it[0]), h('b', { text: `×${it[2]}` }));   // the icon and the amount only; the name is in the tip (user, 2026-11-27)
          })));
      }
      gather.append(h('div', { class: 'rbp-gather-group' },
        h('span', { class: 'q-key', 'data-tip': 'Each rebirth takes one of several items. Pick, for every rank, which one to pay with; the list under the columns follows', tabindex: 0, text: `Rebirth items · ${fmt([...need.values()].reduce((s, e) => s + e.n, 0))}` }),
        h('div', { class: 'rbp-rankgrid' }, ...cols),
        h('div', { class: 'rbp-itemgrid' }, [...need.values()].sort((x, y) => x.name.localeCompare(y.name)).map((e) => tile(itemPic(e.id), e.name, e.n)))));
    }
    if (mitamas) {
      const SP = P.mitama.spirit;
      const pgTiles = [];   // three per row: the PG, its two gems
      // a PG tile's picture is a cluster: the PG card and the Mitama it summons (Sacred Soul: the four, with the card on top)
      const pgSrc = (id) => (I[`item${id}`] && S.meta.skillIcons ? S.meta.skillIcons + I[`item${id}`] + '.png' : null);
      // a row per kind: the PG tile, then the gems that many cost at the first single-PG barter (user, 2026-11-27)
      M.pgs.forEach((n, k) => { if (!n) return; const K = P.mitama.kinds[k], single = K.trades.find((t) => t.pgs === 1);
        pgTiles.push(tile(rbpOrbs([pgSrc(K.pg), I[`face${k}`]], 'rbp-orbs-pg'), K.pgName || `${RBP_KINDS[k]} Mitama`, n, `${n} ${RBP_KINDS[k]} Mitamas to place`), ...(single ? single.gems.map((g) => tile(itemPic(g[0]), g[1], g[2] * n, `${g[2]} ${g[1]} per ${K.pgName || 'PG'}`)) : [])); });
      const soulTile = SP && SP.each ? tile(rbpOrbs([I.face0, I.face1, I.face2, I.face3, pgSrc(SP.item)], 'rbp-orbs-pg'), SP.name, mitamas, `${SP.name} summons a Mitama of the kind you pick: one per Mitama, instead of the kinds’ own PGs (NewMoon only, from ${SP.where})`) : null;
      gather.append(h('div', { class: 'rbp-gather-group' }, h('span', { class: 'q-key', 'data-tip': 'A Summon PG per Mitama, one way or the other: the kinds’ own PGs for gems at Rag’s, or NewMoon’s Sacred Soul for Fighting Spirit at Saien', tabindex: 0, text: `Mitamas · ${fmt(mitamas)}` }),
        // way 1: the kinds’ own PGs and the gems they cost at Rag’s; way 2: Sacred Souls and the Fighting Spirit they cost at Saien
        h('div', { class: 'rbp-gather-sub' }, h('span', { class: 'q-key', 'data-tip': 'One PG per Mitama of its kind: Rag’s Shop in Home III or Shinjuku Babel (10 of each gem per PG; Rag’s Shop in Kakyo Castle asks 8, 20% off)', tabindex: 0, text: 'At Rag’s Shop' }),
          h('div', { class: 'rbp-itemgrid rbp-pgrows' }, ...pgTiles)),
        soulTile ? h('div', { class: 'rbp-gather-sub' }, h('span', { class: 'q-key', 'data-tip': `${SP.where}: ${SP.each} per ${SP.name}, or ${SP.pack[1]} for ${SP.pack[0]}. NewMoon only; each PG summons the kind you pick`, tabindex: 0, text: 'Or at Saien (NewMoon)' }),
          h('div', { class: 'rbp-itemgrid rbp-pgrows' }, soulTile, tile(itemPic(FIGHTING_SPIRIT_ITEM), SP.currency || 'Fighting Spirit', SP.each * mitamas, `${fmt(mitamas)} × ${SP.name} at ${SP.each} each`))) : null,
        h('p', { class: 'note', text: 'Or fuse Mitama demons yourself from two different elementals.' })));
    }
    // the skills swapped onto the bar: each one's price as item pictures (the name and the amount in the tip), one line
    // per way to get it and where that is; then the demons that learn it (fusion)
    if (S.skl.state === 'ready') {
      const got = rbpSkills(plan, D).filter(Boolean).map((id) => ({ s: rbpSkill(id), from: rbpSkillFrom(id, D) })).filter((x) => x.from.how !== 'own');
      if (got.length) {
        let sp = 0;
        const GEM = (id) => id >= 701 && id <= 714;   // Rag's gems
        const price = (id, name, n) => h('span', { class: 'rbp-pricetile', 'data-tip': `${fmt(n)} × ${name}` }, itemPic(id), h('b', { text: `×${fmt(n)}` }));
        const lines = got.map((x) => {
          const barters = x.from.barters || [], gemBarters = barters.filter((b) => (b.items || []).length && b.items.every(([id]) => GEM(id)));
          const cheapest = gemBarters.length > 1 ? gemBarters.reduce((a, b) => (b.items[0][2] < a.items[0][2] ? b : a)) : null;
          const ways = barters.map((b) => {
            const parts = (b.items || []).map(([id, name, n]) => price(id, name, n));
            // Soul Points are the demon's own: it earns them in WILDCAT runs (demon-only dungeons; user, 2026-11-27), see notes\pages\rebirth-planner.md
            if (b.soul) { sp += b.soul; parts.push(h('span', { class: 'rbp-poolchip', 'data-tip': `${fmt(b.soul)} of the demon’s Soul Points (earned in WILDCAT runs, where the demon fights alone)`, text: `${fmt(b.soul)} Soul Points` })); }
            if (!parts.length) parts.push(h('span', { class: 'rbp-poolchip', text: b.cost }));
            // where: the gem lists are Rag's (the data agrees: "Rag in Rag's Shop"), with how a demon gets the skill; else who
            // opens the list and where, traced in the server data (skill-db.json `at`, npc_places: the Soul Points and Skill Orb
            // lists are the Demonic Researcher's in the Cathedral of Shadows, which the wiki's Demon Sacrifice page agrees with);
            // else the list is in the client's table only (user, 2026-11-27: "the data does not say which" was wrong for most)
            const gems = b.items && b.items.length && b.items.every(([id]) => GEM(id));
            const where = gems ? (b === cheapest ? 'Rag’s Shop, Kakyo Castle (20% off): taught to a summoned Elemental, then passed on by fusion' : 'Rag’s Shop, Home III or Shinjuku Babel: taught to a summoned Elemental, then passed on by fusion')
              : b.at ? b.at : 'An NPC exchange; no NPC in the server data offers it';
            const tip = !gems && b.at ? 'Who opens this exchange list, and where, traced in the server data' : null;
            // what the exchange asks of the demon (NPCBarterConditionData): its level, the skills it must not know (the orb's "incompatible" ones)...
            const needs = [b.lv ? `the demon at Lv ${b.lv}` : '', b.needs || ''].filter(Boolean).join('; ');
            return h('div', { class: 'rbp-way' }, h('span', { class: 'rbp-way-items' }, ...parts), h('span', { class: 'rbp-way-where', 'data-tip': tip, text: where }),
              needs ? h('span', { class: 'rbp-way-needs src-count', 'data-tip': 'What the exchange asks, from the game’s barter condition table', text: `Needs ${needs}` }) : null);
          });
          if (x.from.who && x.from.who.length) ways.push(h('div', { class: 'rbp-way' }, h('span', { class: 'rbp-way-where', text: `By fusion: ${x.from.who.slice(0, 4).join(', ')}${x.from.who.length > 4 ? ` and ${x.from.who.length - 4} more` : ''} learn it` })));
          return h('li', { class: 'rbp-skillcost' }, h('div', { class: 'rbp-skillrow' }, ddbImg(x.s.icon, 24, 'skill-ico'), h('span', { class: 'rbp-skilltext' }, sklName(x.s, 'b'), h('span', { class: 'src-count', text: rbpSkillTypeName(x.s) }))), h('div', { class: 'rbp-ways' }, ...ways));
        });
        gather.append(h('div', { class: 'rbp-gather-group' }, h('span', { class: 'q-key', 'data-tip': 'The skills on the bar that the demon does not learn itself: what each costs at an NPC (hover a picture for the item and how many), or who learns it for a fusion. The Soul Points of every priced skill are added up', tabindex: 0, text: `Skills to get · ${got.length}${sp ? ` · ${fmt(sp)} Soul Points` : ''}` }), h('ul', { class: 'gdb-list rbp-skilllist rbp-skillcosts' }, lines)));
      }
    }
    // the demon equipment planned that the demon does not wear yet: each part's picture and where it comes from
    {
      const byId = rbpEqById(P), endFused = rbpEndFused(P, plan, D);
      const worn = D.comp && Array.isArray(D.comp.equipment) ? D.comp.equipment.map(n0) : [];
      const parts = byId ? rbpEquip(plan, D, endFused).filter((id) => id && byId.has(id) && !worn.includes(id)).map((id) => byId.get(id)) : [];
      if (parts.length) gather.append(h('div', { class: 'rbp-gather-group' }, h('span', { class: 'q-key', 'data-tip': 'The Epitaph Parts planned in the equipment step that the demon does not wear yet, and where each comes from in the game (the Item Database has every way)', tabindex: 0, text: `Demon equipment to get · ${parts.length}` }),
        h('ul', { class: 'gdb-list rbp-skilllist rbp-skillcosts' }, parts.map((it) => h('li', { class: 'rbp-skillcost' },
          h('div', { class: 'rbp-skillrow' }, h('span', { class: 'skill-ico' }, rbpEqPic(it.id, 24)), h('span', { class: 'rbp-skilltext' }, h('b', { text: it.name }), h('span', { class: 'src-count' + (it.descJa ? ' tl-name' : ''), 'data-tip': it.descJa ? TL_DESC_TIP(it.descJa) : null, text: rbpEqText(it) }))),
          h('div', { class: 'rbp-ways' }, h('div', { class: 'rbp-way' }, h('span', { class: 'rbp-way-where', text: rbpEqWhere(it.id) }))))))));
    }
    // the Demon Force items to use: every slot the plan fills with an effect the demon doesn't have there yet, as item
    // tiles (one per item, with how many), where each comes from in the tip
    if (byForceId && plan.force) {
      const want = new Map();
      plan.force.slots.forEach((id, i) => { const e = id && id !== forceRead[i] ? byForceId.get(id) : null; if (e) { const w = want.get(e.item) || { e, n: 0 }; w.n++; want.set(e.item, w); } });
      if (want.size) {
        if (S.gdb.state === 'idle') loadGearDb();
        const n = [...want.values()].reduce((s, w) => s + w.n, 0);
        gather.append(h('div', { class: 'rbp-gather-group' }, h('span', { class: 'q-key', 'data-tip': 'The Demon Force items the bar asks for, one per slot it fills: use each on the summoned demon and put its effect in that slot', tabindex: 0, text: `Demon Force items · ${fmt(n)}` }),
          h('div', { class: 'rbp-itemgrid' }, [...want.values()].sort((a, b) => a.e.name.localeCompare(b.e.name)).map(({ e, n: k }) => tile(itemPic(e.item), e.name, k, `${e.name}: ${e.texts.join(', ')}\n${rbpForceWhere(e.item)}`)))));
      }
    }
    if (gather.children.length) card.append(gather);
    return card;
  }
  /** The stats the demon ends with, beside a fresh demon of the same level. */
  function rbpResultCard(P, plan, D, pre, post, fused, kind, mitamaRank, M) {
    const card = h('div', { class: 'card rbp-result' }), d = S.ddb.data, L = plan.level;
    const plain = ddbStats(D.x, L);
    const form = fused && D.x.mt ? D.x.mt : D.x;
    const ranks = post ? post.ranks : pre ? pre.ranks : D.ranks;
    const res = post || pre;
    const growth = res && res.growth ? res.growth.growth : D.growth;
    const mit = fused ? { kind, rank: mitamaRank, slots: D.fused ? D.slots.filter(Boolean) : [], sets: M ? M.activeSets : [], boost: plan.boost } : null;
    const owned = rbpOwnedSkills(P, plan, D, fused), FX = S.rbp.fx || {};
    const r = rbpStats(P, form.stats, growth, L, ranks, mit, owned.flatMap((o) => (FX[o.id] ? FX[o.id].adj : [])));
    const gname = P.growthNames[String(growth)] || `growth ${growth}`, gja = (P.growthNamesJa || {})[String(growth)];
    card.append(h('div', { class: 'card-head' }, h('span', { class: 'card-title', text: `Stats at level ${L}` }), h('span', { class: 'src-count' }, fused ? 'Mitama form · ' : '', tlName(gname, gja, 'growth type'))));
    const max = Math.max(1, ...d.statNames.map((n, i) => Math.max(plain[i], r.stats[i])));
    const rows = d.statNames.map((n, i) => { const a = plain[i], b = r.stats[i], delta = b - a; return h('tr', null, h('td', { text: n }), h('td', { class: 'num', text: fmt(a) }), h('td', { class: 'num rbp-planned', text: fmt(b) }), h('td', { class: 'num ' + (delta > 0 ? 'rbp-up' : delta < 0 ? 'rbp-down' : ''), text: delta ? fmt(delta, '', true) : '' })); });
    void max;
    card.append(h('table', { class: 'tbl rbp-stats' }, h('thead', null, h('tr', null, h('th', { text: 'Stat' }), h('th', { class: 'num', 'data-tip': 'A fresh demon of this level on its own growth: no rebirth, no Mitama, and none of its skills, features or equipment counted (Planned counts them)', text: 'Plain' }), h('th', { class: 'num', text: 'Planned' }), h('th', { class: 'num', text: 'Change' }))), h('tbody', null, rows)));
    const lines = [];
    if (r.mag) lines.push(`Summon magnetite −${r.mag}%`);
    if (lines.length) card.append(h('p', { class: 'note', text: lines.join(' · ') }));
    if (r.setLines.length) card.append(h('ul', { class: 'rbp-setlines' }, r.setLines.map(({ set, text }) => h('li', null, h('b', { text: P.runes[set.rune].name }), ` ${text}`))));
    card.append(rbpBoostList(owned, L));
    card.append(h('p', { class: 'note', text: 'No familiarity; the random bonus of each planned Mitama is not added. Switch skills are left out (whether they are on is up to you).' }));
    return card;
  }
  /**
   * What the planned demon holds that does something by itself: the passives on its skill bar, its features (the
   * Mitama form's when it ends fused) and the parts worn in their place (a part's skill stands in for the feature,
   * CharacterManager::GetTraitSkills). [{ id, kind: 'skill' | 'feature' | 'part', name }]
   */
  function rbpOwnedSkills(P, plan, D, endFused) {
    const out = [];
    rbpSkills(plan, D).forEach((id) => { if (id) out.push({ id, kind: 'skill', name: rbpSkill(id).name }); });
    const byId = rbpEqById(P);
    if (byId) {
      const traits = rbpTraitSlots(P, D, endFused), equip = rbpEquip(plan, D, endFused);
      traits.forEach((tr, i) => {
        const it = equip[i] ? byId.get(equip[i]) : null;
        if (it) out.push({ id: it.skill, kind: 'part', name: it.name });
        else if (tr) out.push({ id: tr, kind: 'feature', name: rbpTrait(tr).name });
      });
    }
    // the Demon Force effects in the demon's slots (the open ones: a slot the form doesn't open holds nothing); the id is "f<stack>" in rbp-effects.json
    const P2 = P && P.force ? rbpForceById(P) : null;
    if (P2) {
      const count = rbpForceCount(P, rbpFormType(D, endFused));
      rbpForce(plan, D).forEach((stack, i) => { if (stack && i < count) { const e = rbpForceEffect(P, P2, stack); out.push({ id: `f${stack}`, kind: 'Demon Force', name: e.name }); } });
    }
    return out;
  }
  // who a boost reaches (TokuseiManager on a partner demon: PARTNER and SELF stay on the demon, SUMMONER goes to you)
  const RBP_FX_GROUPS = [['demon', 'On the demon'], ['you', 'For you (its summoner)'], ['party', 'For the party']];
  const rbpFxGroup = (target) => (target === 'SUMMONER' ? 'you' : target === 'PARTY' ? 'party' : 'demon');
  /**
   * Every boost of the demon's skills, features and parts that the stats table does not show (attack and affinity
   * boosts, costs, cooldown, incantation, actions, resistances...), added up per kind of effect, grouped by who it
   * reaches; one that needs something (a time, a skill kind) says what. The tip names where each part comes from.
   */
  function rbpBoostList(owned, L) {
    const box = h('div', { class: 'rbp-fx' });
    if (S.rbp.fxState !== 'ready') { box.append(h('p', { class: 'note', text: S.rbp.fxState === 'failed' ? 'The skills’ effects could not be loaded (rbp-effects.json is missing beside the page).' : 'Loading the skills’ effects…' })); return box; }
    const FX = S.rbp.fx, by = new Map();
    owned.forEach((o) => {
      const S2 = FX[o.id];
      if (!S2) return;
      S2.fx.forEach(([label, value, pct, cond, note, target, lv, table]) => {
        if (table) return;   // in the table above
        const g = rbpFxGroup(target), n = (note || '').replace(/\s*;?\s*LvL × [\d.,]+/, '').replace(/^;\s*/, '').trim();
        const key = [g, label, pct, cond === 'always' ? '' : n, value === null ? 'x' : ''].join('|');
        const e = by.get(key) || { g, label, pct, cond, note: cond === 'always' ? '' : n, value: value === null ? null : 0, from: [] };
        if (value !== null && e.value !== null) e.value += lv ? value * L : value;
        e.from.push(`${o.name} (${o.kind === 'part' ? 'demon equipment' : o.kind}${value !== null ? `: ${fmt(lv ? value * L : value, pct ? '%' : '', true)}` : ''})`);
        by.set(key, e);
      });
    });
    const all = [...by.values()].filter((e) => e.value !== 0 || e.value === null);
    box.append(h('div', { class: 'card-head' }, h('span', { class: 'card-title', text: 'Boosts' }), h('span', { class: 'src-count', text: all.length ? 'skills, features, equipment and Demon Force' : '' })));
    if (!all.length) { box.append(h('p', { class: 'note', text: 'None beyond the stats above: the demon’s passives, features and parts change nothing else.' })); return box; }
    RBP_FX_GROUPS.forEach(([g, title]) => {
      const list = all.filter((e) => e.g === g).sort((a, b) => (a.cond === 'always' ? 0 : 1) - (b.cond === 'always' ? 0 : 1) || a.label.localeCompare(b.label));
      if (!list.length) return;
      box.append(h('div', { class: 'rbp-fxgroup' }, h('span', { class: 'q-key', text: title }),
        h('ul', { class: 'rbp-fxlist' }, list.map((e) => h('li', { class: e.cond === 'always' ? '' : 'cond', tabindex: 0, 'data-tip': `From ${e.from.join(', ')}${e.note ? `\n${e.note}` : ''}${e.cond === 'unparsed' ? '\nThe game data gives no number for it' : ''}` },
          h('span', { class: 'rbp-fxlabel', text: e.label }), e.value !== null ? h('b', { class: 'num', text: fmt(e.value, e.pct ? '%' : '', true) }) : null,
          e.note ? h('small', { class: 'rbp-fxnote', text: e.note }) : null)))));
    });
    return box;
  }

  const RENDER = { tracker: pageTracker, sim: pageSim, overview: pageOverview, bonuses: pageBonuses, gear: pageGear, expertise: pageExpertise, digitalize: pageDigitalize, history: pageHistory, dcm: pageDcm, quests: pageQuests, keys: pageKeys, geardb: pageGearDb, dungeons: pageDungeons, guides: pageGuides, fusion: pageFusion, dplan: pageDigiPlan, rebirth: pageRebirth, ddb: pageDemonDb, rank: pageRank, skills: pageSkills };
  // On the website: the character pages, and the tools that need the reader or the game, point to the desktop app.
  const DESKTOP_TOOLS = new Set(['tracker', 'sim']);
  function needsBabel() {
    const name = pageName(S.page);
    const what = isTool(S.page) ? (S.page === 'tracker' ? 'follows your quests in the running game' : 'works out your builds with BABEL’s own calculation')
      : 'shows your own character, read from the running game';
    return h('div', { class: 'page' }, h('div', { class: 'empty' },
      h('h2', { class: 'empty-title', text: `${name} needs BABEL` }),
      h('p', { class: 'page-sub', text: `This page ${what}. That needs BABEL, the desktop app, on your PC. The databases and the other planners work right here.` }),
      h('a', { class: 'btn btn-primary', href: RELEASES_URL, target: '_blank', rel: 'noopener' }, 'Download BABEL')));
  }
  function render() {
    renderChrome();
    const main = $('#main'), top = main.scrollTop;
    hideTip();
    main.replaceChildren(WEB && (!isTool(S.page) || DESKTOP_TOOLS.has(S.page)) ? needsBabel()
      : isTool(S.page) || (S.v && S.v.latest) ? RENDER[S.page]() : firstRun());
    main.scrollTop = top;
  }
  let resizeTimer = null;
  window.addEventListener('resize', () => { clearTimeout(resizeTimer); resizeTimer = setTimeout(() => { if (S.data && S.page === 'history') render(); }, 150); });

  // ---------- actions ----------
  async function reload() {
    const d = await host.call('load');
    S.data = { snapshots: d.snapshots || [], gear: d.gear || [], expertise: d.expertise || [], bonuses: d.bonuses || [], ranks: d.ranks || null, digitalize: d.digitalize || null, icons: d.icons || {}, unlocks: d.unlocks || [], summoned: d.summoned || null };
    S.meta = d.meta || {};
    if (!S.data.digitalize) S.mode = 'normal';   // no demon saved: there is nothing to be digitalized with (the choice itself is kept)
    Object.assign(S.xp, { state: 'idle', loadSeq: S.xp.loadSeq + 1 });   // the Expertise planner too (its plan is kept)
    S.sim.state = 'idle';   // the Build Planner starts again from what was just loaded (the build being tried is kept)

    simLoadBuilds();
    derive();
    render();
  }

  // ---------- characters: who is in game, whose data is shown ----------
  const chooser = $('#chooser');
  /** Look at the running game clients (read-only): who is logged in where. `{ keepKnown: true }` (the background
   *  check) looks again only at clients with no character yet; anything else (also a click event) looks at all. */
  async function scanClients(opts) {
    if (S.scanning) return;
    S.scanning = true; renderChrome(); drawChooser();
    try { S.state = await host.call('clients', { keepKnown: !!(opts && opts.keepKnown === true) }); S.lastScan = Date.now(); checkBuild(); }
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
    if (!inGame.length && !saved.length) list.append(h('p', { class: 'note', text: st.scanned ? 'Nobody is logged in. Start IMAGINE, log in, then look again.' : 'Looking…' }));
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
  let demonsFor = null, demonsLook = 0, demonsAsk = null;   // the button it hangs from; which look at the game is the latest; the demon asked about removing
  function placeDemons() {
    if (!demonsFor || !demonsFor.isConnected || !demonsMenu.matches(':popover-open')) return;
    const r = demonsFor.getBoundingClientRect(), w = demonsMenu.offsetWidth, hgt = demonsMenu.offsetHeight;
    demonsMenu.style.left = Math.max(8, Math.min(r.left, window.innerWidth - w - 8)) + 'px';
    demonsMenu.style.top = (r.bottom + 6 + hgt > window.innerHeight - 8 ? Math.max(8, r.top - 6 - hgt) : r.bottom + 6) + 'px';
  }
  const setMode = (mode) => { S.mode = mode; store.set('mode', mode); };
  /** The list as known now: "No demon", then every demon seen. `status`: a line under the load button. `keep` (a
   *  selector, or null): keep the list's scroll and focus that instead of the demon in use. */
  function drawDemons(status, loading, keep) {
    const list = $('#demons-list'), demons = S.demons || [];
    const had = list.contains(document.activeElement);
    const none = h('button', { type: 'button', class: 'char', 'aria-current': digi() ? null : 'true',
      onclick: () => { demonsMenu.hidePopover(); if (digi()) { setMode('normal'); render(); } } },
      h('span', { class: 'dm-icon dm-none dm-off', text: '–' }),
      h('span', { class: 'char-text' }, h('span', { class: 'char-name', text: 'No demon' }), h('span', { class: 'char-line', text: 'Not digitalized' })));
    // each demon as a card: what digitalizing with it would give (demonTile)
    const row = (d) => { const on = digi() && d.current; return demonTile(d, on,
      () => (on ? demonsMenu.hidePopover() : d.current ? (demonsMenu.hidePopover(), setMode('digi'), render())
        : d.loaded === false ? toast('warn', `${d.name} can’t be used yet`, 'The game only loads a demon’s stats and skills when you look at it. Click it in the COMP in game, or open the fusion window at the Cathedral of Shadows (the Master), which loads your whole COMP at once. Then press Load demons from the game.', 12000)
          : useDemon(d))); };
    // A cached demon (one the game no longer has: not in the COMP at the last read, not summoned, not in this read, and
    // not the one in use) can be taken off the list: a trash button on its card, then "Remove?". The others would come
    // back at the next read, so they have none.
    const item = (d) => {
      const tile = row(d);
      if (d.current || d.inComp === true || d.summoned || d.inGame) return tile;
      const ask = demonsAsk === d.key;
      // asking: the same card, tinted, with the question under its content (the list stays where it was)
      const at = (sel) => `.dg-item[data-key="${CSS.escape(d.key)}"] ${sel}`;
      return h('div', { class: 'dg-item' + (ask ? ' asking' : ''), 'data-key': d.key }, tile,
        ask ? h('div', { class: 'dg-ask' }, h('span', { class: 'dgt-line', text: `Remove ${d.name} from this list? It is no longer in your COMP, so what it gives can’t be read again.` }),
          h('span', { class: 'dg-ask-btns' },
            h('button', { type: 'button', class: 'btn btn-sm', onclick: () => forgetDemon(d) }, 'Remove'),
            h('button', { type: 'button', class: 'btn btn-ghost btn-sm dg-keep', onclick: () => { demonsAsk = null; drawDemons(null, false, at('.dg-del')); } }, 'Keep')))
          : h('button', { type: 'button', class: 'btn btn-ghost btn-square btn-sm dg-del', 'aria-label': `Remove ${d.name} from the list`, 'data-tip': 'Remove from this list',
            onclick: () => { demonsAsk = d.key; drawDemons(null, false, at('.dg-keep')); } }, icon('trash')));
    };
    const top = list.scrollTop;
    list.replaceChildren(none, ...demons.map(item));
    const load = $('#demons-load');
    load.disabled = !!loading;
    load.textContent = loading ? 'Reading the game…' : 'Load demons from the game';
    $('#demons-status').textContent = status || (!S.demons ? '' : !demons.length ? 'No demons seen yet. Open the COMP in game, then load them.' : '');
    placeDemons();
    if (keep !== undefined) {
      // asking about (or keeping) one demon: the list stays where it was; only when the card's question runs past the
      // bottom is it brought into view, by as little as it takes
      list.scrollTop = top;
      const el = keep && list.querySelector(keep);
      if (el) { el.focus({ preventScroll: true }); (el.closest('.dg-item') || el).scrollIntoView({ block: 'nearest' }); }
    } else if (had) { const b = list.querySelector('[aria-current="true"]') || list.querySelector('button'); if (b) b.focus(); }
  }
  /** The demons seen before (at once), or with `scan` a fresh look at the game added to them. */
  async function loadDemons(scan) {
    const look = ++demonsLook;
    drawDemons(null, scan);
    try {
      const demons = await host.call('compDemons', scan ? {} : { cached: true });
      if (look !== demonsLook) return;
      S.demons = demons;
      if (scan) refreshSummoned();
      // The game keeps every COMP demon in memory only while the COMP (or Demon List) is open; otherwise just the summoned one.
      const live = demons.filter((d) => d.inGame).length;
      const unread = demons.filter((d) => d.inGame && d.loaded === false).length;   // listed by the game without their details
      drawDemons(!scan ? null : !host.native ? 'Browser preview: stand-in demons.'
        : !live ? 'The game isn’t running, or has no demons loaded. The list is as before.'
        : live === 1 ? 'Only the summoned demon was loaded. Open the COMP in game, then load again for all of them.'
        : `${live} demons in your COMP.` + (unread ? ` ${unread} not read yet: click them in the COMP, or open the fusion window at the Cathedral of Shadows, then load again.` : ''));
    } catch (e) { if (look === demonsLook) drawDemons(String(e && e.message || e)); }
  }
  demonsMenu.addEventListener('toggle', (ev) => {
    if (ev.newState !== 'open') { demonsLook++; demonsAsk = null; return; }
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
  /** Take a demon off the list of demons seen (not the one in use). */
  async function forgetDemon(d) {
    try {
      await host.call('forgetDemon', { key: d.key });
      demonsAsk = null;
      if (S.demons) S.demons = S.demons.filter((x) => x.key !== d.key);
      drawDemons(null, false, null);   // the list stays where it was
      toast('ok', `${d.name} removed from the list`);
    } catch (e) { toast('err', `Couldn’t remove ${d.name}`, String(e && e.message || e), 0); }
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
    if (/ImagineClient is not running/i.test(message)) return ['The game isn\u2019t running', `Start IMAGINE, log in as ${who}, then take the snapshot again.`];
    if (/No running client has/i.test(message)) return [`${who} isn\u2019t logged in`, 'None of the open game clients has that character logged in. Log in, or click the name at the top left to switch to the character that is in game.'];
    if (/No character is selected/i.test(message)) return ['No character yet', 'Start IMAGINE and log in; the app reads the character\u2019s name from the game.'];
    if (/No saved stats yet/i.test(message)) return ['Take the first snapshot undigitalized', 'While digitalized the game counts the demon in your stats, so the first snapshot has to be taken when you are not digitalized (or with the Information window open).'];
    if (/Couldn't find the equipped gear/i.test(message)) return ['Open the Information window for this one', `The app couldn’t find ${who}’s gear list in the game’s memory while the Information window is closed. Open the window in game and take the snapshot again.`];
    if (/access is denied|access denied/i.test(message)) return ['Windows blocked reading the game', 'If the game runs as administrator, start this app as administrator too.'];
    return ['The snapshot didn\u2019t work', message];
  }

  // ---------- the assistant: Black Frost in the corner (assistant.js draws it) ----------
  // The host makes its model out of the game's own files the first time (so it must have seen the game once, to know
  // where it is installed). On or off is a preference of this window; ?assistant=0 leaves it out for one run and
  // ?assistant=1 puts it in (the app's --capture runs leave it out unless asked).
  const A = { el: $('#assistant'), view: null, state: 'off', note: null, swap: 0 };   // state: off | loading | on | missing | failed
  const assistantWanted = () => !WEB && Q.get('assistant') !== '0' && (Q.get('assistant') === '1' || store.get('assistant') !== 'off');
  // The picker (feature "assistant-picker"): another demon stands there instead, its model read from the game's files
  // when it is picked and kept in memory only. Without the feature it is always Black Frost.
  const PICKER = feature('assistant-picker');
  const pickedDemon = () => (PICKER ? Number(store.get('assistant-demon')) || 0 : 0);
  const assistantName = () => (pickedDemon() ? store.get('assistant-demon-name') || 'The demon' : 'Black Frost');
  // "Same as my summoned demon" (on unless switched off): the demon summoned in game stands there and changes when you
  // summon another. The game is read every few seconds (summonedDemon); while it can't be read, the demon summoned
  // when it was last read (a snapshot, or "Load demons from the game"). With none summoned, the last one stays.
  const following = () => PICKER && store.get('assistant-follow') !== 'off';
  async function followSummoned(now) {
    if (!following() || A.followBusy || (!now && document.hidden)) return;
    A.followBusy = true;
    try {
      let live = null;
      try { live = await host.call('summonedDemon'); } catch (e) { live = null; }
      const last = S.data && S.data.summoned && S.data.summoned.type ? { type: S.data.summoned.type, name: S.data.summoned.name } : null;
      const d = live && live.type > 0 ? live : !live ? last : null;   // live { type: 0 }: none out now, the last one stays
      if (!d || !following() || !A.view || A.state === 'loading') return;
      const want = d.type === 723 ? 0 : d.type;   // Black Frost is the default assistant
      if (want === pickedDemon() || d.type === A.followFailed) return;   // one whose model can't be read is tried once
      await switchAssistant({ id: d.type, name: d.name });
      if (pickedDemon() !== want) A.followFailed = d.type;
    } finally { A.followBusy = false; }
  }
  function fillAssistant() {
    const on = store.get('assistant') !== 'off', name = assistantName();
    $('#set-assistant').checked = on;
    $('#assistant-switch-label').textContent = `Show ${name} in the corner`;
    $('#assistant-view').setAttribute('aria-label', `${name}, the assistant. Click for a trick.`);
    $('#assistant-help').textContent = !on ? 'Off. Its model and animations come from your game’s own files.'
      : A.state === 'loading' ? `Getting ${name}’s model from the game’s files…`
      : A.state === 'missing' ? 'Its model comes from the game’s own files, and the app doesn’t know yet where the game is installed. Start IMAGINE and it will show up.'
      : A.state === 'failed' ? 'It couldn’t be drawn here: this window has no 3D graphics (WebGL).'
      : (A.note ? A.note + ' ' : '') + 'Its model and animations come from your game’s own files. Click it for a trick; it also reacts when you take a snapshot.';
    if (PICKER) fillAssistantPick();
  }
  // the picker: a search field listing the compendium's demons (demon-db.json, with their pictures)
  const AP = { list: null, loading: false, input: null };
  function fillAssistantPick() {
    const box = $('#assistant-pick');
    box.hidden = false;
    if (!AP.input) {
      const ph = 'Search a demon';
      const input = h('input', { class: 'input', type: 'search', placeholder: ph, 'aria-label': 'Assistant demon', spellcheck: 'false', autocomplete: 'off', role: 'combobox', 'aria-expanded': 'false' });
      const menu = h('div', { class: 'fus-menu', role: 'listbox', hidden: true });
      let hits = [], at = 0;
      const draw = () => {
        if (!AP.list) { menu.replaceChildren(h('div', { class: 'fus-opt empty', text: AP.loading ? 'Loading the demons…' : 'The demon list could not be loaded.' })); return; }
        const q = input.value.trim().toLowerCase();
        hits = AP.list.filter((d) => !q || d.hay.includes(q))
          .sort((a, b) => (a.lname.startsWith(q) ? 0 : 1) - (b.lname.startsWith(q) ? 0 : 1) || a.name.localeCompare(b.name) || a.level - b.level).slice(0, 60);
        at = Math.min(at, Math.max(0, hits.length - 1));
        menu.replaceChildren(...(hits.length ? hits.map((d, i) => h('div', { class: 'fus-opt' + (i === at ? ' on' : ''), role: 'option', 'aria-selected': String(i === at),
          onmousedown: (ev) => { ev.preventDefault(); pick(d); } },
          h('span', { class: 'dcm-icon', 'aria-hidden': 'true' }, d.icon ? h('img', { src: d.icon, alt: '', loading: 'lazy', onerror: (ev) => ev.target.remove() }) : null),
          h('span', { class: 'fus-opt-name', text: d.name }), h('span', { class: 'src-count', text: `${d.race} · Lv ${d.level}` })))
          : [h('div', { class: 'fus-opt empty', text: 'No demon matches.' })]));
        const on = menu.querySelector('.fus-opt.on'); if (on) on.scrollIntoView({ block: 'nearest' });
      };
      // Enter pressed before the list is in picks the first match once it is
      const listed = () => { draw(); if (AP.enterLater && !menu.hidden && hits[at]) { AP.enterLater = false; pick(hits[at]); } };
      const open = () => { menu.hidden = false; input.setAttribute('aria-expanded', 'true'); loadAssistantList(listed); draw(); };
      const close = () => { menu.hidden = true; input.setAttribute('aria-expanded', 'false'); };
      // a demon picked by hand stands there from now on: it no longer follows the summoned demon
      const pick = (d) => { close(); input.value = d.name; input.blur(); store.set('assistant-follow', 'off'); fillAssistant(); switchAssistant(d); };
      input.addEventListener('focus', () => { input.select(); open(); });
      input.addEventListener('input', () => { at = 0; open(); });
      input.addEventListener('blur', () => { close(); input.value = assistantName(); });
      input.addEventListener('keydown', (ev) => {
        if (ev.key === 'ArrowDown') { ev.preventDefault(); if (menu.hidden) open(); else { at = Math.min(at + 1, hits.length - 1); draw(); } }
        else if (ev.key === 'ArrowUp') { ev.preventDefault(); at = Math.max(at - 1, 0); draw(); }
        else if (ev.key === 'Enter') { if (!AP.list && AP.loading) { ev.preventDefault(); AP.enterLater = true; } else if (!menu.hidden && hits[at]) { ev.preventDefault(); pick(hits[at]); } }
        else if (ev.key === 'Escape') { if (!menu.hidden) { ev.stopPropagation(); close(); } }
      });
      input.addEventListener('blur', () => { AP.enterLater = false; });
      AP.input = input;
      AP.follow = h('input', { type: 'checkbox', class: 'switch', role: 'switch', id: 'set-assistant-follow' });
      AP.follow.addEventListener('change', () => { store.set('assistant-follow', AP.follow.checked ? 'on' : 'off'); fillAssistant(); if (AP.follow.checked) followSummoned(true); });
      AP.label = h('span', { class: 'field-help' });
      box.append(h('label', { class: 'switch-row' }, h('span', { text: 'Same as my summoned demon' }), AP.follow),
        AP.label, h('div', { class: 'fus-combo' }, h('div', { class: 'search' }, icon('search'), input), menu));
    }
    AP.follow.checked = following();
    AP.label.textContent = following() ? 'Or pick one yourself (it then stops following your summoned demon)' : 'Which demon stands there';
    if (document.activeElement !== AP.input) AP.input.value = assistantName();
    AP.input.disabled = A.state === 'loading';
  }
  function loadAssistantList(then) {
    if (AP.list || AP.loading) return;
    AP.loading = true;
    fetch('demon-db.json').then((r) => (r.ok ? r.json() : null)).catch(() => null).then((d) => {
      AP.loading = false;
      // every base demon and its Inexperienced variant; the other variants (Accomplished, Deformed, Vermillion…) are left out,
      // and so are demons without a picture (the game's files have no model for most of them either)
      AP.list = d && Array.isArray(d.demons) ? d.demons.filter((x) => x.icon && (!x.variantOf || /^Inexperienced /.test(x.name))).map((x) => ({ id: x.id, name: x.name, lname: x.name.toLowerCase(), race: x.race, level: x.level, icon: x.icon, hay: `${x.name} ${x.race}`.toLowerCase() })) : null;
      then();
    });
  }
  // Clicks go to the box it stands in (its canvas is bigger, for the animations, and lets clicks through).
  A.el.addEventListener('click', () => { if (A.view) A.view.poke(); });
  // a press on it starts no text selection (quick clicks would select the nearest text on the page, under it)
  A.el.addEventListener('mousedown', (ev) => ev.preventDefault());
  // Holding the right button and dragging sideways turns it around (left and right only); the way it is left facing is kept.
  let turning = null;
  A.el.addEventListener('contextmenu', (ev) => ev.preventDefault());
  A.el.addEventListener('pointerdown', (ev) => {
    if (ev.button !== 2 || !A.view) return;
    turning = { id: ev.pointerId, x: ev.clientX };
    A.el.setPointerCapture(ev.pointerId); A.el.classList.add('turning'); ev.preventDefault();
  });
  A.el.addEventListener('pointermove', (ev) => {
    if (!turning || ev.pointerId !== turning.id || !A.view) return;
    A.view.turn((turning.x - ev.clientX) * 0.8);   // its near side follows the pointer
    turning.x = ev.clientX;
  });
  const endTurn = (ev) => {
    if (!turning || ev.pointerId !== turning.id) return;
    turning = null; A.el.classList.remove('turning');
    if (A.view) store.set('assistant-yaw', String(Math.round(A.view.yaw)));
  };
  A.el.addEventListener('pointerup', endTurn);
  A.el.addEventListener('pointercancel', endTurn);
  // it sits on top of the page: the wheel over it still scrolls the page
  A.el.addEventListener('wheel', (ev) => { $('#main').scrollBy({ top: ev.deltaY }); }, { passive: true });
  /** A demon's model from the host: 0 is the default assistant (Black Frost). null when it can't be had. */
  async function assistantModel(demon) {
    try { return await host.call('assistant', demon ? { demon } : {}); } catch (e) { return null; }
  }
  /** Stand the model on the #assistant-view canvas; false when it can't be drawn. `place` moves the box to fit it. */
  function mountAssistant(model, place) {
    const yaw = parseFloat(store.get('assistant-yaw'));   // the way it was left facing
    let view = null;
    try { view = window.NMAssistant.mount($('#assistant-view'), model, Number.isFinite(yaw) ? { yaw } : {}); } catch (e) { view = null; }
    if (!view) return false;
    A.view = view;
    if (place !== false) placeAssistant();
    return true;
  }
  /** Far enough from the window's edges that its widest and lowest animation fits the way it faces now. (Turned
   *  another way an animation can reach further: its canvas has the room, and only the window's own edge can cut it.) */
  function placeAssistant() {
    if (!A.view) return;
    // Placed by how it stands idle, never closer to the edge than Black Frost is (44 px): a demon with a wide move (a
    // rush) would otherwise stand far from the corner; that move may be cut by the window's edge for a moment.
    document.body.style.setProperty('--assistant-right', Math.max(44, A.view.home.standR + 4) + 'px');
    // Raised at most as much as Black Frost needs (about 20 px): a big demon whose animation dips below its feet would
    // otherwise stand far above the window's bottom; its deepest pose may be cut by the window's edge for a moment.
    document.body.style.setProperty('--assistant-bottom', Math.max(2, Math.min(24, A.view.home.b)) + 'px');
  }
  /** Its canvas gave up its 3D context for good (dispose): the next model gets a new one. */
  function freshAssistantCanvas() {
    const old = $('#assistant-view'), fresh = old.cloneNode(false);
    fresh.removeAttribute('style'); fresh.className = ''; old.replaceWith(fresh);
  }
  async function startAssistant() {
    if (A.view || A.state === 'loading' || !window.NMAssistant) return;
    A.state = 'loading'; fillAssistant();
    let model = await assistantModel(pickedDemon());
    if (!model && pickedDemon()) {   // the picked demon's model can't be read (any more): Black Frost again
      A.note = `${assistantName()}’s model couldn’t be read from the game’s files, so Black Frost is back.`;
      store.set('assistant-demon', ''); store.set('assistant-demon-name', '');
      model = await assistantModel(0);
    }
    if (!assistantWanted()) { A.state = 'off'; fillAssistant(); return; }   // switched off while it loaded
    if (!model) { A.state = 'missing'; fillAssistant(); return; }
    A.el.hidden = false;
    if (!mountAssistant(model)) { A.el.hidden = true; A.state = 'failed'; fillAssistant(); return; }
    A.state = 'on'; document.body.classList.add('has-assistant'); fillAssistant();
  }
  function stopAssistant() {
    A.swap++;   // a switch still loading is dropped
    endSummon();
    if (A.view) A.view.dispose();
    A.view = null; A.state = 'off'; A.el.hidden = true; document.body.classList.remove('has-assistant');
    freshAssistantCanvas();
    fillAssistant();
  }
  // the game's summoning effect pieces, asked for once (null: the game's folder isn't known, the switch is plain)
  let summonFx;
  async function assistantFx() {
    if (summonFx === undefined) { try { summonFx = await host.call('assistantSummon'); } catch (e) { summonFx = null; } }
    return summonFx;
  }
  /** A summoning still playing: the old demon and the effect go at once (another pick, or switched off). */
  function endSummon() {
    const s = A.summoning;
    if (!s) return;
    A.summoning = null;
    s.arrive();
    s.fx.remove();
  }
  /** The picker chose a demon: read its model from the game's files and summon it in place of the one standing there. */
  async function switchAssistant(d) {
    const was = { id: store.get('assistant-demon') || '', name: store.get('assistant-demon-name') || '' };
    const isDefault = d.id === 723;   // Black Frost is the default assistant (the kept model file)
    store.set('assistant-demon', isDefault ? '' : String(d.id)); store.set('assistant-demon-name', isDefault ? '' : d.name);
    A.note = null;
    if (!A.view) { fillAssistant(); if (assistantWanted() && A.state !== 'loading') startAssistant(); return; }   // off: only remembered
    const token = ++A.swap;
    A.state = 'loading'; fillAssistant();
    const [model, fx] = await Promise.all([assistantModel(isDefault ? 0 : d.id), assistantFx()]);
    if (token !== A.swap) return;   // switched off, or another demon picked meanwhile
    if (!model) {
      store.set('assistant-demon', was.id); store.set('assistant-demon-name', was.name);
      A.state = 'on'; A.note = `${d.name}’s model couldn’t be read from the game’s files, so ${assistantName()} stays.`; fillAssistant();
      return;
    }
    endSummon();
    // the old demon keeps its canvas until the new one arrives; the new one gets a fresh canvas, hidden till then
    const old = A.view, oldCanvas = $('#assistant-view');
    A.view = null;
    oldCanvas.removeAttribute('id');
    const fresh = oldCanvas.cloneNode(false);
    fresh.removeAttribute('style'); fresh.className = ''; fresh.id = 'assistant-view';
    oldCanvas.after(fresh);
    const box = { w: A.el.clientWidth, h: A.el.clientHeight };
    const gone = () => { old.dispose(); oldCanvas.remove(); };
    if (!mountAssistant(model, false)) { gone(); A.el.hidden = true; A.state = 'failed'; document.body.classList.remove('has-assistant'); fillAssistant(); return; }
    A.state = 'on'; fillAssistant();
    const still = matchMedia('(prefers-reduced-motion: reduce)').matches;
    if (!fx || still) { gone(); placeAssistant(); return; }
    // the effect: a canvas over the box, wider and taller (the light comes from above), its ground at the box's feet
    fresh.style.visibility = 'hidden';
    oldCanvas.classList.add('summon-out');
    const fxCanvas = h('canvas', { class: 'summon-fx', 'aria-hidden': 'true' });
    const W = Math.round(box.w * 2.4), H = Math.round(box.h * 2.1), below = 24;
    Object.assign(fxCanvas.style, { left: Math.round((box.w - W) / 2) + 'px', top: (box.h + below - H) + 'px', width: W + 'px', height: H + 'px' });
    A.el.append(fxCanvas);
    let arrived = false;
    const arrive = () => {
      if (arrived) return;
      arrived = true;
      gone(); placeAssistant();
      fresh.style.visibility = ''; fresh.classList.add('summon-in');
      setTimeout(() => fresh.classList.remove('summon-in'), 600);
    };
    const s = { fx: fxCanvas, arrive };
    A.summoning = s;
    const rgb = model.summon && Array.isArray(model.summon.color) ? model.summon.color : null;
    await window.NMAssistant.summon(fxCanvas, fx, { box, ground: H - below - 6, color: rgb, circle: model.summon ? model.summon.circle : 1.5, arrive });
    if (A.summoning === s) { A.summoning = null; fxCanvas.remove(); }
  }
  const assistantReact = (what) => { if (A.view) A.view.react(what); };

  $('#set-assistant').addEventListener('change', (ev) => {
    store.set('assistant', ev.target.checked ? 'on' : 'off');
    if (ev.target.checked) startAssistant(); else stopAssistant();
  });

  async function takeSnapshot() {
    if (S.busy) return;
    S.busy = true; renderChrome();
    assistantReact('work');
    try {
      const r = await host.call('snapshot');
      S.demons = null;   // a snapshot can change the saved demon: ask the host for the list again next time
      await reload();
      assistantReact('ok');
      if (r.missing) toast('warn', 'Snapshot saved, with gaps', `No saved bonuses for ${r.missing}. Open the Information window in game and take another snapshot.`, 0);
      else if (r.source === 'window' || r.source === 'status') toast('ok', 'Snapshot saved', 'Stats, gear and expertise were all read live.' + (r.picture ? ' The picture was updated from the Information window.' : ''));
      else if (/\(digitalized/.test(r.statsNote || '')) toast('warn', 'Snapshot saved', 'Gear and expertise are live. The 12 stats and HP/MP are the saved ones, because you were digitalized (the game then counts the demon in them). Snapshot again when not digitalized to refresh them.', 12000);
      else toast('warn', 'Snapshot saved', 'Gear and expertise are live. The 12 stats and HP/MP couldn’t be read from the game this time, so the saved ones were kept. Take another snapshot to refresh them.', 12000);
    } catch (e) {
      assistantReact('fail');
      const [title, text] = explain(String(e && e.message || e));
      toast('err', title, text, 0);
      if (/No running client has|No character is selected/i.test(String(e && e.message))) scanClients();   // who IS in game?
    } finally {
      S.busy = false; renderChrome();
    }
  }
  $('#snap').addEventListener('click', takeSnapshot);
  $('#snap-help').addEventListener('click', openSnapSteps);
  $('#snap-steps-close').addEventListener('click', () => $('#snap-steps').close());
  $('#snap-steps-go').addEventListener('click', () => { $('#snap-steps').close(); takeSnapshot(); });
  $('#snap-steps').addEventListener('click', (ev) => { if (ev.target === ev.currentTarget) ev.currentTarget.close(); });

  // settings
  const dlg = $('#settings');
  function fillSettings() {
    $('#set-character').textContent = S.settings.character || 'None yet';
    $('#set-csv').textContent = S.settings.csv || '—';
    $('#set-data').textContent = S.settings.dataDir || '—';
    fillAssistant();
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
  $('#open-settings').addEventListener('click', () => { S.alignNote = ''; fillSettings(); fillAbout(); fillUpdates(); dlg.showModal(); });
  dlg.addEventListener('click', (ev) => { if (ev.target === dlg) dlg.close(); });

  // ---------- Send feedback ----------
  // The form posts to the feedback service (feedback-worker\, a Cloudflare Worker), which opens a public GitHub issue as
  // the "BABEL feedback" app: players need no GitHub account, and the app holds no GitHub key. The browser preview sends
  // test: true, so the service checks everything and answers with the issue it would open, but opens none.
  const FEEDBACK_URL = 'https://babel-feedback.babel-feedback.workers.dev/feedback';
  const fbDlg = $('#feedback');
  const fb = { kind: '', busy: false, sections: [] };   // sections: { page, text, el } in the order they were added
  const fbAbout = () => (S.about ? { version: S.about.version, commit: S.about.commit, built: S.about.built, windows: S.about.windows, webview2: S.about.webview2 } : null);
  // Sections: feedback on one page or tool each, picked from the sidebar's own list (PAGES and TOOLS)
  const FB_ITEMS = PAGES.concat(TOOLS);
  const fbName = (id) => (FB_ITEMS.find((p) => p[0] === id) || [])[1] || '';
  // The picker is the app's own dropdown (a popover, as What changed's snapshot picker), not a native <select>: each
  // page or tool with its sidebar icon, under the sidebar's own headings.
  let fbMenus = 0;
  function fbSectionEl(s) {
    const id = 'fb-menu-' + (++fbMenus);
    const ta = h('textarea', { class: 'input fb-message fb-section-text', maxlength: '4000', rows: '4', placeholder: 'What about this page or tool?' });
    ta.addEventListener('input', () => { s.text = ta.value; });
    s.label = h('span', { class: 'fb-pick-label' });
    s.btn = h('button', { type: 'button', class: 'select snap-btn fb-pick', popovertarget: id, 'aria-haspopup': 'listbox' }, s.label, icon('chevron'));
    const opt = ([pid, name, ic]) => h('button', { type: 'button', class: 'char fb-opt', 'data-page': pid,
      onclick: () => { s.menu.hidePopover(); s.page = pid; $('#fb-error').textContent = ''; fbDrawSections(); ta.focus(); } },
      icon(ic), h('span', { class: 'char-text' }, h('span', { class: 'char-name', text: name })), h('span', { class: 'char-line fb-opt-note' }));
    s.menu = h('div', { class: 'menu fb-menu', id, popover: 'auto', 'aria-label': 'Page or tool',
      ontoggle: (ev) => { if (ev.newState === 'open') fbOpenMenu(s); } },
      h('div', { class: 'char-list' }, h('div', { class: 'list-head', text: 'Pages' }), ...PAGES.map(opt), h('div', { class: 'list-head', text: 'Tools (beta)' }), ...TOOLS.map(opt)));
    const rm = h('button', { type: 'button', class: 'btn btn-ghost btn-square', 'aria-label': 'Remove this page or tool', 'data-tip': 'Remove',
      onclick: () => { fb.sections = fb.sections.filter((x) => x !== s); fbDrawSections(); $('#fb-add').focus(); } }, icon('trash'));
    return h('div', { class: 'fb-section' }, h('div', { class: 'fb-section-head' }, s.btn, rm), ta, s.menu);
  }
  /** A section's dropdown, just opened: under its button (above it when there is no room below), as wide as the button,
   *  the picked one (else the first free one) focused; arrow keys move through the ones that can be picked. */
  function fbOpenMenu(s) {
    const menu = s.menu, r = s.btn.getBoundingClientRect();
    menu.style.width = Math.max(240, r.width) + 'px';
    const w = menu.offsetWidth, hgt = menu.offsetHeight;
    menu.style.left = Math.max(8, Math.min(r.left, window.innerWidth - w - 8)) + 'px';
    menu.style.top = (r.bottom + 6 + hgt > window.innerHeight - 8 ? Math.max(8, r.top - 6 - hgt) : r.bottom + 6) + 'px';
    const list = menu.querySelector('.char-list');
    const cur = menu.querySelector('.fb-opt[aria-current="true"]') || menu.querySelector('.fb-opt:not([disabled])');
    if (cur) { list.scrollTop = cur.offsetTop - list.offsetTop - (list.clientHeight - cur.offsetHeight) / 2; cur.focus({ preventScroll: true }); }
    if (!menu.dataset.keys) {
      menu.dataset.keys = '1';
      menu.addEventListener('keydown', (ev) => {
        if (ev.key !== 'ArrowDown' && ev.key !== 'ArrowUp') return;
        const rows = [...menu.querySelectorAll('.fb-opt:not([disabled])')], i = rows.indexOf(document.activeElement);
        const next = rows[ev.key === 'ArrowDown' ? Math.min(rows.length - 1, i + 1) : Math.max(0, i - 1)];
        if (next) { next.focus(); ev.preventDefault(); }
      });
    }
    fbDlg.addEventListener('scroll', () => { if (menu.matches(':popover-open')) menu.hidePopover(); }, { once: true });   // its button moves away
  }
  function fbDrawSections() {
    for (const s of fb.sections) if (!s.el) s.el = fbSectionEl(s);
    $('#fb-sections').replaceChildren(...fb.sections.map((s) => s.el));
    // a page or tool is picked once: the other sections' lists grey it out
    const used = new Set(fb.sections.map((s) => s.page).filter(Boolean));
    for (const s of fb.sections) {
      const item = FB_ITEMS.find((p) => p[0] === s.page);
      s.label.replaceChildren(...(item ? [icon(item[2]), h('span', { text: item[1] })] : [h('span', { class: 'fb-pick-empty', text: 'Pick a page or tool…' })]));
      s.btn.setAttribute('aria-label', item ? `Page or tool: ${item[1]}` : 'Pick a page or tool');
      s.menu.querySelectorAll('.fb-opt').forEach((o) => {
        const taken = used.has(o.dataset.page) && o.dataset.page !== s.page;
        o.disabled = taken;
        o.setAttribute('aria-current', String(o.dataset.page === s.page));
        o.querySelector('.fb-opt-note').textContent = taken ? 'Added' : '';
      });
    }
    $('#fb-add').disabled = fb.sections.length >= FB_ITEMS.length;
    $('#fb-add-help').hidden = fb.sections.length > 0;
  }
  // Attachments: the support bundle's files (the app takes personal details out), as the person picks them, encrypted
  // here before they leave the PC: a fresh AES-256-GCM key locks the gzipped files, and the developer's public key
  // (RSA-OAEP, SHA-256) locks that key. Only the private key in the developer's .secrets\ opens them
  // (feedback-worker\open-files.ps1); the service and GitHub only ever see the locked envelope.
  const FEEDBACK_FILES_KEY = 'MIIBojANBgkqhkiG9w0BAQEFAAOCAY8AMIIBigKCAYEAnnSP06wVggTaCZo6ppr5m+5f3QRqQHqmxbOI/WoeNDkgbfxwrjcc3IHJ2HSzljCJS0UuJuV3tN3s2KEdBBerduQlXmvjZxG/qyHTmbQx7RsIC5eR6cZR+B/GSAi3L2z6OcgxMlrV1Y45AFazdsjLgZTgb5XQ1CUQkApNRit1hdY6/YOXRT1MEa1hClo0koGJeHI038PHV7fybWrVWgn2XirIXwVcHRPNrEukPwk0VqQQzFDIn0qu4XaBqiXMr9fH5dXcuiSft/E/6yNjqExf3TISC2wcYODA2vqvgl05J1Fuxz2XEAxJEp9xDaNvyXKotYzemgeyKVU4WvHUSBT9j1acN372UlYXmCc93QQoexkNmUCArfAAjafe3pdicjZjq2sCmkArOe8j32Hal//P2KTW4bfAkXpLP6Y690MN/oMJsc1itbGm9ABbsax4AoMP65A3OHsyznfC6ETBCneeQBBPqPVdRzY5iGGzGUJXxksFVzQ3Pom3ccWB0hSYrG0xAgMBAAE=';
  const FB_FILES_MAX = 14 * 1024 * 1024;   // the service's limit for the encrypted envelope (base64 characters)
  const b64 = (buf) => { const u = new Uint8Array(buf instanceof ArrayBuffer ? buf : buf.buffer); let s = ''; for (let i = 0; i < u.length; i += 0x8000) s += String.fromCharCode.apply(null, u.subarray(i, i + 0x8000)); return btoa(s); };
  /** Lock `payload` (JSON) for the developer: { v, alg, key, iv, data } (feedback-worker\src\attachment.js says what each is). */
  async function fbSeal(payload) {
    const zipped = await new Response(new Blob([JSON.stringify(payload)]).stream().pipeThrough(new CompressionStream('gzip'))).arrayBuffer();
    const pub = await crypto.subtle.importKey('spki', Uint8Array.from(atob(FEEDBACK_FILES_KEY), (c) => c.charCodeAt(0)), { name: 'RSA-OAEP', hash: 'SHA-256' }, false, ['encrypt']);
    const aes = await crypto.subtle.generateKey({ name: 'AES-GCM', length: 256 }, true, ['encrypt']);
    const iv = crypto.getRandomValues(new Uint8Array(12));
    const data = await crypto.subtle.encrypt({ name: 'AES-GCM', iv }, aes, zipped);
    const key = await crypto.subtle.encrypt({ name: 'RSA-OAEP' }, pub, await crypto.subtle.exportKey('raw', aes));
    return { v: 1, alg: 'RSA-OAEP-256+A256GCM', key: b64(key), iv: b64(iv), data: b64(data) };
  }
  // All of the bundle's files go, character names kept (they're encrypted). Gathering starts when the box is ticked (a
  // self-check runs first, a few seconds), so they're usually ready by the time Send is pressed.
  fb.files = null;   // { includeNames, files: [{ name, text, bytes }] } once gathered
  fb.filesLoad = null;
  fb.filesError = '';
  function fbDrawFiles() {
    const on = $('#fb-files').checked;
    $('#fb-public-more').textContent = !on ? 'Nothing else goes with it: no snapshots, logs or files.'
      : 'The attached files are not public: the issue only says that some are attached, and only BABEL’s developer can open them.';
    $('#fb-files-error').textContent = on ? fb.filesError : '';
  }
  function fbLoadFiles() {
    fb.files = null; fb.filesError = '';
    fbDrawFiles();
    const load = fb.filesLoad = host.call('feedbackFiles', { includeNames: true }).then((r) => {
      if (load !== fb.filesLoad) return;
      fb.files = { includeNames: !!r.includeNames, files: r.files || [] };
    }).catch((e) => { if (load === fb.filesLoad) { fb.filesError = 'Couldn’t gather the files: ' + String(e && e.message || e); fbDrawFiles(); } });
    return load;
  }
  $('#fb-files').addEventListener('change', () => { if ($('#fb-files').checked && (!fb.filesLoad || fb.filesError)) fbLoadFiles(); else fbDrawFiles(); });

  $('#fb-add').addEventListener('click', () => {
    // starts on the page you were looking at, unless it already has a section
    const s = { page: fb.sections.some((x) => x.page === S.page) ? '' : S.page, text: '' };
    fb.sections.push(s);
    fbDrawSections();
    if (s.page) s.el.querySelector('textarea').focus();
    else { s.btn.focus(); s.menu.showPopover(); }   // nothing to start on: the list, open
  });
  function fbDraw() {
    document.querySelectorAll('#fb-kind [data-kind]').forEach((b) => b.setAttribute('aria-pressed', String(b.dataset.kind === fb.kind)));
    const n = $('#fb-message').value.trim().length;
    $('#fb-count').textContent = n ? `${n} / 4000` : '';
    $('#fb-send').disabled = fb.busy;
    $('#fb-send').textContent = fb.busy ? 'Sending…' : 'Send';
  }
  function fbOpen() {
    $('#fb-fields').hidden = false; $('#fb-done').hidden = true; $('#fb-error').textContent = '';
    fbDraw();
    fbDlg.showModal();
    if (!S.about) fillAbout();
    // "Attach diagnostics files" is ticked to start with: gather them now, so they're ready by the time Send is pressed
    if ($('#fb-files').checked && (!fb.filesLoad || fb.filesError)) fbLoadFiles();
    fbDrawFiles();
    (fb.kind ? $('#fb-message') : fbDlg.querySelector('#fb-kind button')).focus();
  }
  function fbClear() {
    fb.kind = ''; ['#fb-title', '#fb-message', '#fb-contact', '#fb-website'].forEach((s) => { $(s).value = ''; });
    fb.sections = []; fbDrawSections();
    $('#fb-files').checked = true; fb.files = null; fb.filesLoad = null; fb.filesError = ''; fbDrawFiles();   // gathered again next time it opens
  }
  $('#open-feedback').addEventListener('click', fbOpen);
  $('#fb-close').addEventListener('click', () => fbDlg.close());   // what was written stays until it is sent
  $('#fb-cancel').addEventListener('click', () => fbDlg.close());
  fbDlg.addEventListener('click', (ev) => { if (ev.target === fbDlg) fbDlg.close(); });
  document.querySelectorAll('#fb-kind [data-kind]').forEach((b) => b.addEventListener('click', () => { fb.kind = b.dataset.kind; $('#fb-error').textContent = ''; fbDraw(); }));
  $('#fb-message').addEventListener('input', fbDraw);
  $('#fb-form').addEventListener('submit', async (ev) => {
    ev.preventDefault();
    if (fb.busy) return;
    const err = $('#fb-error'), message = $('#fb-message').value.trim();
    if (!fb.kind) { err.textContent = 'Pick what it is: something’s wrong, an idea or a question.'; return; }
    const unpicked = fb.sections.find((s) => !s.page);
    if (unpicked) { err.textContent = 'Pick a page or tool for each section, or remove the empty one.'; unpicked.btn.focus(); return; }
    const blank = fb.sections.find((s) => !s.text.trim());
    if (blank) { err.textContent = `Write something about ${fbName(blank.page)}, or remove it.`; blank.el.querySelector('textarea').focus(); return; }
    const sections = fb.sections.map((s) => ({ page: fbName(s.page), text: s.text.trim() }));
    if (message.length + sections.reduce((n, s) => n + s.text.length, 0) < 10) {
      err.textContent = 'Write a little more: at least 10 characters.'; (fb.sections.length ? fb.sections[0].el.querySelector('textarea') : $('#fb-message')).focus(); return;
    }
    const attach = $('#fb-files').checked;
    if (attach && fb.filesError) { err.textContent = 'The files couldn’t be gathered: untick “Attach diagnostics files”, or try again.'; return; }
    err.textContent = '';
    fb.busy = true; fbDraw();
    if (!S.about) await fillAbout();   // sent at once after opening: the version wasn't read yet
    const sent = { kind: fb.kind, title: $('#fb-title').value.trim(), message, sections, contact: $('#fb-contact').value.trim(),
      about: fbAbout(), website: $('#fb-website').value, test: !host.native };
    try {
      if (attach) {
        $('#fb-send').textContent = 'Gathering files…';
        if (!fb.filesLoad) fbLoadFiles();
        if (!fb.files) await fb.filesLoad;   // still gathering: wait for it
        if (!fb.files) throw new Error(fb.filesError || 'Couldn’t gather the files.');
        const files = fb.files.files;
        if (files.length) {
          $('#fb-send').textContent = 'Encrypting…';
          sent.attachment = await fbSeal({ files: files.map(({ name, text }) => ({ name, text })), includeNames: fb.files.includeNames, made: new Date().toISOString() });
          if (sent.attachment.data.length > FB_FILES_MAX) throw new Error('The diagnostics files are too big to send (10 MB at most once compressed). Untick “Attach diagnostics files” to send your message without them.');
          sent.attachmentInfo = { files: files.length, bytes: files.reduce((n, f) => n + f.bytes, 0) };
        }
        $('#fb-send').textContent = 'Sending…';
      }
      const ctl = new AbortController(), timer = setTimeout(() => ctl.abort(), sent.attachment ? 60000 : 20000);
      let r;
      try { r = await fetch(FEEDBACK_URL, { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify(sent), signal: ctl.signal }); }
      finally { clearTimeout(timer); }
      const j = await r.json().catch(() => null);
      if (!r.ok || !j || !j.ok) throw new Error((j && j.error) || `The feedback service answered ${r.status}.`);
      fbShowDone(j);
      fbClear();
    } catch (e) {
      const offline = e && (e.name === 'AbortError' || e instanceof TypeError);
      err.textContent = offline ? 'Couldn’t reach the feedback service. Check your internet connection and try again; your message is still here.'
        : String(e && e.message || e);
    } finally { fb.busy = false; fbDraw(); }
  });
  function fbShowDone(j) {
    const done = $('#fb-done');
    if (j.test) {
      // the browser preview: what would have been opened (an attachment is kept for an hour, to try opening it)
      done.replaceChildren(
        h('div', { class: 'notice' }, icon('info'), h('div', null, h('b', { text: 'Browser preview: no issue was opened. ' }),
          'The service checked it and would open this issue:' + (j.attachment ? ` The encrypted files are kept for an hour as ${j.attachment}.` : ''))),
        h('div', { class: 'fb-issue' }, h('div', { class: 'fb-issue-title', text: j.issue.title }), h('div', { class: 'field-help', text: 'Labels: ' + j.issue.labels.join(', ') }), h('pre', { text: j.issue.body })),
        h('div', { class: 'field-row' }, h('button', { type: 'button', class: 'btn btn-primary', text: 'Close', onclick: () => fbDlg.close() })));
    } else {
      const who = S.state.active || S.settings.character || '';
      const kids = [h('div', { class: 'notice' }, icon('check'), h('b', { text: `Thanks for the feedback${who ? ' ' + who : ''}!` }))];
      const row = h('div', { class: 'field-row' });
      if (j.number) row.append(h('button', { type: 'button', class: 'btn', text: 'Open on GitHub', onclick: () => host.call('openIssue', { number: j.number }).catch(() => {}) }));
      row.append(h('button', { type: 'button', class: 'btn btn-primary', text: 'Close', onclick: () => fbDlg.close() }));
      done.replaceChildren(...kids, row);
    }
    $('#fb-fields').hidden = true; done.hidden = false;
  }
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

  // About this version, and When something goes wrong (the self-check, the support bundle, the log)
  async function copyText(text) {
    try { await navigator.clipboard.writeText(text); return true; }
    catch (e) {
      try {   // the older way, from a box inside the dialog (a modal dialog makes everything outside it unreachable)
        const box = h('textarea', { style: 'position:fixed;left:-9999px;top:0', 'aria-hidden': 'true' });
        box.value = text; dlg.append(box); box.select();
        const ok = document.execCommand('copy'); box.remove(); return ok;
      } catch (e2) { return false; }
    }
  }
  /** Flash a button's label ("Copied") and put it back. */
  function flashLabel(btn, text) {
    const was = btn.dataset.label || btn.textContent; btn.dataset.label = was; btn.textContent = text;
    clearTimeout(btn._flash); btn._flash = setTimeout(() => { btn.textContent = was; }, 1600);
  }
  async function fillAbout() {
    const dl = $('#about-block');
    try {
      const a = await host.call('about');
      if (!a || typeof a !== 'object') throw new Error('no answer');
      S.about = a;
      const row = (k, v) => [h('dt', { text: k }), h('dd', { text: v })];
      dl.replaceChildren(...row('Version', a.version), ...row('Commit', a.commit), ...row('Built', a.built), ...row('Windows', a.windows), ...row('WebView2', a.webview2));
    } catch (e) { S.about = null; dl.replaceChildren(h('dt', { text: 'Version' }), h('dd', { text: 'Not available' })); }
    $('#about-copy').disabled = !S.about;
  }
  $('#about-copy').addEventListener('click', async (ev) => {
    if (!S.about) return;
    flashLabel(ev.currentTarget, (await copyText(S.about.text)) ? 'Copied' : 'Couldn’t copy');
  });

  function drawDoctor(r) {
    const out = $('#doctor-out');
    const word = { pass: 'Pass: ', warn: 'Warning: ', fail: 'Fail: ' };
    const kids = [h('div', { class: 'checks-sum' }, h('span', { text: r.summary.text }),
      h('button', { type: 'button', class: 'btn btn-sm', text: 'Copy results', onclick: async (ev) => { flashLabel(ev.currentTarget, (await copyText(r.text)) ? 'Copied' : 'Couldn’t copy'); } }))];
    let section = null;
    for (const c of r.checks) {
      if (c.section !== section) { section = c.section; kids.push(h('h3', { text: section })); }
      kids.push(h('div', { class: 'check check-' + c.status },
        icon(c.status === 'pass' ? 'check' : c.status === 'warn' ? 'alert' : 'x'),
        h('div', null,
          h('div', null, h('span', { class: 'sr-only', text: word[c.status] || '' }), h('span', { class: 'check-name', text: c.name })),   // (how long it took is in the detail)
          h('div', { class: 'check-detail', text: c.detail }),
          c.advice ? h('div', { class: 'check-advice' }, h('b', { text: 'What to do: ' }), c.advice) : null)));
    }
    out.replaceChildren(...kids);
    out.scrollIntoView({ block: 'start' });   // the dialog scrolls: show the results from their first line
  }
  let doctorBusy = false;
  $('#doctor-run').addEventListener('click', async (ev) => {
    if (doctorBusy) return;
    doctorBusy = true;
    const btn = ev.currentTarget, out = $('#doctor-out');
    btn.disabled = true; btn.textContent = 'Checking…';
    out.hidden = false;
    out.replaceChildren(h('span', { class: 'field-help', text: 'Looking at your files, the game and each reader. This takes a few seconds.' }));
    out.scrollIntoView({ block: 'nearest' });
    try { drawDoctor(await host.call('doctor')); }
    catch (e) {
      out.replaceChildren(h('div', { class: 'notice notice-warn' }, icon('alert'), h('div', null, h('b', { text: 'The check couldn’t run. ' }), String(e && e.message || e))));
    } finally { doctorBusy = false; btn.disabled = false; btn.textContent = 'Check my setup'; }
  });

  let bundleBusy = false;
  $('#bundle-save').addEventListener('click', async (ev) => {
    if (bundleBusy) return;
    bundleBusy = true;
    const btn = ev.currentTarget, result = $('#bundle-result');
    btn.disabled = true; btn.textContent = 'Saving…';
    $('#bundle-done').hidden = true;
    try {
      const r = await host.call('saveBundle', { includeNames: $('#bundle-names').checked });
      if (r && typeof r === 'object' && r.path) {   // (null: the save window was cancelled)
        result.hidden = false;
        result.textContent = `Saved ${r.path} (${Math.max(1, Math.round(r.zipBytes / 1000))} KB, ${r.files.length} files). Character names are ${r.includeNames ? 'included' : 'left out'}.`;
        $('#bundle-done').hidden = false;
        result.scrollIntoView({ block: 'nearest' });
      }
    } catch (e) {
      result.hidden = true;
      toast('err', 'The bundle couldn’t be saved', String(e && e.message || e), 0);
    } finally { bundleBusy = false; btn.disabled = false; btn.textContent = 'Save a support bundle…'; }
  });
  $('#bundle-show').addEventListener('click', () => host.call('reveal', { what: 'bundle' }).catch(() => {}));
  $('#logs-show').addEventListener('click', () => host.call('reveal', { what: 'logs' }).catch(() => {}));

  // ---------- updates (the app's update.rs) ----------
  // A look at BABEL's releases at start (unless turned off) and every hour while it is open; a new version shows a notice once per
  // run. Nothing is installed until the player clicks Update: the app then downloads it, checks its signature, swaps the
  // exe and starts again. Settings > Updates shows where things stand and the release notes.
  const U = { last: null, busy: false, failed: '', noticed: new Set() };
  /** The release notes (GitHub Markdown, as the release page has them) as plain elements: headings, bullets, lines. */
  function releaseNotes(md) {
    const kids = [];
    let list = null;
    const clean = (s) => s.replace(/\*\*|__|`/g, '').replace(/\[([^\]]+)\]\([^)]*\)/g, '$1').trim();
    for (const raw of String(md || '').split('\n')) {
      const line = raw.trim();
      if (!line) { list = null; continue; }
      const head = /^#{1,6}\s+(.*)$/.exec(line), item = /^[-*+]\s+(.*)$/.exec(line);
      if (head) { list = null; kids.push(h('div', { class: 'upd-notes-head', text: clean(head[1]) })); }
      else if (item) { if (!list) { list = h('ul'); kids.push(list); } list.append(h('li', { text: clean(item[1]) })); }
      else { list = null; kids.push(h('p', { text: clean(line) })); }
    }
    return kids;
  }
  const mb = (n) => (n / 1048576).toFixed(1);
  function fillUpdates() {
    const r = U.last, up = r && r.update, st = $('#upd-status');
    $('#upd-auto').checked = S.settings.updateCheck !== false;
    $('#upd-pre').checked = !!S.settings.updatePre;
    const off = !!(r && r.enabled === false);
    $('#upd-check').disabled = off || U.busy;
    $('#upd-install').hidden = !(up && up.installable);
    $('#upd-install').disabled = U.busy;
    const notes = $('#upd-notes');
    notes.hidden = !(up && up.notes);
    if (up && up.notes) notes.replaceChildren(...releaseNotes(up.notes));
    if (U.busy) return;   // the install writes its own progress
    $('#upd-progress').hidden = true;
    st.textContent = U.failed ? `The update didn’t go through: ${U.failed}`
      : !r ? 'Not checked yet.'
      : off ? r.reason
      : r.error ? `${r.error}${r.checkedAt ? ` (${r.checkedAt})` : ''}`
      : up ? `BABEL ${up.version} is out${up.pre ? ' (a test version)' : ''}. You have ${r.current}.${up.installable ? '' : ' This one can’t be installed from inside BABEL: download it from its release page.'}`
      : `BABEL ${r.current} is the newest version. Checked ${r.checkedAt}.`;
  }
  async function checkUpdates(opts) {
    try {
      const r = await host.call('updateCheck', opts || {});
      if (r && r.off) return null;   // the look at start is turned off
      U.last = r;
    } catch (e) { U.last = { enabled: true, error: String(e && e.message || e) }; }
    fillUpdates();
    return U.last;
  }
  function updateNotice(up) {
    if (U.noticed.has(up.version)) return;
    U.noticed.add(up.version);
    const el = h('div', { class: 'toast toast-info', role: 'status' }, icon('info'),
      h('div', null,
        h('div', { class: 'toast-title', text: `BABEL ${up.version} is out` }),
        h('div', { class: 'toast-text', text: up.installable ? 'BABEL closes for a moment and opens again with the new version. Your data stays as it is.' : 'This one has to be downloaded from its release page.' }),
        h('div', { class: 'toast-acts' },
          up.installable ? h('button', { type: 'button', class: 'btn btn-primary btn-sm', text: 'Update and restart', onclick: () => { el.remove(); installUpdate(); } }) : null,
          h('button', { type: 'button', class: 'btn btn-sm', text: 'What’s new', onclick: () => { el.remove(); openSettingsAt('#updates'); } }))),
      h('button', { type: 'button', class: 'btn btn-ghost btn-square', 'aria-label': 'Later', 'data-tip': 'Later', onclick: () => el.remove() }, icon('x')));
    homeToasts();
    toastsEl.append(el);
  }
  async function autoCheckUpdates() {
    if (U.busy) return;
    const r = await checkUpdates({ auto: true });
    if (r && r.update) updateNotice(r.update);
  }
  async function installUpdate() {
    const up = U.last && U.last.update;
    if (!up || !up.installable || U.busy) return;
    U.busy = true; U.failed = '';
    openSettingsAt('#updates');
    fillUpdates();
    const st = $('#upd-status'), bar = $('#upd-progress');
    st.textContent = `Downloading BABEL ${up.version}…`;
    bar.hidden = false; bar.removeAttribute('value');
    const poll = setInterval(async () => {
      const s = await host.call('updateStatus').catch(() => null);
      if (!s || !U.busy) return;
      if (s.phase === 'downloading' && s.total) { bar.max = s.total; bar.value = s.done; st.textContent = `Downloading BABEL ${up.version}… ${mb(s.done)} of ${mb(s.total)} MB`; }
      else if (s.phase === 'checking') { bar.removeAttribute('value'); st.textContent = 'Checking the download’s signature…'; }
      else if (s.phase === 'restarting') st.textContent = 'Restarting…';
    }, 300);
    try {
      await host.call('updateInstall', { version: up.version });
      st.textContent = 'Restarting…';   // the window closes and the new version opens
    } catch (e) {
      clearInterval(poll);
      U.busy = false; U.failed = String(e && e.message || e);
      fillUpdates();
    }
  }
  function openSettingsAt(sel) {
    if (!dlg.open) { S.alignNote = ''; fillSettings(); fillAbout(); fillUpdates(); dlg.showModal(); }
    const el = $(sel);
    if (el) requestAnimationFrame(() => el.scrollIntoView({ block: 'start' }));
  }
  $('#upd-check').addEventListener('click', async (ev) => {
    const btn = ev.currentTarget;
    btn.disabled = true; btn.textContent = 'Checking…'; U.failed = '';
    await checkUpdates({ force: true });
    btn.textContent = 'Check now'; fillUpdates();
  });
  $('#upd-install').addEventListener('click', installUpdate);
  $('#upd-page').addEventListener('click', () => host.call('updatePage').catch(() => {}));
  $('#upd-auto').addEventListener('change', (ev) => {
    S.settings.updateCheck = ev.currentTarget.checked;
    host.call('saveSettings', { updateCheck: S.settings.updateCheck }).catch(() => {});
  });
  $('#upd-pre').addEventListener('change', async (ev) => {
    S.settings.updatePre = ev.currentTarget.checked;
    await host.call('saveSettings', { updatePre: S.settings.updatePre }).catch(() => {});
    if (U.last && U.last.enabled !== false) checkUpdates({});   // what is on offer changes with it
  });
  // Just updated (the new exe was started with --updated-from): say so once. It stays until the player dismisses it.
  function sayUpdated() {
    const from = Q.get('updated');
    if (!from) return;
    host.call('about').then((a) => {
      const now = a && a.version ? a.version : '';
      const el = h('div', { class: 'toast toast-ok', role: 'status' }, icon('check'),
        h('div', null,
          h('div', { class: 'toast-title', text: now ? `BABEL is now ${now}` : 'BABEL was updated' }),
          h('div', { class: 'toast-text', text: `It was ${from}.` }),
          h('div', { class: 'toast-acts' },
            h('button', { type: 'button', class: 'btn btn-sm', text: 'What’s new', onclick: () => host.call('updatePage', { current: true }).catch(() => {}) }),
            h('button', { type: 'button', class: 'btn btn-primary btn-sm', text: 'Dismiss', onclick: () => el.remove() }))));
      homeToasts();
      toastsEl.append(el);
    }).catch(() => {});
  }

  // The game was updated: say so once per game build, gently (the reader's memory layouts are for one build)
  const warnedBuilds = new Set();
  async function checkBuild() {
    try {
      const r = await host.call('buildCheck');
      const fresh = ((r && r.unknown) || []).filter((c) => !warnedBuilds.has(c.fingerprint));
      if (!fresh.length) return;
      fresh.forEach((c) => warnedBuilds.add(c.fingerprint));
      toast('warn', 'The game was updated', 'Readings may be wrong until the reader is updated. Settings > Check my setup has the details.', 15000);
    } catch (e) { /* an older host has no such check */ }
  }

  // keyboard
  document.addEventListener('keydown', (ev) => {
    if (document.querySelector('dialog[open], [popover]:popover-open')) return;   // any dialog (settings, characters, picture) or the demon dropdown
    if (ev.key === 'F5' && !WEB) { ev.preventDefault(); takeSnapshot(); return; }
    if (ev.ctrlKey && !ev.shiftKey && !ev.altKey && ev.key >= '1' && ev.key <= String(PAGES.length)) { ev.preventDefault(); go(PAGES[Number(ev.key) - 1][0]); return; }
    if (ev.altKey && !ev.ctrlKey && !ev.shiftKey && (ev.key === 'ArrowLeft' || ev.key === 'ArrowRight')) { ev.preventDefault(); step(ev.key === 'ArrowLeft' ? -1 : 1); return; }
    if (ev.key === 'BrowserBack' || ev.key === 'BrowserForward') { ev.preventDefault(); step(ev.key === 'BrowserBack' ? -1 : 1); return; }
    if ((ev.ctrlKey && ev.key.toLowerCase() === 'f') || (ev.key === '/' && !/^(INPUT|SELECT|TEXTAREA)$/.test(document.activeElement.tagName))) {
      const box = $('[data-search]');
      if (box) { ev.preventDefault(); box.focus(); box.select(); }
    }
  });

  // game status
  // Cheap check every few seconds; a full look at the clients only when one opened or closed, or (every half
  // minute) while a client is still sitting at the title screen.
  // Clients whose character is known are not read again (keepKnown); nothing runs while the window is minimized
  // (the app hides the page then).
  async function pollStatus() {
    if (S.scanning || S.busy || document.hidden) return;
    try {
      const st = await host.call('status');
      if (st.gameRunning && A.state === 'missing' && assistantWanted()) startAssistant();   // the game is up now: its files can be found
      const waiting = S.state.clients.some((c) => !c.character) && Date.now() - (S.lastScan || 0) > 30000;
      if (st.clientsChanged || waiting) { await scanClients({ keepKnown: true }); await settle(false); }
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
    window.__babel = { S, takeSnapshot, assistant: A, host };   // for checking the page from DevTools (and for scripted runs: tours)
    // The saved data is on screen; now see who is in game. One character: show it. Several: ask.
    if (host.native || Q.has('clients')) { await scanClients(); await settle(true); }
    if (assistantWanted()) {
      const going = startAssistant();
      // asked for by name (a picture is about to be taken): wait until it stands there
      if (Q.get('assistant') === '1') await Promise.race([going.then(() => new Promise((r) => setTimeout(r, 400))), new Promise((r) => setTimeout(r, 5000))]);
      if (PICKER) going.then(() => followSummoned(true));
    }
    if (PICKER) setInterval(() => followSummoned(false), 5000);   // summoned another demon in game: it comes here too
    requestAnimationFrame(() => requestAnimationFrame(() => host.call('ready').catch(() => {})));
    // updates: a few seconds after the page is up (it never slows the start), then every hour while it stays open
    sayUpdated();
    setTimeout(autoCheckUpdates, 4000);
    setInterval(autoCheckUpdates, 3600 * 1000);
    if (host.native) {
      setInterval(pollStatus, 5000);
      document.addEventListener('visibilitychange', () => { if (!document.hidden) pollStatus(); });   // back from minimized: look now
    }
  })();
})();
