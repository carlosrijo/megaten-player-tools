'use strict';
// BABEL: the page inside the desktop window.
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
    fuse: '<circle cx="8" cy="8" r="3.5"/><circle cx="16" cy="8" r="3.5"/><path d="M8 11.5v1.5a4 4 0 0 0 4 4 4 4 0 0 0 4-4v-1.5M12 17v3.5"/>',
    horns: '<path d="M6.5 3.5c-.6 3 .4 5.4 2.6 6.6M17.5 3.5c.6 3-.4 5.4-2.6 6.6"/><circle cx="12" cy="14.5" r="6"/><path d="M9.6 14h.01M14.4 14h.01M10 17.2h4"/>',
    flask: '<path d="M9 3.5h6M10 3.5v5.2L5.2 17.6A2 2 0 0 0 7 20.5h10a2 2 0 0 0 1.8-2.9L14 8.7V3.5"/><path d="M7.6 14.5h8.8"/>',
    podium: '<path d="M3.5 20h17M4.5 20v-5.5h5V20M9.5 20V9h5v11M14.5 20v-7.5h5V20"/><path d="M12 4.2v2"/>',
    plus: '<path d="M12 5v14M5 12h14"/>',
    gate: '<path d="M5 20.5V10a7 7 0 0 1 14 0v10.5"/><path d="M3 20.5h18"/><path d="M9.5 20.5v-5a2.5 2.5 0 0 1 5 0v5"/>',
    copy: '<rect x="8.5" y="8.5" width="11" height="11" rx="2"/><path d="M5.5 15.5h-1a1 1 0 0 1-1-1v-10a1 1 0 0 1 1-1h10a1 1 0 0 1 1 1v1"/>',
    trash: '<path d="M4.5 7h15M9.5 7V4.5h5V7M6.5 7l1 12.5h9l1-12.5M10 10.5v6M14 10.5v6"/>',
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
    const settings = { character: 'Rubito', csv: 'C:\\Users\\…\\smt-stats\\rubito-stats.csv', theme: store.get('theme') || 'system', dataDir: '…\\imagine-reader\\data', csvExists: true };
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
          const [snapshots, gear, expertise, bonuses, ranks, digitalize, icons, unlocks] = empty ? [[], [], [], [], null, null, {}, []] : await Promise.all([
            get('../../data/dash/snapshots.json', []), get('../../data/dash/gear.json', []), get('../../data/dash/expertise.json', []),
            get('../../data/dash/bonuses.json', []), get('../../data/expertise-ranks.json', null), get(Q.get('digi') || '../../data/dash/digitalize.json', null), get('../../data/dash/icons.json', {}), get('../../data/dash/digi-unlocks.json', [])]);
          // the summoned demon comes with the page data (dash\summoned.json); here the stand-in list's summoned one
          const summoned = empty ? null : (await this.call('compDemons', { cached: true })).find((d) => d.summoned) || null;
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
            ['Seraph', 66, 'Divine', '', 2, true, false], ['Pixie', 2, 'Fairy', '', 0, false, false]].map(([name, level, race, mitama, forceStack, equipment, inGame]) =>
            ({ key: name, name, level, race, mitama, forceStack, equipment, inGame: !(args && args.cached) && inGame, current: name === cur, summoned: name === 'Specter' && Q.get('summoned') !== '0', icon: null, loaded: name !== 'Pixie', inComp: name !== 'Seraph', seenAt: '2026-11-23T15:40:23',
              // stand-ins for the three states of the unlock tag
              unlock: name === 'Alilat' ? { listed: false, level: 0, raceLevel: 1, race, reached: false } : { listed: true, level: name === 'Seraph' ? 8 : 5, raceLevel: name === 'Seraph' ? 5 : 10, race, reached: name !== 'Seraph' },
              // the saved demon's own numbers as its offer, so its card looks as in the app (the others have none here)
              offer: name === cur && S.data && S.data.digitalize ? (({ rate, alignment, stats, skills }) => ({ rate, alignment, stats, skills, combat: {} }))(S.data.digitalize) : null }));
        }
        // The browser preview has no game and no files to change: say so instead of pretending (the desktop app switches)
        if (cmd === 'useDemon') { await wait(300); throw Object.assign(new Error('This is the browser preview: it shows stand-in demons and can’t change your data. Pick the demon in the desktop app (BABEL.exe).'), { kind: 'user' }); }
        if (cmd === 'setAlignment') {
          await wait(900);
          fake.align = fake.align || {}; fake.align[fake.active] = args.alignment;
          return { changed: true, rows: fake.active === 'Rubito' ? 5 : 0, changedRows: fake.active === 'Rubito' ? 3 : 0, backup: 'rubito-stats.backup-before-alignment-20261122-233212.csv' };
        }
        // The Simulator's numbers and saved builds are the reader's own: the browser preview has them when it is
        // served by "imagine-reader serve" (src\Build\PreviewServer.cs), which answers these like the app does.
        if (cmd.startsWith('sim')) {
          const r = await fetch('/host/' + cmd, { method: 'POST', body: JSON.stringify(args || {}) }).catch(() => null);
          if (!r || !r.ok) throw Object.assign(new Error('This browser preview has no reader behind it, so it can’t work out a build. Open it through “imagine-reader serve”, or use the desktop app.'), { kind: 'user' });
          const m = await r.json();
          if (!m.ok) throw Object.assign(new Error(m.error), { kind: m.kind });
          return m.result;
        }
        // the assistant's model: the file the reader made (imagine-reader.exe model); ?assistant=none stands in for "game never seen"
        if (cmd === 'assistant') return Q.get('assistant') === 'none' ? null : get('../../data/cache/models/D2_11_003a.json', null);
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
        if (cmd === 'saveBundle') { await wait(300); throw Object.assign(new Error('This is the browser preview: it can’t save a support bundle. Use the desktop app (BABEL.exe).'), { kind: 'user' }); }
        if (cmd === 'buildCheck') return Q.get('build') === 'old' ? { clients: 1, known: 0, unknown: [{ folder: 'client1', pid: 1, fingerprint: 'preview', same: false }] } : { clients: fake.clients.length, known: fake.clients.length, unknown: [] };
        if (cmd === 'log') return true;   // the page's error reports: nothing to write them to here
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
      ['ta_chance', 'Technical chance'], ['ta_power', 'Technical power'], ['pursuit_chance', 'Pursuit chance'], ['pursuit_power', 'Pursuit power']]]],
    [['Damage dealt', [['dmg_close', 'Close-range'], ['dmg_long', 'Long-range'], ['dmg_spell', 'Spell'], ['dmg_support', 'Support']]],
      ['Damage taken', [['taken_close', 'Close-range'], ['taken_long', 'Long-range'], ['taken_spell', 'Spell'], ['taken_support', 'Support']]],
      ['Speed', [['cooldown', 'Cooldown'], ['move_speed', 'Movement']]]],
  ];
  /** The Bonuses row behind a key stat (same name, same unit; the cap is called "Limit break cap" there). */
  const BONUS_NAME = { lb_cap: 'Limit break cap' };
  const bonusRowOf = (k, label, unit) => (S.data.bonuses || []).find((r) => r.group === 'Combat' && r.bonus === (BONUS_NAME[k] || label) && (r.unit || '') === (unit || ''));
  /** "Alilat (summoned)": the summoned demon's name as its bonus parts carry it (empty when it gives nothing). */
  const summonedName = () => { for (const r of (S.data.bonuses || [])) { const p = (r.parts || []).find((x) => x.kind === 'summoned'); if (p) return String(p.source); } return ''; };
  const TOP_CHANGES = 5;   // the Overview lists this many changes until "Show all"
  const PAGES = [['overview', 'Overview', 'grid'], ['bonuses', 'Bonuses', 'list'], ['gear', 'Gear', 'shield'], ['expertise', 'Expertise', 'star'], ['digitalize', 'Digitalize', 'bolt'], ['history', 'History', 'chart']];
  // Tools: pages about the game itself, not about the character (they work before the first snapshot too).
  const TOOLS = [['skills', 'Skill Database', 'bolt'], ['ddb', 'Demon Database', 'horns'], ['geardb', 'Gear Database', 'shield'], ['rank', 'Demon Rank', 'podium'], ['dcm', 'DCM Locator', 'search'], ['dungeons', 'Dungeon Explorer', 'gate'], ['quests', 'Quest Guide', 'book'], ['fusion', 'Fusion Planner', 'fuse'], ['sim', 'Simulator', 'flask']];
  const isTool = (page) => TOOLS.some((t) => t[0] === page);
  const BONUS_GROUPS = [
    ['Combat', 'Combat', 'You · combat'], ['Stats', 'Stats', 'You · stats'], ['Affinity', 'Affinity', 'You · affinity'], ['Other', 'Other', 'You · other'],
    ['Partner', 'Partner', 'Your partner'], ['Only in some fights', 'Situational', 'Only in some fights · not added to the totals'],
  ];
  const KIND = { item: 'Item', mod: 'Mod', tarot: 'Tarot', soul: 'Soul fusion', appearance: 'Appearance', set: 'Set bonus', 'fusion set': 'Fusion set', expertise: 'Expertise', clan: 'Clan bonuses', dlevel: 'Digitalize level', quest: 'Quest bonus', compendium: 'Compendium', demon: 'Digitalize demon', summoned: 'Summoned demon', stats: 'Your stats' };
  const KIND_ORDER = ['item', 'mod', 'tarot', 'soul', 'appearance', 'set', 'fusion set', 'expertise', 'clan', 'dlevel', 'quest', 'compendium', 'demon'];
  const lowIsGood = (name) => /taken|cooldown|cast time|incantation/i.test(name);

  // Stats that give nothing past a ceiling (from the user, 2026-11-23). The ceiling applies to the whole value
  // (gear + expertise, in the shown mode). To add one: its key from COMBAT above and its ceiling.
  // A negative one is a floor: cooldown stops at 5% of the skill's own (the server's lowest COOLDOWN_TIME).
  const CAPS = { final_crit: 100, lb_chance: 100, ta_chance: 100, pursuit_chance: 100, pursuit_power: 100, cooldown: -95 };
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
    page: Q.get('page') || 'overview',   // always opens on Overview (the last page is not remembered)
    busy: false, scanning: false, lastScan: 0,
    state: { active: null, scanned: false, clients: [], characters: [] },   // who is in game, whose data is shown
    compare: null,
    over: { open: null, demon: store.get('demon-open') === '1', allChanges: false, switching: null, aff: store.get('aff') || null },   // (aff: the affinity series shown on its own)   // Overview: the opened stat, demon details shown, every change listed, the demon being switched to
    demons: null,   // the COMP demons as last listed (the picker shows these at once, then looks at the game again)
    bonus: { q: '', group: 'all', open: new Set() },
    gear: { view: 'pieces', sel: null, cols: {}, colq: '', sort: null, short: Q.has('short') ? Q.get('short') === '1' : store.get('short') === '1' },
    dcm: { q: '', kind: 'all', open: new Set(), data: null, notes: {}, state: 'idle' },
    quest: { q: '', type: 'available', open: new Set(), data: null, state: 'idle' },
    gdb: { q: '', slot: 'all', known: true, open: new Set(), shown: 150, data: null, state: 'idle' },
    // Fusion Planner: the engine once loaded, which view, the picked demons (forward), the demon wanted and the ones planned before it (reverse)
    fus: { state: 'idle', eng: null, view: store.get('fus-view') === 'reverse' ? 'reverse' : 'forward', slots: [null, null, null], third: false, target: null, trail: [], with: '', shown2: 30, shown3: 30 },
    dun: { q: '', kind: 'all', open: new Set(), mob: new Set(), data: null, state: 'idle',   // Dungeon Explorer: opened places, opened enemies
      group: store.get('dun-group') !== '0', shown: new Set() },   // listed under the place you enter them from; the groups opened (all start folded)
    // Demon Rank: which demons, the level they are compared at ('base', '1' or '99'), the column sorted (an index of RANK_COLS) and its direction (1 = highest first)
    rank: { q: '', kind: 'contract', race: '', at: 'base', mt: false, sort: 0, dir: 1 },
    ddb: { q: '', kind: 'contract', race: '', align: '', sort: 'level', open: new Set(), level: {}, mtView: new Set(), data: null, state: 'idle', only: null },   // only: a demon id a link opened
    // Skill Database: the action type shown ('' = all), who can get it, element, sort, opened skills, how many rows drawn
    skl: { q: '', type: '', who: 'get', el: '', sort: 'name', open: new Set(), shown: 200, data: null, state: 'idle', byDemon: null, byDemonFrom: null },
    // Simulator: the character as read (init, base), the build being tried and its result (build, cur), what is open
    sim: { state: 'idle', error: '', init: null, base: null, cur: null, build: null, catalog: {}, tab: 'gear', open: null, expOpen: new Set(), allDiff: false, cap: null, seq: 0, timer: null, working: false, redraw: null,
      // saved builds: the list, which page is up ('home' = the builds, 'edit' = one build), the build in the editor
      // (id null = the unsaved one), the cards' results, and whether the list under "Simulator" in the left bar is open
      view: 'home', id: null, name: '', builds: [], cards: new Map(), jobs: Promise.resolve(), who: null,
      // the base of the build in the editor: a saved character's name, or '' for none (then level and align count);
      // inits / bases: what the reader said about each base, asked once per reading (the promise, and the answer)
      char: '', level: 0, align: 'NEUTRAL', inits: new Map(), bases: new Map(), useSeq: 0,
      demon: '',   // the build's digitalize demon: one of its base's COMP demons ("type:name"), or '' = not digitalized
      saveState: '', storeTimer: null, naming: false, confirmDel: false, navOpen: store.get('sim-nav') !== '0' },
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
      // what the demon summoned beside you gives you (only a normal total: nobody is out while digitalized)
      const summoned = n0((bonusRowOf(k, label, unit) || {}).summoned);
      // what VIT and SPD take off cooldown, each mode with its own stats
      const statsRow = bonusRowOf(k, label, unit) || {}, stats = n0(statsRow.stats), statsDigi = n0(statsRow.stats_digi);
      by[k] = { stat: k, label, unit, gear, gearDigi, exp, demon, summoned, stats, statsDigi };
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
  // The Simulator's saved builds hang under its entry (drawSimNav); the small arrow on the entry folds them away.
  const simNavBox = h('div', { class: 'nav-sub', hidden: true });
  const simNavToggle = h('button', { type: 'button', class: 'nav-fold', hidden: true, 'aria-label': 'Show or hide your saved builds',
    onclick: () => { S.sim.navOpen = !S.sim.navOpen; store.set('sim-nav', S.sim.navOpen ? '1' : '0'); drawSimNav(); } }, icon('chevron'));
  TOOLS.forEach(([id, label, ic]) => {
    const btn = h('button', { type: 'button', class: 'nav-item', 'data-page': id, 'data-tip': label, onclick: () => (id === 'sim' ? simGoHome() : go(id)) },
      icon(ic), h('span', { class: 'nav-label', text: label }));
    if (id === 'sim') nav.append(h('div', { class: 'nav-group' }, btn, simNavToggle), simNavBox); else nav.append(btn);
  });
  function go(page) {
    if (S.page === page) return;
    if (S.page === 'ddb') S.ddb.only = null;   // a demon a link opened alone: the whole list again next time
    S.page = page;
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
    drawSimNav();
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
  function affinityCard() {
    const DG = S.data.digitalize, aff = affinity(), n = aff.boost.length;
    // (no heading of its own: it shares the row's, "Key stats and affinities", with the stat sheet beside it)
    const card = h('section', { class: 'card', 'aria-label': 'Affinity' });
    if (n < 3) {   // a radar needs three spokes
      card.append(h('p', { class: 'note', text: n ? aff.boost.concat(aff.resistance).filter((e) => e.total).map((e) => `${e.name} ${fmt(e.total, '%')}`).join(' · ') : 'No affinity boosts or resistances yet.' }));
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
    show.forEach((k) => {
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
    const split = (e) => { const parts = [['gear', e.gear], ['expertise', e.exp], ...(digi() && DG ? [[DG.demon, e.demon]] : [])].filter(([, v]) => v); return parts.length > 1 ? ` (${parts.map(([name, v]) => `${name} ${fmt(v, '%')}`).join(' + ')})` : parts.length ? ` (${parts[0][0]})` : ''; };
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
      h('div', { class: 'seg aff-pick', role: 'group', 'aria-label': 'Show' }, pick(null, 'All'), AFF_SERIES.map(([k, label]) => pick(k, label, true))));
    // a line about the series shown on its own; nothing under "All" (the user asked for that hint to go)
    if (focus) card.append(h('p', { class: 'note', text: focus === 'cap' ? 'Boosts stop at 100%' + (raised.length ? `, except where something of yours raises the cap: ${raised.map((c) => `${c.name} ${fmt(c.total)}%`).join(', ')}.` : '. Nothing of yours raises it for any element.')
      : focus === 'res' ? 'How much of each element you resist. Inside the 0 ring is a weakness.'
      : 'What you add to the damage you deal with each element.' }));
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
  function refreshSummoned() { const el = $('.page-head .summoned'); if (el) el.replaceWith(summonedCard()); }

  /**
   * What you wear, in one line under the demon's: every worn piece as its icon from the game in a slot-like tile, then
   * the complete sets. No names or numbers here (those are the Gear page's): hovering a tile shows the game's tooltip,
   * a click opens that piece on the Gear page. Null before any gear was read.
   */
  function gearBand() {
    const items = S.v.gear.filter((g) => g.kind !== 'set'), sets = S.v.gear.filter((g) => g.kind === 'set');
    if (!items.length) return null;
    const open = (g) => { S.gear.view = 'pieces'; if (g) S.gear.sel = g.row; go('gear'); };
    const tile = (g) => withTip(h('button', { type: 'button', class: 'gear-pc', onclick: () => open(g),
      'aria-label': g.kind === 'set' ? `${setTitle(g)}, complete set` : `${g.slot}: ${g.item}` }, gearIcon(g)), gearTip(g));
    return h('section', { class: 'card band gear-band', 'aria-label': 'Gear' },
      h('h3', { class: 'band-title', text: 'Gear' }),
      // one run of tiles that wraps in a narrow window; the sets stay together (their label never ends a line alone)
      h('div', { class: 'band-items' }, items.map(tile),
        sets.length ? h('span', { class: 'gear-sets' }, h('span', { class: 'stat-label', text: sets.length === 1 ? 'Set' : 'Sets' }), sets.map(tile)) : null),
      h('button', { type: 'button', class: 'btn btn-sm', 'data-tip': 'Every piece and what it adds, on the Gear page', onclick: () => open(null) }, 'View more'));
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
      h('div', { class: 'dg-bars who-stats' }, mine.map(statRow))), summonedCard()));
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
      const srcs = [['sw-gear', 'Gear', r.gearNow], ['sw-exp', 'Expertise', r.exp], ...(digi() && DG ? [['sw-demon', DG.demon, r.demonNow]] : []),
        ...(!digi() ? [['sw-demon', summonedName(), r.summonedNow]] : []), ['sw-base', 'Your stats', r.statsNow]].filter(([, , v]) => v);
      const open = S.over.open === k;
      const bonusRow0 = bonusRowOf(k, r.label, r.unit);
      // a bar only where there is a cap, so every bar on the sheet means the same thing: how close to the cap
      // (full and soft red when past it, with an "Over cap" tag beside the name; by how much is in the tooltip and under the opened line)
      const meter = cap === null ? h('span') : h('span', { class: 'cap-bar' + (over ? ' over' : '') }, h('i', { style: `width:${Math.max(0, Math.min(100, (r.total / cap) * 100))}%` }));
      const row = h('button', { type: 'button', class: 'sheet-row', 'aria-expanded': String(open),
        'data-tip': (srcs.length ? srcs.map(([, name, v]) => `${name} ${f(v)}`).join(' · ') : 'Nothing adds to this yet')
          + (cap === null ? '' : `. Caps at ${fmt(cap, r.unit)}` + (over ? `: ${plain(over)} over, which adds nothing.` : '.')),
        onclick: () => { S.over.open = open ? null : k; render(); } },
        h('span', { class: 'chev' }, icon('chevron')),
        // past the cap: a short "Over cap" tag beside the name (by how much is in the tooltip and under the opened line)
        h('span', { class: 'sheet-name' }, h('span', { class: 'sheet-label', text: label }), over ? h('span', { class: 'tag tag-over', text: 'Over cap' }) : null), meter,
        h('span', { class: 'sheet-value' + (r.total < 0 && !lowIsGood(r.label) ? ' bad' : ''), text: fmt(r.total, r.unit) }));
      if (!open) return row;
      const bonusRow = bonusRow0;
      const line = (cls, name, value, note) => h('div', { class: 'sheet-src' }, h('i', { class: 'swatch ' + cls }),
        h('span', null, name, note ? h('small', { text: note }) : null), h('b', { text: value }));
      return [row, h('div', { class: 'sheet-detail' },
        srcs.length ? srcs.map(([cls, name, v]) => line(cls, name, f(v),
          cls === 'sw-gear' && r.extra ? (digi() ? `${f(r.extra)} of it only while digitalized` : `${f(r.extra)} more while digitalized`)
            : cls === 'sw-exp' ? toggleShareText(bonusRow0, 'it')
            : name === 'Your stats' ? statsNote(bonusRow0) : null))
          : h('p', { class: 'note', text: 'Nothing adds to this yet.' }),
        toggleShare(bonusRow0) ? h('p', { class: 'note', text: togglesNote() }) : null,
        cap === null ? null : h('p', { class: 'note', text: `Caps at ${fmt(cap, r.unit)}: ` + (over ? `the ${plain(over)} above it adds nothing, so those points could go elsewhere.`
          : pastCap(r.total, cap) >= 0 ? 'reached, more adds nothing.' : `${plain(Math.abs(pastCap(r.total, cap)))} to go.`) }),
        bonusRow ? h('button', { type: 'button', class: 'btn btn-sm', 'data-tip': 'Every source of this, on the Bonuses page', onclick: () => { S.bonus.q = bonusRow.bonus; S.bonus.group = 'all'; S.bonus.open.add(bonusRow.row); go('bonuses'); } },
          'View more') : null)];
    };
    const sheet = h('section', { class: 'card sheet', 'aria-label': 'Key stats' }, h('div', { class: 'sheet-cols' }, SHEET.map((col) => h('div', { class: 'sheet-col' },
      col.map(([title, stats]) => h('div', { class: 'sheet-group' }, h('h3', { class: 'list-head', text: title }), stats.map(sheetRow)))))));

    // One row under one heading: the stat sheet and, beside it, affinity. Both boxes start and end on the same lines
    // (affinity used to carry its own heading, which pushed its box below the sheet's: the user's "misalignment").
    // Both follow the shown mode, so the heading says which. Then what changed, across the page.
    page.append(sect('Key stats and affinities', digi() && DG ? `While digitalized with ${DG.demon}` : 'Not digitalized', null,
      h('div', { class: 'over-grid' }, sheet, affinityCard())));
    page.append(changesCard());

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
      onclick: () => { S.exp.q = c.name; go('expertise'); } }, h('span', { text: c.name }), h('b', { text: `${c.class}-${c.rank}` }));
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
      h('button', { type: 'button', class: 'btn btn-sm', 'data-tip': 'Every expertise and what it adds, on the Expertise page', onclick: () => { S.exp.q = ''; go('expertise'); } }, 'View more'));
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
    const sel = h('select', { class: 'select', 'aria-label': 'Compare with snapshot', onchange: () => { S.compare = sel.value; render(); } },
      earlier.slice().reverse().map((x) => h('option', { value: x.taken_at, selected: x.taken_at === base.taken_at, text: `${when(x.taken_at)} · ${countText(differs(x))}` })));
    const wrap = sect('What changed', h('label', { class: 'card-sub', style: 'display:inline-flex;align-items:center;gap:8px' }, 'since', sel), null, card);

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
        h('span', null, p.source, crystal ? h('span', { class: 'part-from', text: crystal }) : null, isTrue(p.digi) && p.kind !== 'stats' ? h('span', { class: 'tag ' + (digi() ? 'tag-digi' : 'tag-off'), style: 'margin-left:8px', text: 'Digi only' }) : null, isTrue(p.toggle) ? toggleTag('margin-left:8px') : null,
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
  const skillIcon = (r, folder) => {   // folder: another character's skill icons (the Simulator's imported base); else the shown character's
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
   * The Expertise page's opening summary, in the Overview Expertise band's style (the user's request): a top line with
   * the counts at the left and points / cap at the right (points / 100, "pt", as the game's Expertise window) over a thin
   * bar; then one compact line per raised expertise, regular ones by points and then the chains: its name and class-rank
   * (orange), how far it is to its highest class (max_class from the client's ExpertClassData; Class <max> Rank 0 is
   * "Max"), its points (chains cost none), and one run of the icons of every skill its reached ranks give (passives,
   * then actives). A line filters the passive list below to that expertise.
   */
  function expSummary(rows) {
    const R = S.data.ranks || {}, P = R.points || null, all = (R.ranks || []).filter((r) => n0(r.points) > 0);
    const pt = (v) => (n0(v) / 100).toFixed(2) + 'pt';
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
      P ? h('div', { class: 'band-points', 'data-tip': P.cap ? `Cap ${pt(P.cap)} = 17000.00pt + 1000.00pt per 10 levels (level ${P.level})`
          + (P.level === 99 ? ' + 1000.00pt at 99' : '') + ` + ${P.extension} expansions × 1000.00pt` : null },
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
        const ic = skillIcon(s);
        ic.dataset.tip = `${s.name} · ${s.active ? 'active' : 'passive'} (${s.rank})` + (s.off ? ', toggle switched off' : '');
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

  function pageExpertise() {
    const rows = S.data.expertise.filter((r) => !isDlevel(r) && !isExtra(r));
    const page = h('div', { class: 'page' });
    page.append(pageHead('Passive skills your expertise ranks unlock (this assumes you acquired every one), and your other learned passives. Digitalization Levels are on the Digitalize page; the quest bonus and Demonic Compendium are on Bonuses.'));
    page.append(expSummary(rows));
    // the expertise share of each bonus, without what the Digitalization Levels, quest bonus and compendium put into it
    const totals = S.data.bonuses.filter((r) => isTrue(r.counted)).map((r) => [r, round2(n0(r.expertise) - shareOf(r, ['dlevel', 'quest', 'compendium']))]).filter(([, v]) => v);
    page.append(sect('What expertise adds in total', `${rows.filter((r) => isTrue(r.counted)).length} passives counted`, null,
      totalsCard(null, null, totals, 'No expertise bonuses counted yet.')));
    passiveList(page, rows, S.exp, { id: 'exp-list', heading: 'Every passive', search: 'Search an expertise, passive or effect', rankTip: 'The rank that unlocks this passive', empty: 'No expertise read yet. Take a snapshot.' });
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
  function demonTile(d, on, pick, compact, tag, iconFolder) {   // iconFolder: another character's skill icons (the Simulator's imported base)
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
  // found on IMAGINE itself). Both sit beside this file.
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
      'The DCM list, zone names and shop stock are the IMAGINE client\'s. Drops, chests, presents and events come from the public server data, so IMAGINE may have changed them. Lines marked “IMAGINE note” are confirmed by players.')));
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
    page.append(h('p', { class: 'note', text: `${quests.length} quests, ${quests.filter((x) => x.walkthrough).length} with steps from the event files. Data made on ${D.data.generatedAt}. Numbers in brackets are the map grid position.` }));
    drawChips(); draw();
    return page;
  }
  // ---------- Tools: Gear Database ----------
  // gear-db.json is made by "imagine-reader geardb": what each piece gives is read from the IMAGINE client's current
  // tables; where it comes from is what the community spreadsheets say. It sits beside this file.
  // Item pictures are in gear-db-icons\ beside it (the game's own art, made locally with geardb --client, not in git).
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
    const facts = [it.slot, it.gender && it.gender !== 'Any' ? it.gender + ' only' : '', it.alignment ? it.alignment + ' only' : '', it.weapon || '', it.affinity ? it.affinity + ' affinity' : ''].filter(Boolean);
    box.append(h('div', { class: 'detail-meta' }, facts.map((x) => h('span', { class: 'tag', text: x }))));
    if (it.sheetOnly) box.append(h('p', { class: 'note', text: 'This row is from the spreadsheet only: its name matched no item in the game data, so the effects below are the sheet\'s own text.' }));
    // The sections are named and ordered as in a worn piece's tooltip on the Gear page: set bonus, then TIP_ORDER.
    // s.any: the bonus needs one of the listed pieces; otherwise it needs all of them
    (it.sets || []).forEach((s) => {
      const b = block('Set bonus', s.effects); if (!b) return;
      b.lastChild.prepend(h('li', { class: 'gdb-with', text: s.any && s.with.length > 1 ? `Pieces: this and any one of ${s.with.join(', ')}` : `Pieces: this, ${s.with.join(', ')}` }));
      box.append(b); });
    // sheet-only rows: "effects" is the sheet's slot text, not the game's characteristics
    [block('Appearance Bonus', it.appearance), block('Basic Features', it.stats), block(it.sheetOnly ? 'Effects' : 'Characteristics', it.effects),
      it.mods ? block('Modification Slot', [`${it.mods} slot${it.mods === 1 ? '' : 's'}`]) : null].filter(Boolean).forEach((b) => box.append(b));
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
      'Stats, effects and sets are read from the IMAGINE client\'s current item tables. "Where to get it" is traced through the public server data (drops, chests, shops, events), which IMAGINE may have changed. "Community notes" are what the community wrote down for IMAGINE, and can be imprecise.')));
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
            h('span', { class: 'chev' }, icon('chevron')), ddbImg(it.icon, 28, 'dcm-icon gdb-icon'), h('span', null, it.name, it.sheetOnly ? h('span', { class: 'src-count', text: 'sheet only' }) : null))),
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
  // ---------- Tools: Dungeon Explorer ----------
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
        Object.values(d.mobs).forEach((m) => { m.hay = [m.name, m.race || '', ...m.skills.map(sk), ...m.enemySkills.map(sk), ...m.traits.map(sk)].join(' ').toLowerCase(); });
        // a drop points at d.items[id] = [name, the game's description, picture]
        const item = (g) => { const [name, desc, icon] = d.items[g.i] || ['', '', null]; return Object.assign(g, { item: name, desc, icon }); };
        (d.everywhere || []).forEach(item);   // the game-wide drops every enemy that inherits drops can give
        d.dungeons.forEach((x, i) => {
          x.key = i;
          (x.shared || []).forEach(item); (x.bossChest || []).forEach(item); (x.chests || []).forEach(item);
          x.mobs.forEach((e) => { e.mob = d.mobs[e.m]; (e.drops || []).forEach(item); e.hay = [e.mob.hay, e.where || '', ...(e.drops || []).map((g) => g.item)].join(' ').toLowerCase(); });
          x.hay = [x.name, x.from || '', ...(x.shared || []).map((g) => g.item)].join(' ').toLowerCase();
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
        h('div', { class: 'dun-skill-row' }, list.map((id) => { const [n, d, icon] = D.skills[id];
          const el = icon ? h('span', { class: 'dun-skill', tabindex: 0, role: 'img', 'aria-label': n }, skillPic(icon))
            : h('span', { class: 'tag dun-skill-name', tabindex: 0, text: n });
          return withTip(el, () => h('div', null, tipHead(icon ? skillPic(icon) : null, n, kind), ...tipLines(d))); }))) : null;
    };
    [skills('Skills', m.skills, 'Skill'), skills('Enemy-only skills', m.enemySkills, 'Enemy-only skill', 'Skills only this enemy version uses'),
      skills('Traits', m.traits, 'Trait', 'Its innate passive features')].forEach((b) => b && box.append(b));
    if (e.drops) box.append(h('div', { class: 'gdb-block' }, h('span', { class: 'q-key', text: 'Drops' }),
      h('ul', { class: 'gdb-list dun-drop-list' }, e.drops.map((g) => withTip(h('li', { tabindex: 0 }, h('span', { class: 'gdb-kind dun-rate', text: `${nf.format(g.rate)}%` }),
        dunItemPic(g, 20), g.item, g.max ? h('span', { class: 'src-count', text: `up to ${fmt(g.max)}` }) : null),
        () => h('div', null, tipHead(dunItemPic(g, 32), g.item, `${nf.format(g.rate)}% per kill${g.max ? ` · up to ${fmt(g.max)}` : ''}`), ...tipLines(g.desc)))))));
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
          h('span', null, e.mob.name, e.boss ? h('span', { class: 'tag tag-boss', style: 'margin-left:8px', text: 'Boss' }) : null,
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
    const chestList = (list, who) => (list || []).map((g) => ({ i: g.i, item: g.item, desc: g.desc, icon: g.icon, best: g.rate, max: g.max || 0, from: [{ who, rate: g.rate }] }));
    const boss = chestList(x.bossChest, 'boss'), other = chestList(x.chests, 'chest');
    // One short warning for all of it: the rates are the vanilla server data's, and a place whose enemies drop nothing
    // there (Dark Babel: IMAGINE sets those on its own server) says so. The game-wide drops any enemy anywhere can give
    // (d.everywhere) are left off the page: the user asked to ignore them.
    if (loot.length || boss.length || other.length) box.append(h('div', { class: 'notice notice-warn dun-warn' }, icon('alert'), h('span', null,
      h('b', { text: 'Vanilla rates, not IMAGINE\'s. ' }),
      loot.length ? '' : 'No enemy drops here in the server data (IMAGINE keeps its own). ',
      boss.length || other.length ? 'Each chest item rolls on its own.' : '')));
    else box.append(h('div', { class: 'notice notice-warn dun-warn' }, icon('alert'), h('span', null, h('b', { text: 'No drops here in the server data. ' }), 'IMAGINE keeps its own.')));
    if (loot.length) box.append(h('div', { class: 'dun-loot-block' }, h('span', { class: 'q-key', text: `Drops here · ${loot.length === 1 ? '1 item' : `${loot.length} items`}` }), strip(loot, dunLootTip)));
    if (boss.length) box.append(h('div', { class: 'dun-loot-block' }, h('span', { class: 'q-key', 'data-tip': 'What the boss chest at the end of a stage can hold, over every stage of this place', text: `End chest · ${boss.length === 1 ? '1 item' : `${boss.length} items`}` }), strip(boss, dunLootTip)));
    if (other.length) box.append(h('div', { class: 'dun-loot-block' }, h('span', { class: 'q-key', 'data-tip': 'Other chests the events here open', text: `Other chests · ${other.length === 1 ? '1 item' : `${other.length} items`}` }), strip(other, dunLootTip)));
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
      if (!by.has(g.i)) by.set(g.i, { i: g.i, item: g.item, desc: g.desc, icon: g.icon, best: 0, max: 0, from: [] });
      const l = by.get(g.i); l.best = Math.max(l.best, g.rate); l.max = Math.max(l.max, g.max || 0); l.from.push({ who, rate: g.rate });
    };
    (x.shared || []).forEach((g) => add(g, null));
    x.mobs.forEach((e) => (e.drops || []).forEach((g) => add(g, e)));
    return (x.loot = [...by.values()].sort((a, b) => b.best - a.best || a.item.localeCompare(b.item)));
  }
  function dunLootTip(l) {
    const rows = l.from.slice().sort((a, b) => b.rate - a.rate);
    const shown = rows.slice(0, 10), more = rows.length - shown.length;
    const inChest = rows[0] && (rows[0].who === 'boss' || rows[0].who === 'chest');
    return h('div', null, tipHead(dunItemPic(l, 32), l.item, `Up to ${nf.format(l.best)}% ${inChest ? 'per chest' : 'per kill'}${l.max > 1 ? ` · up to ${fmt(l.max)} at once` : ''}`),
      ...tipLines(l.desc),
      h('div', { class: 'tt-sec', text: rows[0] && rows[0].who === 'all' ? 'Dropped by' : rows[0] && (rows[0].who === 'boss' || rows[0].who === 'chest') ? 'Found in' : 'Dropped here by' }),
      shown.map((r) => h('div', { class: 'tt-line dun-tt-row' }, h('span', { text: r.who === 'all' ? 'Any enemy, in any place' : r.who === 'boss' ? 'The boss chest at the end of a stage' : r.who === 'chest' ? 'A chest the events here open' : r.who ?`${r.who.mob.name}${r.who.boss ? ' (boss)' : ''} · level ${r.who.mob.level}` : 'Every enemy here' }), h('b', { text: `${nf.format(r.rate)}%` }))),
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
          ...[x.tarot, x.soul].filter(Boolean).flatMap((e) => [e.name, ...e.lines]), ...x.contract.map((c) => c.zone)].join(' ').toLowerCase();
      });
      if (S.page === 'ddb' || S.page === 'skills' || S.page === 'rank') render();
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
  function ddbDetail(x) {
    const D = S.ddb, d = D.data, box = h('div', { class: 'q-detail ddb-detail' });
    const sk = (id) => d.skills[id] || { name: 'A skill the game data does not name', desc: '' };
    const block = (label, ...kids) => h('div', { class: 'gdb-block' }, h('span', { class: 'q-key', text: label }), h('div', null, ...kids));
    box.append(h('div', { class: 'detail-meta' }, h('span', { class: 'tag', text: x.race }), h('span', { class: 'tag', text: `Base Lv ${x.level}` }), ddbAlign(x),
      x.gender && !/^(None|Na)$/i.test(x.gender) ? h('span', { class: 'tag', text: x.gender }) : null,
      x.fusion ? h('span', { class: 'tag', 'data-tip': 'The game\'s fusion picks it by level within its race.', text: 'Normal fusion' }) : null,
      x.mitama ? h('span', { class: 'tag', text: 'Mitama' }) : null,
      x.counted ? null : h('span', { class: 'tag tag-off', 'data-tip': 'The compendium lists it but does not count it toward its totals.', text: 'Not counted in the compendium' })));

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
        h('span', null, h('b', { text: s.name }), tag ? h('span', { class: 'src-count', text: tag }) : null, inline && s.desc ? h('small', { class: 'ddb-desc', text: s.desc }) : null)); };
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

    // Where to get it
    const where = h('ul', { class: 'gdb-list gdb-get' });
    x.contract.forEach((c) => where.append(h('li', null, h('span', { class: 'gdb-kind', text: 'Contract' }), h('b', { text: c.zone }), h('span', { class: 'gdb-rate', text: ddbLv(c) }),
      c.moon ? h('span', { class: 'tag', style: 'margin-left:8px', text: ddbMoon(c.moon) }) : null, c.season ? h('span', { class: 'tag', style: 'margin-left:8px', text: c.season + ' event' }) : null)));
    const dcm = S.dcm.data && S.dcm.data.items.find((it) => it.devilId === x.id);
    if (dcm) where.append(h('li', null, h('span', { class: 'gdb-kind', text: 'DCM' }), h('b', { text: dcm.lines && dcm.lines[0] ? `${dcm.lines[0].kind}: ${dcm.lines[0].text}` : 'No source in the game data' }),
      h('button', { type: 'button', class: 'btn btn-sm', style: 'margin-left:10px', onclick: () => { S.dcm.q = x.name; S.dcm.kind = 'all'; S.dcm.open = new Set([dcm.id]); go('dcm'); } }, 'Open in DCM Locator')));
    if (x.fusion) where.append(h('li', null, h('span', { class: 'gdb-kind', text: 'Fusion' }), h('span', null, `Normal two-demon fusion: a ${x.race} result picked by level`)));
    box.append(block('Where to get it', where.childNodes.length ? where : h('span', { class: 'zero', text: 'The game data names no place to contract it, no fusion by level and no DCM.' })));
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
      'Demons, stats, affinities, skills, tarot and soul are the IMAGINE client\'s. Where each spawns comes from the public server data, so IMAGINE may have changed it. Dungeon and boss copies of a demon are not listed on their own: they show under “Enemy only”.')));
    const all = D.data.demons;
    const inKind = (x, k) => (k === 'all' ? true : k === 'contract' ? x.contract.length > 0 : !x.contract.length);
    const inPick = (x) => (!D.race || x.race === D.race) && (!D.align || x.align === D.align);   // the Race and Alignment dropdowns
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
      rows.forEach((x) => {
        const open = D.open.has(x.id);
        const toggle = () => { if (D.open.has(x.id)) D.open.delete(x.id); else D.open.add(x.id); draw(); };
        const c0 = x.contract[0];
        const where = c0 ? h('span', null, c0.zone, h('span', { class: 'src-count', text: ddbLv(c0) + (x.contract.length > 1 ? ` · and ${x.contract.length - 1} more` : '') }))
          : h('span', { class: 'zero', text: x.fusion ? 'Fusion only' : '—' });
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
      count.textContent = `Showing ${rows.length} of ${all.length} demons. ${all.filter((x) => x.contract.length).length} can be contracted somewhere. Data made on ${D.data.generatedAt}.`;
    };
    const sel = (label, value, options, onchange) => h('label', { class: 'gdb-check' }, label,
      h('select', { class: 'input ddb-select', 'aria-label': label, onchange: (ev) => onchange(ev.target.value) }, options.map(([v, t]) => h('option', { value: v, selected: v === value ? true : null, text: t }))));
    page.append(h('div', { class: 'toolbar' }, searchBox(D.q, 'Search a demon, race, skill, feature, tarot or zone', (v) => { D.q = v; D.only = null; drawChips(); draw(); }),
      sel('Race', D.race, [['', 'All races']].concat(races.map((r) => [r, r])), (v) => { D.race = v; D.only = null; drawChips(); draw(); }),
      sel('Alignment', D.align, [['', 'All'], ['Law', 'Law'], ['Neutral', 'Neutral'], ['Chaos', 'Chaos']], (v) => { D.align = v; D.only = null; drawChips(); draw(); }),
      sel('Sort', D.sort, [['level', 'Level'], ['name', 'Name'], ['race', 'Race']], (v) => { D.sort = v; draw(); })));
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
    const inKind = (x, k) => (k === 'all' ? true : k === 'contract' ? x.contract.length > 0 : x.contract.length > 0 || x.fusion || hasDcm(x));
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
  const sklIcon = (x) => { const box = ddbImg(x.icon, 24, 'skill-ico'); if (!x.icon && SKL_NO_ICON[x.noIcon]) box.setAttribute('data-tip', SKL_NO_ICON[x.noIcon]); return box; };
  function loadSkillDb() {
    loadDemonDb();   // who learns each skill
    if (S.skl.state !== 'idle') return;
    S.skl.state = 'loading';
    fetch('skill-db.json', { cache: 'no-store' }).then((r) => (r.ok ? r.json() : null)).catch(() => null).then((d) => {
      S.skl.data = d; S.skl.state = d ? 'ready' : 'failed';
      if (d) d.skills.forEach((x) => { x.hay0 = sklWords([x.name, x.el, x.desc, ...(x.exp || []), x.fusion ? 'fusion' : '', x.barter ? 'barter' : '', ...(x.barter || []).map((b) => b.cost)].join(' ')); });
      if (S.page === 'skills') render();
    });
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
  function sklDetail(x, who) {
    const box = h('div', { class: 'q-detail skl-detail' });
    const block = (label, ...kids) => h('div', { class: 'gdb-block' }, h('span', { class: 'q-key', text: label }), h('div', null, ...kids));
    if (x.desc) box.append(block('In game', h('p', { class: 'ddb-desc skl-desc', text: x.desc })));
    const facts = [['Target', x.target], ['Reach', x.reach != null ? String(x.reach) : null, 'In the game\'s own distance units: compare skills with each other.'],
      ['Damage from', x.stat, 'The stat its damage or effect is worked out from.'], ['Needs', x.weapon],
      ['Heals', x.heal != null ? String(x.heal) : null, 'The healing modifier (a % is of the target\'s Max HP).'],
      ['MP while held', x.drain ? `${x.drain} MP` : null, 'Drained while the stance is held.']].filter((f) => f[1]);
    if (facts.length) box.append(block('Details', h('div', { class: 'skl-facts' }, facts.map(([k, v, tip]) =>
      h('div', { class: 'ddb-stat', 'data-tip': tip || null }, h('span', { class: 'q-key', text: k }), h('b', { text: v }))))));
    const get = h('ul', { class: 'gdb-list gdb-get' });
    (x.exp || []).forEach((e) => get.append(h('li', null, h('span', { class: 'gdb-kind', text: 'Expertise' }), h('b', { text: e }))));
    (x.barter || []).forEach((b) => get.append(h('li', null, h('span', { class: 'gdb-kind', text: 'Barter' }), h('b', { text: b.cost }),
      h('span', { class: 'src-count', text: b.for === 'demon' ? ' · your summoned demon learns it (Skill: Demon)' : ' · you learn it' }))));
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
    const helpBox = h('div'), body = h('tbody'), more = h('div', { class: 'skl-more' }), count = h('p', { class: 'note' });
    const drawChips = () => {
      const pool = all.filter(match);
      chips.replaceChildren(...[['', 'All types']].concat(types.map((t) => [t.key, t.name])).map(([k, label]) =>
        h('button', { type: 'button', class: 'chip-btn', 'aria-pressed': String(D.type === k), onclick: () => { D.type = k; D.shown = 200; drawChips(); draw(); } },
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
          h('span', { class: 'chev' }, icon('chevron')), sklIcon(x), h('span', null, x.name, x.el ? h('span', { class: 'src-count', text: x.el }) : null))),
        cell(x.cost, 'dim'), cell(x.cast), cell(x.cd), cell(x.power != null ? String(x.power) : x.heal != null ? `heals ${x.heal}` : ''), cell(x.stiff != null ? fmt(x.stiff) : ''),
        h('td', null, whoSummary(x))));
      if (open) body.append(h('tr', { class: 'detail-row' }, h('td', { colspan: COLS }, sklDetail(x, whoOf(x)))));
    };
    const draw = () => {
      body.replaceChildren(); more.replaceChildren();
      const rows = all.filter((x) => (!D.type || x.type === D.type) && match(x));
      const order = types.map((t) => t.key);
      rows.sort((a, b) => (D.type ? 0 : order.indexOf(a.type) - order.indexOf(b.type)) || SORTS[D.sort](a, b));
      let shown = 0, last = null;
      for (const x of rows) {
        if (shown >= D.shown) break;
        if (!D.type && x.type !== last) {
          last = x.type;
          const n = rows.filter((r) => r.type === x.type).length;
          body.append(h('tr', { class: 'dun-group-row', onclick: () => { D.type = x.type; D.shown = 200; drawChips(); draw(); window.scrollTo(0, 0); } },
            h('td', { colspan: COLS }, typeOf[x.type].name, h('span', { class: 'src-count', text: `${n} skill${n === 1 ? '' : 's'} · show only these` }))));
        }
        row(x); shown++;
      }
      if (!rows.length) body.append(h('tr', null, h('td', { colspan: COLS, class: 'empty-row', text: D.q.trim() ? `Nothing matches “${D.q.trim()}”.` : 'No skill in this group.' })));
      if (rows.length > D.shown) more.append(h('button', { type: 'button', class: 'btn btn-sm', onclick: () => { D.shown += 300; draw(); } }, `Show more (${rows.length - D.shown} left)`));
      drawHelp();
      const nGet = all.filter((x) => inWho(x, 'get')).length;
      count.textContent = `Showing ${Math.min(rows.length, D.shown)} of ${rows.length} skills (${all.length} in the game data, ${nGet} of them can be learned by a demon, come from expertise or an NPC exchange (barter), or are fusion skills). `
        + (S.ddb.state === 'ready' ? '' : 'Still loading the demons: who learns each skill shows once they are in. ')
        + `${d.skipped.untranslated} skills the client only names in Japanese, item uses and debug skills are left out. Data made on ${d.generatedAt}.`;
    };
    const sel = (label, value, options, onchange) => h('label', { class: 'gdb-check' }, label,
      h('select', { class: 'input ddb-select', 'aria-label': label, onchange: (ev) => onchange(ev.target.value) }, options.map(([v, t]) => h('option', { value: v, selected: v === value ? true : null, text: t }))));
    const els = [...new Set(all.map((x) => x.el).filter(Boolean))].sort();
    page.append(h('div', { class: 'toolbar' }, searchBox(D.q, 'Search a skill, element, effect, expertise or demon', (v) => { D.q = v; D.shown = 200; drawChips(); draw(); }),
      sel('Who can get it', D.who, SKL_WHO, (v) => { D.who = v; D.shown = 200; drawChips(); draw(); }),
      sel('Element', D.el, [['', 'All']].concat(els.map((e) => [e, e])), (v) => { D.el = v; D.shown = 200; drawChips(); draw(); }),
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
  // ---------- Simulator (Tools): other gear and expertise ranks, and what they would give ----------
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
  const simDemonName = (key) => String(key || '').replace(/^\d+:/, '').replace(/:\d+$/, '');   // "type:name:level" (older: "type:name")
  /** The demons of the editor's base (what that character's demon picker lists; asked once per reading). */
  function simDemonList() {
    const M = S.sim, entry = M.bases.get(simBaseKey(M.char, M.level, M.align));
    if (!entry || !M.char) return Promise.resolve([]);
    if (!entry.demonsAsk) entry.demonsAsk = host.call('simDemons', { base: M.char })
      .then((r) => { entry.demons = (r && r.demons) || []; entry.skillIcons = (r && r.skillIcons) || null; return entry.demons; })
      .catch(() => { entry.demons = []; return entry.demons; });
    return entry.demonsAsk;
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
  const simCardKey = (b) => { const at = simBaseOf(b); return JSON.stringify([simBaseKey(at.who, at.level, at.align), typeof b.demon === 'string' ? b.demon : 'saved:' + digi(), b.slots, b.points]); };
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
  /** The saved builds under "Simulator" in the left bar; the one in the editor is marked instead of the entry itself. */
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
      : [h('button', { type: 'button', class: 'btn btn-ghost btn-square btn-sm', 'aria-label': `Duplicate ${b.name}`, 'data-tip': 'Duplicate', onclick: () => simDuplicate(b, false) }, icon('copy')),
        h('button', { type: 'button', class: 'btn btn-ghost btn-square btn-sm', 'aria-label': `Delete ${b.name}`, 'data-tip': 'Delete', onclick: () => drawActs(true) }, icon('trash'))]);
    drawActs(false);
    const icons = res ? c.slots.map((k) => res.slots[k] && res.slots[k].icon).filter(Boolean) : [];
    // what it changes: your stats first, then combat, then the rest; within each, the biggest first
    const rank = { Stats: 0, Combat: 1, Affinity: 2, Other: 3 };
    const diffs = res && entry ? simDiff(entry.init.result, res).filter((x) => rank[x.group] !== undefined).sort((x, y) => rank[x.group] - rank[y.group] || y.size - x.size) : [];
    simFill(el,
      h('span', { class: 'sim-build-head' },
        h('span', { class: 'dgt-id' }, h('button', { type: 'button', class: 'sim-build-name', 'data-tip': `Open ${b.name}`, text: b.name }),
          h('span', { class: 'dgt-line', text: [at.who || 'No base', count, b.saved_at ? `saved ${when(b.saved_at)}` : ''].filter(Boolean).join(' · ') })),
        acts),
      icons.length ? h('span', { class: 'sim-build-icons' }, icons.slice(0, 8).map((src) => h('img', { src, alt: '' })), icons.length > 8 ? h('span', { class: 'dgt-line', text: `+${icons.length - 8}` }) : null) : null,
      !known ? h('span', { class: 'dgt-line', text: 'Working it out…' })
        : !res ? h('span', { class: 'dgt-line', text: known.why || 'This build can’t be worked out with the game data as it is now.' })
        : diffs.length ? h('span', { class: 'dgt-adds' }, diffs.slice(0, 4).map((x) => h('span', { class: 'dgt-add' }, h('span', { text: x.label }), h('b', { class: x.good ? 'good' : 'bad', text: fmt(x.d, x.unit, true) }))),
          diffs.length > 4 ? h('span', { class: 'dgt-line', text: `and ${diffs.length - 4} more` }) : null)
        : n ? h('span', { class: 'dgt-line', text: 'Moves none of your numbers.' }) : null);
    // its result, one card after the other; the card is drawn again when it is in
    if (!known) M.jobs = M.jobs.then(async () => {
      const now = M.cards.get(b.id);
      if (!(now && now.key === key)) {
        let r = null, why = '';
        let its = null;
        try { its = await simBase(at.who, at.level, at.align); }
        catch (e) { why = at.who ? `Its base, ${at.who}, can’t be read: ${String((e && e.message) || e)}` : String((e && e.message) || e); }
        if (!why) try { r = await host.call('simulate', { ...simArgs(at.who, at.level, at.align), demon: simDemonOf(b, its.init), slots: b.slots, points: b.points }); } catch (e) { r = null; }
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
  // The build's demon picker: "No demon" and the base's COMP demons as the cards of the other pages' picker (demonTile),
  // in a dropdown under the name in the demon band. The pick is the build's own: nothing is saved for the character.
  const simDemonsMenu = $('#sim-demons');
  let simDemonsFor = null;   // the button it hangs from
  async function drawSimDemons() {
    const M = S.sim, box = $('#sim-demons-list');
    const pick = (key) => { simDemonsMenu.hidePopover(); if (key === M.demon) return; M.demon = key; simRun(); render(); };
    const none = h('button', { type: 'button', class: 'char', 'aria-current': M.demon ? null : 'true', onclick: () => pick('') },
      h('span', { class: 'dm-icon dm-none dm-off', text: '–' }),
      h('span', { class: 'char-text' }, h('span', { class: 'char-name', text: 'No demon' }), h('span', { class: 'char-line', text: 'Not digitalized' })));
    const entry = M.bases.get(simBaseKey(M.char, M.level, M.align)) || {};
    if (!entry.demons) simFill(box, none, h('p', { class: 'note', style: 'padding:8px', text: `Reading ${M.char}’s demons…` }));
    const list = await simDemonList();
    simFill(box, none, list.map((d) => demonTile(d, d.key === M.demon, () => pick(d.key), false, null, entry.skillIcons)),
      list.length ? null : h('p', { class: 'note', style: 'padding:8px', text: `No demons seen for ${M.char} yet. Open the COMP in game as ${M.char}, then load the demons on the Digitalize page.` }));
    $('#sim-demons-note').textContent = `${M.char}’s COMP demons as last seen. The pick is this build’s: ${M.char}’s own digitalize demon is not changed.`;
    simPlace(simDemonsMenu, simDemonsFor);
  }
  /** Puts a dropdown under its button (above it when there is no room below). */
  function simPlace(menu, btn) {
    if (!btn || !btn.isConnected || !menu.matches(':popover-open')) return;
    const r = btn.getBoundingClientRect(), w = menu.offsetWidth, hgt = menu.offsetHeight;
    menu.style.left = Math.max(8, Math.min(r.left, window.innerWidth - w - 8)) + 'px';
    menu.style.top = (r.bottom + 6 + hgt > window.innerHeight - 8 ? Math.max(8, r.top - 6 - hgt) : r.bottom + 6) + 'px';
  }
  simDemonsMenu.addEventListener('toggle', (ev) => { if (ev.newState === 'open') { simPlace(simDemonsMenu, simDemonsFor); drawSimDemons(); } });
  $('#main').addEventListener('scroll', () => { if (simDemonsMenu.matches(':popover-open')) simDemonsMenu.hidePopover(); });   // its button moves away
  /** The Simulator's first page: "New build", then every saved build as a card. */
  function simHome(page) {
    const M = S.sim, builds = M.builds || [], n = simDraftCount();
    page.append(pageHead('Your builds. Open one to keep working on it, or start a new one. Nothing here touches the game or your saved snapshots.'));   // (no demon picker here: it is the editor's, and the cards follow what was picked there)
    page.append(h('div', { class: 'sim-builds' },
      h('button', { type: 'button', class: 'dg-tile sim-build sim-new', onclick: () => simOpen(null) },
        h('span', { class: 'sim-new-plus', 'aria-hidden': 'true' }, icon('plus')),
        h('span', { class: 'dgt-id' }, h('span', { class: 'dgt-name', text: 'New build' }),
          h('span', { class: 'dgt-line', text: n ? `Continue where you left off: ${n} change${n === 1 ? '' : 's'} not saved yet`
            : simDraft().base ? `Continue where you left off: ${simDraft().base} as imported, not saved yet` : 'Start empty, or import a character as its base' }))),
      builds.map(simCard)));
    page.append(h('p', { class: 'note', text: builds.length
      ? 'Each card compares the build with its base as last read (digitalized when the build has a demon); one with no base shows what it gives by itself. A saved build keeps every change you make to it.'
      : 'No saved builds yet. Start a new one and press “Save build”: it then shows up here and under Simulator in the left bar.' }));
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
      const res = await host.call('simulate', { ...simNow(), demon: M.demon, slots: M.build.slots, points: M.build.points });
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
  function simDiff(a, b) {
    // b digitalized: against a digitalized too where it has a demon (what another demon, or other gear, changes),
    // else against a as it is (what digitalizing adds)
    const d = !!b.digi, da = d && !!a.digi, out = [], sa = simStatsOf(a, da), sb = simStatsOf(b, d);
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
    s[part] = id;
  }

  // --- the picker: what goes in one part of a piece ---
  const simDlg = $('#sim-pick');
  let simPickDone = null;   // tidies up after the open picker
  $('#sim-pick-close').addEventListener('click', () => simDlg.close());
  simDlg.addEventListener('click', (ev) => { if (ev.target === simDlg) simDlg.close(); });   // a click on the dimmed page around it
  simDlg.addEventListener('close', () => { if (simPickDone) simPickDone(); simPickDone = null; });

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
    if (!alive) return;

    const spec = M.build.slots[k], was = M.init.build.slots[k];
    const crystal = part === 'tarot' || part === 'soul';
    const inUse = part === 'whole' ? (spec && spec.basic === spec.id && spec.special === spec.id ? spec.id : -1) : spec ? spec[part] : -1;
    const asRead = was ? (part === 'whole' ? -1 : was[part]) : -1;
    const linesOf = (it) => (part === 'basic' ? it.basic : part === 'special' ? it.special : part === 'id' ? it.look : [...it.basic, ...it.special, ...it.look]);
    let opts = crystal
      ? cat[part].map((c) => ({ id: c.id, name: c.name, lines: c.lines, sets: c.sets }))
      : cat.items.map((it) => ({ id: it.id, name: it.name, tag: it.tag, sex: it.sex, lines: linesOf(it), sets: part === 'id' || part === 'whole' ? it.sets : [], own: (part === 'basic' || part === 'special') && spec && it.id === spec.id }));
    // a piece made for the other sex can't be worn: it is left out where the piece itself is picked (as a donor of features it is fine)
    const mySex = M.init.meta.sex, worn = part === 'id' || part === 'whole';
    if (mySex && worn) opts = opts.filter((o) => !o.sex || o.sex === mySex || o.id === (spec && spec.id));
    if (crystal) opts.unshift({ id: 0, name: part === 'tarot' ? 'No tarot' : 'No soul fusion', lines: [], sets: [], first: true });
    if (part === 'whole') opts.unshift({ id: 0, name: 'Nothing: leave this slot empty', lines: [], sets: [], first: true });
    opts.forEach((o) => {
      o.gives = o.lines.length + o.sets.length > 0;
      o.hay = [o.name, o.tag || '', ...o.lines.map((l) => l.t), ...o.sets.flatMap((s) => [...s.with, ...s.lines.map((l) => l.t)])].join(' ').toLowerCase();
      o.rank = o.id === inUse ? 0 : o.first ? 1 : o.own ? 2 : o.id === asRead ? 3 : 4;
    });
    opts.sort((a, b) => a.rank - b.rank || a.name.localeCompare(b.name));

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
      preview.replaceChildren(h('span', { class: 'sim-preview-label', text: `With ${o.first ? 'that' : o.name}:` }), h('span', { class: 'note', text: 'working it out…' }));
      const mine = ++look;
      timer = setTimeout(async () => {
        const cand = simClone(M.build);
        simApply(cand, k, part, o.id);
        let res;
        try { res = await host.call('simulate', { ...simNow(), demon: M.demon, slots: cand.slots, points: cand.points }); } catch (e) { res = null; }
        if (!alive || mine !== look) return;
        const head = h('span', { class: 'sim-preview-label', text: `With ${o.first ? 'that' : o.name}:` });
        if (!res) { preview.replaceChildren(head, h('span', { class: 'note', text: 'could not be worked out.' })); return; }
        const d = simDiff(M.cur, res).filter((x) => x.group !== 'Only in some fights' && x.group !== 'Partner');
        if (!d.length) { preview.replaceChildren(head, h('span', { class: 'note', text: 'no change to your numbers.' })); return; }
        const top = d.slice(0, 9);
        simFill(preview, head, top.map((x) => h('span', { class: 'sim-chip ' + (x.good ? 'good' : 'bad'), 'data-tip': `${x.label}: ${fmt(x.from, x.unit)} → ${fmt(x.to, x.unit)}` },
          h('span', { text: x.label }), h('b', { text: fmt(x.d, x.unit, true) }))), d.length > top.length ? h('span', { class: 'note', text: `and ${d.length - top.length} more` }) : null);
      }, 160);
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
      const row = (o, i) => h('div', { class: 'sim-opt', role: 'option', 'aria-selected': String(i === st.active), 'data-i': i, onclick: () => choose(o), onpointermove: () => setActive(i) },
        h('div', { class: 'sim-opt-head' }, h('b', { text: o.name }),
          o.tag ? h('span', { class: 'tag', text: o.tag }) : null,
          o.sex ? h('span', { class: 'tag', text: `${o.sex} only` }) : null,
          o.id === inUse ? h('span', { class: 'tag sim-tag-use', text: 'In use' }) : null,
          o.own ? h('span', { class: 'tag', text: 'This piece’s own' }) : null,
          o.id === asRead && o.id !== inUse ? h('span', { class: 'tag', text: 'As last read' }) : null),
        o.lines.length ? h('div', { class: 'sim-opt-lines' }, o.lines.map((l, j) => [j ? ' · ' : null,
          h('span', { class: [l.c ? 'is-' + l.c : '', st.hit && st.hit(l.t) ? 'sim-hit' : ''].join(' ').trim() || null, text: simLineText(l) })])) : null,
        o.sets.map((s) => h('div', { class: 'sim-opt-lines sim-opt-set' }, h('span', { class: 'q-key', text: `${crystal ? 'Fusion set' : 'Set'} with ${s.with.join(', ')}: ` }), s.lines.map((l) => simLineText(l)).join(' · ') || 'nothing the game data has numbers for')),
        o.gives || o.first ? null : h('div', { class: 'sim-opt-lines zero', text: 'Gives nothing here' }));
      list.replaceChildren(...st.rows.slice(0, st.shown).map(row));
      if (!st.rows.length) list.append(h('p', { class: 'empty-row', text: words.length ? `Nothing matches “${st.q.trim()}”.` : 'Nothing fits here.' }));
      if (st.rows.length > st.shown) list.append(h('button', { type: 'button', class: 'btn btn-sm sim-more', onclick: () => { st.shown += 120; draw(); } }, `Show more (${st.rows.length - st.shown} left)`));
      showPreview(st.rows[st.active]);
    };
    const input = h('input', { class: 'input', type: 'search', spellcheck: 'false', 'aria-label': 'Search', placeholder: crystal ? 'Search a crystal or an effect (e.g. rapid)' : 'Search a name or an effect (e.g. limit break)',
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

  function pageSim() {
    const M = S.sim;
    const page = h('div', { class: 'page page-sim' });   // (no snapshot of the shown character needed: a build can stand on any character, or on none)
    simLoad();
    if (M.state !== 'ready') {
      page.append(pageHead(SIM_SUB));
      page.append(M.state === 'failed'
        ? h('div', { class: 'notice notice-warn' }, icon('alert'), h('div', null, h('b', { text: 'The simulator can’t start. ' }), M.error, ' ',
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
      const entry = M.bases.get(simBaseKey(M.char, M.level, M.align)) || {}, list = entry.demons || null;
      if (!list) simDemonList().then(() => { if (bandBox.isConnected) simFill(bandBox, demonBand()); });
      const d = M.demon && list ? list.find((x) => x.key === M.demon) : null, O = d ? d.offer : null;
      const shown = M.demon ? (d ? d.name : simDemonName(M.demon)) : 'No demon';
      const pic = h('div', { class: 'demon-pic ' + (M.demon ? alignClass(O && O.alignment) : 'none'), 'aria-hidden': 'true' }, h('span', { text: M.demon ? shown.charAt(0) : '–' }));
      if (d && d.icon) pic.append(h('img', { src: d.icon, alt: '' }));
      const name = h('button', { type: 'button', class: 'name-btn', popovertarget: 'sim-demons', 'aria-haspopup': 'menu',
        'data-tip': M.demon ? `This build is digitalized with ${shown}. Pick another of ${M.char}’s demons, or no demon` : `Pick one of ${M.char}’s demons to see this build digitalized with it`,
        onclick: (ev) => { simDemonsFor = ev.currentTarget; } }, shown, h('span', { class: 'chev down' }, icon('chevron')));
      const chips = !M.demon ? [h('span', { class: 'tag tag-off', text: 'Not digitalized' })]
        : !d ? [h('span', { class: 'tag tag-off', text: list ? `Not among ${M.char}’s demons as last seen: not counted` : 'Reading the demons…' })]
        : [h('span', { class: 'tag', text: `Lv ${fmt(d.level)}` }), d.race ? h('span', { class: 'tag', text: d.race }) : null,
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
    page.append(h('div', { class: 'page-head sim-head' }, headLeft, headRight), simFill(bandBox, demonBand()), h('div', { class: 'sim-layout' }, editor, out));

    // The build's name and what can be done with it. Drawn only when that changes (never by a result coming in), so
    // a name being typed is left alone.
    function drawTitle() {
      const back = h('button', { type: 'button', class: 'btn btn-ghost sim-back', 'data-tip': 'All your builds', onclick: simGoHome }, h('span', { class: 'chev' }, icon('chevron')), 'Builds');
      if (!M.id && !M.naming) {
        simFill(headLeft, back, h('b', { class: 'sim-name-text', text: 'New build' }), h('span', { class: 'card-sub', text: 'not saved' }),
          h('button', { type: 'button', class: 'btn btn-primary', 'data-tip': 'Keep this build: it gets a name, a card and a line under Simulator', onclick: () => { M.naming = true; drawTitle(); } }, 'Save build…'));
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
          : [h('button', { type: 'button', class: 'btn btn-ghost btn-square', 'aria-label': 'Duplicate this build', 'data-tip': 'Duplicate: a copy to change freely',
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
    function slotCard(k) {
      const spec = M.build.slots[k], was = M.init.build.slots[k], res = M.cur.slots[k] || {}, slotName = meta.slots[k];
      const fresh = spec && res.id === spec.id;   // the result on screen is of this build (a moment after a change it is not yet)
      const changed = !simSame(spec, was), open = M.open === k;
      const name = spec ? (fresh ? res.name : '…') : 'Empty';
      const bits = !spec || !fresh ? [] : [res.basic ? `features of ${res.basic}` : '', res.special ? `characteristics of ${res.special}` : '', res.tarot ? `tarot ${res.tarot}` : '', res.soul ? `soul ${res.soul}` : ''].filter(Boolean);
      const card = h('div', { class: 'sim-slot' + (open ? ' open' : '') + (changed ? ' changed' : '') });
      card.append(h('button', { type: 'button', class: 'sim-slot-head', 'aria-expanded': String(open), 'data-fk': 'slot' + k, onclick: () => { M.open = open ? null : k; drawEditor(); } },
        spec && fresh && res.icon ? h('img', { class: 'slot-icon', src: res.icon, alt: '' }) : h('span', { class: 'slot-icon sim-noicon' }),
        h('span', { class: 'sim-slot-text' },
          h('span', { class: 'sim-slot-top' }, h('span', { class: 'slot-name', text: slotName }), h('b', { class: 'sim-slot-item', text: name }),
            fresh && res.limit ? h('span', { class: 'tag sim-tag-limit', 'data-tip': 'Fusion bonus at MAX', text: 'LIMIT' }) : null,
            changed && !blank ? h('span', { class: 'tag sim-tag-changed', text: 'Changed' }) : null),
          bits.length ? h('span', { class: 'sim-slot-sub', text: bits.join(' · ') }) : null),
        h('span', { class: 'chev' }, icon('chevron'))));
      if (!open) return card;

      const body = h('div', { class: 'sim-slot-body' });
      card.append(body);
      // The chips of one part: what was picked for it before. A click uses that one again (Undo still goes back to
      // what you wear); the x takes the chip off the list. The one in use is marked.
      const hist = simHist();
      const histChips = (part) => {
        const list = hist[`${k}:${part}`] || [];
        if (!list.length) return null;
        const cur = !spec ? -1 : part === 'whole' ? (spec.basic === spec.id && spec.special === spec.id ? spec.id : -1) : spec[part];
        return h('div', { class: 'sim-hist', role: 'group', 'aria-label': 'Picked before' }, h('span', { class: 'sim-hist-label', text: 'Picked before' }),
          list.map((x) => h('span', { class: 'sim-hchip' + (x.id === cur ? ' on' : '') },
            h('button', { type: 'button', class: 'sim-hchip-use', 'aria-pressed': String(x.id === cur), 'data-fk': `h${k}${part}${x.id}`, 'data-tip': x.id === cur ? `${x.name}: in use` : `Use ${x.name}`,
              onclick: () => { if (x.id === cur) return; simApply(M.build, k, part, x.id); change(); } }, h('span', { text: x.name })),
            h('button', { type: 'button', class: 'sim-hchip-x', 'aria-label': `Remove ${x.name} from this list`, 'data-tip': 'Remove from this list',
              onclick: () => { simHistDrop(k, part, x.id); drawEditor(); } }, icon('x')))));
      };
      const pickBtn = (part, text, fk) => h('button', { type: 'button', class: 'sim-pickbtn', 'data-fk': fk || `p${k}${part}`, onclick: () => simPick(k, part, () => { drawEditor(); drawHead(); }) },
        h('span', { text }), h('span', { class: 'chev' }, icon('chevron')));
      if (!spec) {
        body.append(h('p', { class: 'note', text: was ? 'Taken off in this build.' : 'Nothing is worn here.' }),
          h('div', { class: 'sim-slot-foot' }, h('button', { type: 'button', class: 'btn btn-sm btn-primary', 'data-fk': `w${k}`, onclick: () => simPick(k, 'whole', () => { drawEditor(); drawHead(); }) }, 'Put a piece here…'),
            was ? h('button', { type: 'button', class: 'btn btn-sm', onclick: () => { M.build.slots[k] = simClone(was); change(); } }, 'Put it back') : null),
          histChips('whole'));
        return card;
      }
      const L = fresh ? res.lines : {};
      const undo = (part) => (was && spec[part] !== was[part]
        ? h('button', { type: 'button', class: 'btn btn-ghost btn-sm sim-undo', 'data-tip': 'Back to what you wear', onclick: () => { spec[part] = was[part]; change(); } }, 'Undo') : null);
      const partRow = (part, value, lines, none, extra) => h('div', { class: 'sim-part' + (was && spec[part] !== was[part] ? ' changed' : '') },
        h('div', { class: 'sim-part-key' }, h('span', { text: SIM_PARTS[part][0].split(' · ')[0] }), SIM_PARTS[part][0].includes(' · ') ? h('small', { text: SIM_PARTS[part][0].split(' · ')[1] }) : null),
        h('div', { class: 'sim-part-val' }, h('div', { class: 'sim-part-pick' }, pickBtn(part, value), undo(part)), histChips(part), fresh ? simLines(lines, none) : null, extra || null));
      // the sets this piece is in (by name, as the set list has them)
      const mySets = fresh ? M.cur.sets.filter((s) => s.kind === 'set' && s.name.split(', ').includes(simPlain(res.name))) : [];
      const setBox = mySets.length ? h('div', { class: 'sim-sets' }, mySets.map((s) => h('div', { class: 'sim-set' + (s.active ? '' : ' off') },
        h('span', { class: 'q-key', text: `Set ${s.worn} worn${s.active ? '' : ' (not complete)'}: ${s.name}` }), simLines(s.lines)))) : null;
      body.append(
        partRow('id', fresh ? res.name : '…', L.appearance, mySets.length ? null : 'No appearance bonus and no set.', setBox),
        partRow('basic', fresh ? (res.basic || `${res.name} (its own)`) : '…', (L.item || []).filter((l) => l.n !== 'fusion great success'), 'No basic features.'),
        partRow('special', fresh ? (res.special || `${res.name} (its own)`) : '…', L.characteristic, 'No characteristics.'));
      if (meta.tarot[k]) body.append(partRow('tarot', fresh ? (res.tarot || 'None') : '…', L.tarot, spec.tarot ? 'Nothing the game data has numbers for.' : null));
      if (meta.soul[k]) body.append(partRow('soul', fresh ? (res.soul || 'None') : '…', L.soul, spec.soul ? 'Nothing the game data has numbers for.' : null));
      // Fusion bonus: LIMIT is every bonus of this kind of piece at the top of its table ("MAX" in the game)
      if (fresh && res.fuse_max && res.fuse_max.length) {
        const max = res.fuse_max.map((f) => `${f.stat} +${f.bonus}`).join(' · ');
        const now = (res.fuse || []).map((f) => `${f.stat} +${f.bonus}`).join(' · ');
        const sw = h('input', { type: 'checkbox', class: 'switch', role: 'switch', checked: res.limit ? true : null, 'data-fk': `f${k}`, 'aria-label': 'LIMIT: fusion bonus at MAX',
          onchange: () => { spec.fuse = sw.checked ? [50, 50, 50] : [0, 0, 0]; change(); } });
        body.append(h('div', { class: 'sim-part' + (was && !simSame(spec.fuse, was.fuse) ? ' changed' : '') },
          h('div', { class: 'sim-part-key' }, h('span', { text: 'Fusion bonus' }), h('small', { text: 'LIMIT' })),
          h('div', { class: 'sim-part-val' }, h('label', { class: 'sim-limit' }, sw, h('span', null, h('b', { text: res.limit ? 'LIMIT (MAX)' : 'Not at MAX' }),
            h('small', { text: res.limit ? ` ${max}` : now ? ` now ${now}; LIMIT gives ${max}` : ` LIMIT gives ${max}` })),
            was && !simSame(spec.fuse, was.fuse) ? h('button', { type: 'button', class: 'btn btn-ghost btn-sm sim-undo', onclick: (ev) => { ev.preventDefault(); spec.fuse = simClone(was.fuse); change(); } }, 'Undo') : null))));
      }
      if (fresh && L.mod && L.mod.length) body.append(h('div', { class: 'sim-part' },
        h('div', { class: 'sim-part-key' }, h('span', { text: 'Mods' }), h('small', { text: 'kept as they are' })), h('div', { class: 'sim-part-val' }, simLines(L.mod))));
      body.append(h('div', { class: 'sim-slot-foot' },
        h('button', { type: 'button', class: 'btn btn-sm', 'data-fk': `w${k}`, 'data-tip': SIM_PARTS.whole[1], onclick: () => simPick(k, 'whole', () => { drawEditor(); drawHead(); }) }, 'Replace the whole piece…'),
        h('button', { type: 'button', class: 'btn btn-sm', onclick: () => { M.build.slots[k] = null; change(); } }, 'Take it off'),
        changed ? h('button', { type: 'button', class: 'btn btn-sm', onclick: () => { M.build.slots[k] = simClone(was); change(); } }, 'Undo this slot') : null));
      const whole = histChips('whole');
      if (whole) body.append(whole);
      return card;
    }

    // --- expertise ---
    let expLive = null;   // refreshes the parts of the expertise tab a result changes (the sliders are left alone)
    function expertiseTab() {
      const defs = M.init.expertise, P = M.build.points, P0 = M.init.build.points;
      const baseRank = (id) => Math.floor(n0(P0[id]) / 10000), rankOf = (id) => Math.floor(n0(P[id]) / 10000);
      const box = h('div', { class: 'sim-exp' });
      // Points spent against the cap. The cap can be set by hand, to plan for more expansions or a higher level: it
      // only changes what counts as free or over here, never a stat. The line is built once and only its texts are
      // refreshed, so the box being typed in is never replaced under the cursor.
      const cap0 = M.base.expertise.cap, RP = (mine && S.data.ranks && S.data.ranks.points) || {};
      const capNow = () => (M.cap === null || M.cap === undefined ? cap0 : M.cap);
      const spent = h('span', { class: 'card-sub' }), freeTag = h('span', { class: 'tag' }), meter = h('span', { class: 'exp-bar' }, h('i')), capNote = h('p', { class: 'note' });
      const capIn = h('input', { class: 'input sim-cap-in', type: 'number', min: 0, step: 1000, inputmode: 'numeric', placeholder: 'no cap', 'aria-label': 'Expertise point cap, in pt' });
      const setCap = (v, typing) => {
        M.cap = v === null || v === cap0 ? null : Math.max(0, Math.round(v));
        simSave(); drawBar(typing); drawHead();
      };
      const capUndo = h('button', { type: 'button', class: 'btn btn-ghost btn-sm sim-undo', 'data-tip': cap0 ? `Back to your cap in game, ${simPt(cap0)}` : 'Back to no cap, as in game', onclick: () => setCap(null) }, 'Undo');
      const capStep = (by, text, tip) => h('button', { type: 'button', class: 'btn btn-ghost btn-square btn-sm', 'aria-label': tip, 'data-tip': tip, onclick: () => setCap(n0(capNow()) + by) }, text);
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
        const total = defs.filter((d) => d.counts).reduce((a, d) => a + n0(P[d.id]), 0), cap = capNow();
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
        h('span', null, u.pick ? h('span', { class: 'sim-unlock-pick', 'data-tip': 'The game lets you learn only this many of these. Which one counts is the one you learned in game.', text: u.pick === 1 ? 'Learn one of: ' : `Learn ${u.pick} of: ` }) : null, u.skills.map((s, i) => [i ? ', ' : null, h('span', { class: s.passive ? '' : 'sim-active', 'data-tip': s.passive ? null : 'An active skill: it adds no stats by itself', text: s.name })])))) : h('li', { text: 'The game data lists no skills for its ranks.' }));
      const row = (d) => {
        const base = baseRank(d.id), open = M.expOpen.has(d.id);
        const label = h('span', { class: 'sim-exp-rank' }), was = h('span', { class: 'sim-exp-was' });
        const slider = h('input', { type: 'range', class: 'sim-range', min: 0, max: d.max, step: 1, value: rankOf(d.id), 'aria-label': `${d.name}: rank` });
        const undo = h('button', { type: 'button', class: 'btn btn-ghost btn-sm sim-undo', onclick: () => set(base) }, 'Undo');
        const detail = h('div', { class: 'sim-exp-detail' });
        const el = h('div', { class: 'sim-exp-row' });
        const paint = () => {
          const v = rankOf(d.id);
          label.textContent = simRank(v);
          slider.value = v; slider.style.setProperty('--fill', `${d.max ? (v / d.max) * 100 : 0}%`);
          el.classList.toggle('changed', v !== base);
          was.textContent = v !== base ? `was ${Math.floor(base / 10)}-${base % 10}` : '';
          undo.hidden = v === base;
          if (M.expOpen.has(d.id)) detail.replaceChildren(unlockList(d, v)); else detail.replaceChildren();
          drawBar();
        };
        const set = (v) => {
          v = Math.max(0, Math.min(d.max, v));
          P[d.id] = v === base ? n0(P0[d.id]) : v * 10000;   // untouched: the exact points as read (part of a rank counts toward chains)
          paint(); drawHead(); simRunSoon();
        };
        slider.addEventListener('input', () => set(Number(slider.value)));
        const step = (by, text, tip) => h('button', { type: 'button', class: 'btn btn-ghost btn-square btn-sm', 'aria-label': tip, 'data-tip': tip, onclick: () => set(rankOf(d.id) + by) }, text);
        el.append(h('button', { type: 'button', class: 'sim-exp-name', 'aria-expanded': String(open), 'data-tip': 'What its ranks unlock',
            onclick: (ev) => { if (M.expOpen.has(d.id)) M.expOpen.delete(d.id); else M.expOpen.add(d.id); ev.currentTarget.setAttribute('aria-expanded', String(M.expOpen.has(d.id))); paint(); } },
            h('span', { class: 'chev' }, icon('chevron')), h('span', { text: d.name })),
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
        const now = M.cur.expertise.ranks, before = M.base.expertise.ranks;
        chainBox.replaceChildren(...chains.map((d) => {
          const a = n0(before[d.id]), b = n0(now[d.id]);
          if (!a && !b && !M.expOpen.has('chains')) return null;
          const needs = d.needs.map((x) => `${Math.round(x.share * 100)}% of ${x.name}${x.rank ? ` (needs ${simRank(x.rank)})` : ''}`).join(' + ');
          return h('div', { class: 'sim-chain' + (a !== b ? ' changed' : '') + (b ? '' : ' off'), 'data-tip': needs ? `${d.name} = ${needs}` : null },
            h('span', { text: d.name }), h('span', { class: 'sim-exp-val' }, h('span', { class: 'sim-exp-rank', text: b ? simRank(b) : 'Not reached' }), a !== b ? h('span', { class: 'sim-exp-was', text: a ? `was ${Math.floor(a / 10)}-${a % 10}` : 'new' }) : null));
        }).filter(Boolean));
        if (!chainBox.childNodes.length) chainBox.append(h('p', { class: 'note', text: 'No chain expertise reached.' }));
      };
      expLive = () => { drawBar(); drawChains(); };
      const allChains = h('button', { type: 'button', class: 'btn btn-ghost btn-sm', onclick: () => { if (M.expOpen.has('chains')) M.expOpen.delete('chains'); else M.expOpen.add('chains'); allChains.textContent = M.expOpen.has('chains') ? 'Only the ones reached' : 'Show every chain'; drawChains(); } },
        M.expOpen.has('chains') ? 'Only the ones reached' : 'Show every chain');
      simFill(box, bar,
        trained.length ? [h('h3', { class: 'list-head', text: 'Your expertise' }), trained.map(row)] : null,
        h('h3', { class: 'list-head sim-list-head' }, 'Chain expertise', h('small', { text: 'worked out from the ones above' }), allChains), chainBox,
        rest.length ? [h('h3', { class: 'list-head', text: 'Not trained yet' }), rest.map(row)] : null);
      drawChains();
      return box;
    }

    function drawEditor() {
      const keep = document.activeElement && editor.contains(document.activeElement) ? document.activeElement.dataset.fk : null;
      const c = simChanges();
      const tab = (id, label, n) => h('button', { type: 'button', 'aria-pressed': String(M.tab === id), onclick: () => { M.tab = id; drawEditor(); } }, label, n ? h('span', { class: 'sim-count', text: n }) : null);
      expLive = null;
      simFill(tabsBox, tab('gear', 'Gear', c.slots.length), tab('expertise', 'Expertise', c.ranks.length));
      simFill(bodyBox, M.tab === 'gear' ? h('div', { class: 'sim-slots' }, M.build.slots.map((_, k) => slotCard(k))) : expertiseTab());
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
        const cdef = COMBAT.find((x) => x[0] === k), unit = cdef[2];

        const to = simRowVal(simCombatRow(cur, k), D), from = simRowVal(simCombatRow(base, k), Db), d = round2(to - from);
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
          : h('div', { class: 'chg' }, h('span', { text: x.label }),
            h('span', { class: 'chg-vals' }, x.fresh ? 'new: ' : fmt(x.from, x.unit) + ' → ', h('b', { text: x.gone ? 'gone' : fmt(x.to, x.unit) })), simDelta(x)))));
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
      if (S.page === 'fusion') render();
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

  /** A demon picker: a search field that lists matching demons under it (arrow keys, Enter, Esc). */
  function demonCombo(E, current, placeholder, onPick, list) {
    const wrap = h('div', { class: 'fus-combo' });
    const input = h('input', { class: 'input', type: 'search', placeholder, value: current ? current.label : '', 'aria-label': placeholder, spellcheck: 'false', autocomplete: 'off', role: 'combobox', 'aria-expanded': 'false' });
    const menu = h('div', { class: 'fus-menu', role: 'listbox', hidden: true });
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
    const pick = (d) => { close(); input.value = d.label; onPick(d); };
    input.addEventListener('focus', () => { input.select(); open(); });
    input.addEventListener('input', () => { at = 0; open(); });
    input.addEventListener('blur', () => { close(); if (!input.value.trim() && current) onPick(null); else if (current) input.value = current.label; });
    input.addEventListener('keydown', (ev) => {
      if (ev.key === 'ArrowDown') { ev.preventDefault(); if (menu.hidden) open(); else { at = Math.min(at + 1, hits.length - 1); draw(); } }
      else if (ev.key === 'ArrowUp') { ev.preventDefault(); at = Math.max(at - 1, 0); draw(); }
      else if (ev.key === 'Enter') { if (!menu.hidden && hits[at]) { ev.preventDefault(); pick(hits[at]); } }
      else if (ev.key === 'Escape') { if (!menu.hidden) { ev.stopPropagation(); close(); } }
    });
    wrap.append(h('div', { class: 'search' }, icon('search'), input), menu);
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
  function pluginLine(E, sp) {
    if (!sp || !sp.plugin) return null;
    const p = E.F.plugins[String(sp.plugin)];
    // the game's key item names it ("Plugin: Uriel", "Enhanced Plugin: Skadi"); its ways to get it are joined with " · "
    const ways = p && p.how ? p.how.split(' · ') : ['The data does not say where to get it.'];
    return h('div', { class: 'fus-plugin' }, icon('info'), h('div', null, h('b', { text: p && p.name ? `Needs the key item “${p.name}”` : 'Needs a fusion plugin' }),
      ways.length > 1 ? h('ul', null, ways.map((w) => h('li', { text: w }))) : h('div', { text: ways[0] })));
  }
  /** The same, short, for a recipe card: the key item's name, and its ways to get it in a tip (cards keep one size). */
  function pluginNote(E, sp) {
    if (!sp || !sp.plugin) return null;
    const p = E.F.plugins[String(sp.plugin)];
    const ways = p && p.how ? p.how.split(' · ') : ['The data does not say where to get it.'];
    return withTip(h('div', { class: 'fus-plugin fus-plugin-sm', tabindex: '0' }, icon('info'), h('span', { class: 'fus-plugin-name', text: p && p.name ? `Needs “${p.name}”` : 'Needs a fusion plugin' })),
      () => h('div', null, h('b', { text: p && p.name ? `How to get “${p.name}”` : 'Fusion plugin' }), h('ul', { class: 'fus-tip-ways' }, ways.map((w) => h('li', { text: w })))));
  }

  function pageFusion() {
    const U = S.fus, page = h('div', { class: 'page page-fusion' });
    loadFusion();
    page.append(pageHead('Pick two or three demons to see what they make, or pick the demon you want to see every way to make it.',
      h('div', { class: 'seg', role: 'group', 'aria-label': 'Planner' },
        [['forward', 'Fuse demons'], ['reverse', 'Find a recipe']].map(([id, label]) =>
        h('button', { type: 'button', 'aria-pressed': String(U.view === id), onclick: () => { U.view = id; store.set('fus-view', id); render(); } }, label)))));
    if (U.state !== 'ready') {
      page.append(h('p', { class: 'note', text: U.state === 'failed' ? 'The fusion data could not be loaded (fusion-data.json is missing beside the page).' : 'Loading the fusion data…' }));
      return page;
    }
    page.append(U.view === 'reverse' ? fusionReverse(U.eng) : fusionForward(U.eng));
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
        d.mitamaFused ? null : h('button', { type: 'button', class: 'btn btn-sm', style: 'margin-left:auto', onclick: () => { U.view = 'reverse'; U.target = d; U.trail = []; render(); } }, 'Other ways to make it')));
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

  function fusionReverse(E) {
    const U = S.fus, box = h('div', { class: 'fus-body' });
    if (!E.built()) {
      // the list of demons to pick from needs every two- and three-demon fusion worked out once (about a million): say so, then do it
      box.append(h('p', { class: 'note', role: 'status' }, h('span', { class: 'spin ink', style: 'margin-right:8px;vertical-align:-2px' }), 'Working out every fusion once (a second or two)…'));
      setTimeout(() => { E.build(); if (S.page === 'fusion') render(); }, 30);
      return box;
    }
    // an ingredient can be clicked to plan it only when some fusion makes it
    const plan0 = (d) => { if (U.target) U.trail.push(U.target); U.target = d; U.shown2 = 30; U.shown3 = 30; render(); };
    const chip = (d, tag) => demonChip(E, d, undefined, E.canMake(d) ? plan0 : null, tag);
    const top = h('div', { class: 'toolbar' }, demonCombo(E, U.target, 'Which demon do you want?', (d) => { U.target = d; U.trail = []; U.shown2 = 30; U.shown3 = 30; render(); }, E.targets()));
    box.append(top);
    if (U.trail.length) box.append(h('nav', { class: 'fus-trail', 'aria-label': 'Planned before' }, h('span', { class: 'src-count', style: 'margin-left:0', text: 'Back to:' }),
      U.trail.map((d, i) => h('button', { type: 'button', class: 'btn btn-ghost btn-sm', onclick: () => { U.target = d; U.trail = U.trail.slice(0, i); render(); } }, d.label))));
    const T = U.target;
    if (!T) { box.append(h('p', { class: 'note', text: 'Pick a demon to see every special fusion, two-demon fusion and tri-fusion that makes it, cheapest first. Click a demon in a recipe to see how to make that one too.' })); return box; }
    box.append(h('div', { class: 'fus-out fus-target' }, demonPic(E, T, true), h('div', null, h('div', { class: 'fus-out-name', text: T.label }), h('div', { class: 'card-sub', text: `${T.raceName} · base level ${T.lvl}` }))));
    const R = E.recipes(T);
    let q = '';
    const hasQ = (r) => !q || r.some((d) => d.label.toLowerCase().includes(q) || d.raceName.toLowerCase().includes(q));
    const elemTag = (r) => {
      const e = r.find((d) => E.isElemental(d)), o = r.find((d) => !E.isElemental(d));
      if (!e || !o || r.length !== 2) return null;
      const up = o.lvl < T.lvl || (o.lvl === T.lvl && o.id < T.id);
      return h('span', { class: 'tag ' + (o.id === T.id ? 'fus-same' : up ? 'fus-up' : 'fus-down'), text: o.id === T.id ? 'Stays the same' : up ? 'Rank up' : 'Rank down' });
    };
    // a recipe as a card: its kind and its levels added up on top, then one ingredient per line; the grid gives every
    // card of a section the same size (CSS: grid-auto-rows 1fr)
    const rcard = (parts, head, foot) => h('li', { class: 'fus-rcard' },
      h('div', { class: 'fus-rcard-head' }, head || null, h('span', { class: 'src-count fus-sum', 'data-tip': 'The ingredients’ base levels added up', text: `Σ ${parts.reduce((s2, p) => s2 + p.d.lvl, 0)}` })),
      parts.map((p, i) => [i ? h('span', { class: 'fus-plus', 'aria-hidden': 'true', text: '+' }) : null, chip(p.d, p.tag)]),
      foot || null);
    const recipeRow = (r) => rcard(r.map((d) => ({ d })), elemTag(r));
    const section = (title, sub, rows, key) => {
      const list = rows.filter(hasQ), shown = U[key] || 30;
      const sec = h('section', { class: 'card fus-sec' }, h('div', { class: 'card-head' }, h('h3', { class: 'card-title' }, title, h('span', { class: 'src-count', text: q ? `${list.length} of ${rows.length}` : `${rows.length}` })), sub ? h('span', { class: 'card-sub', text: sub }) : null));
      if (!list.length) sec.append(h('p', { class: 'note', text: rows.length ? `None with “${U.with.trim()}”.` : 'None.' }));
      else sec.append(h('ul', { class: 'fus-recipes' }, list.slice(0, shown).map(recipeRow)));
      if (list.length > shown) sec.append(h('button', { type: 'button', class: 'btn btn-sm', style: 'justify-self:start', onclick: () => { U[key] = shown + 60; drawLists(); } }, `Show ${Math.min(60, list.length - shown)} more`));
      return sec;
    };
    const any = R.special.length || R.two.length || R.three.length;
    if (!any) { box.append(h('p', { class: 'notice' }, icon('info'), h('span', { text: `No fusion makes ${T.label}. It comes from somewhere else in the game (a quest, an event, a contract), or only from demons the planner doesn’t list.` }))); return box; }
    if (R.special.length) {
      const sec = h('section', { class: 'card fus-sec' }, h('div', { class: 'card-head' }, h('h3', { class: 'card-title', text: 'Special fusion' }), h('span', { class: 'card-sub', text: 'Always this result, whatever the levels' })));
      const grid = h('ul', { class: 'fus-recipes' });
      R.special.forEach((sp) => {
        const parts = sp.src.map(([id, variant]) => ({ d: E.byId.get(id),
          tag: variant ? null : h('span', { class: 'tag', 'data-tip': 'Only this exact demon; its other versions don’t count', text: 'exact' }) })).filter((p) => p.d);
        grid.append(rcard(parts, h('span', { class: 'tag tag-digi', text: parts.length === 3 ? 'Tri-fusion' : 'Two demons' }), pluginNote(E, sp)));
      });
      sec.append(grid);
      box.append(sec);
    }
    const lists = h('div', { class: 'fus-body' });
    const elemRows = R.two.filter((r) => r.some((d) => E.isElemental(d)));
    const drawLists = () => {
      q = (U.with || '').trim().toLowerCase();
      lists.replaceChildren(section('Two demons', elemRows.length ? `${elemRows.length} with an elemental (rank up or down)` : '', R.two, 'shown2'),
        section('Tri-fusion (three demons)', '', R.three, 'shown3'));
    };
    if (R.two.length || R.three.length) {
      box.append(h('div', { class: 'toolbar' }, searchBox(U.with || '', 'Only recipes with this demon or race', (v) => { U.with = v; U.shown2 = 30; U.shown3 = 30; drawLists(); })), lists);
      drawLists();
    }
    box.append(h('p', { class: 'note', text: 'Recipes use the demons the race chart can make, at their base levels; a demon that has levelled up can give a higher result. Special fusions are listed whatever the levels. Mitama-fused demons are left out: the game won’t fuse them.' }));
    return box;
  }

  const RENDER = { sim: pageSim, overview: pageOverview, bonuses: pageBonuses, gear: pageGear, expertise: pageExpertise, digitalize: pageDigitalize, history: pageHistory, dcm: pageDcm, quests: pageQuests, geardb: pageGearDb, dungeons: pageDungeons, fusion: pageFusion, ddb: pageDemonDb, rank: pageRank, skills: pageSkills };
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
    S.data = { snapshots: d.snapshots || [], gear: d.gear || [], expertise: d.expertise || [], bonuses: d.bonuses || [], ranks: d.ranks || null, digitalize: d.digitalize || null, icons: d.icons || {}, unlocks: d.unlocks || [], summoned: d.summoned || null };
    S.meta = d.meta || {};
    if (!S.data.digitalize) S.mode = 'normal';   // no demon saved: there is nothing to be digitalized with (the choice itself is kept)
    S.sim.state = 'idle';   // the Simulator starts again from what was just loaded (the build being tried is kept)

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
        : d.loaded === false ? toast('warn', `${d.name} can’t be used yet`, 'The game only loads a demon’s stats and skills when you look at it. Click it in the COMP in game, or open the fusion window at the Cathedral of Shadows (the Master), which loads your whole COMP at once. Then press Load demons from the game.', 12000)
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
  const A = { el: $('#assistant'), view: null, state: 'off' };   // state: off | loading | on | missing | failed
  const assistantWanted = () => Q.get('assistant') !== '0' && (Q.get('assistant') === '1' || store.get('assistant') !== 'off');
  function fillAssistant() {
    const on = store.get('assistant') !== 'off';
    $('#set-assistant').checked = on;
    $('#assistant-help').textContent = !on ? 'Off. Its model and animations come from your game’s own files.'
      : A.state === 'loading' ? 'Getting its model from the game’s files…'
      : A.state === 'missing' ? 'Its model comes from the game’s own files, and the app doesn’t know yet where the game is installed. Start IMAGINE and it will show up.'
      : A.state === 'failed' ? 'It couldn’t be drawn here: this window has no 3D graphics (WebGL).'
      : 'Its model and animations come from your game’s own files. Click it for a trick; it also reacts when you take a snapshot.';
  }
  // Clicks go to the box it stands in (its canvas is bigger, for the animations, and lets clicks through).
  A.el.addEventListener('click', () => { if (A.view) A.view.poke(); });
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
  async function startAssistant() {
    if (A.view || A.state === 'loading' || !window.NMAssistant) return;
    A.state = 'loading'; fillAssistant();
    let model = null;
    try { model = await host.call('assistant'); } catch (e) { model = null; }
    if (!assistantWanted()) { A.state = 'off'; fillAssistant(); return; }   // switched off while it loaded
    if (!model) { A.state = 'missing'; fillAssistant(); return; }
    A.el.hidden = false;
    const yaw = parseFloat(store.get('assistant-yaw'));   // the way it was left facing
    try { A.view = window.NMAssistant.mount($('#assistant-view'), model, Number.isFinite(yaw) ? { yaw } : {}); } catch (e) { A.view = null; }
    if (!A.view) { A.el.hidden = true; A.state = 'failed'; fillAssistant(); return; }
    // Far enough from the window's edges that its widest and lowest animation fits the way it faces now. (Turned
    // another way an animation can reach further: its canvas has the room, and only the window's own edge can cut it.)
    document.body.style.setProperty('--assistant-right', Math.max(18, A.view.home.r + 4) + 'px');
    document.body.style.setProperty('--assistant-bottom', Math.max(2, A.view.home.b) + 'px');
    A.state = 'on'; document.body.classList.add('has-assistant'); fillAssistant();
  }
  function stopAssistant() {
    if (A.view) A.view.dispose();
    A.view = null; A.state = 'off'; A.el.hidden = true; document.body.classList.remove('has-assistant');
    // its canvas gave up its 3D context for good: the next start gets a new one
    const old = $('#assistant-view'), fresh = old.cloneNode(false);
    fresh.removeAttribute('style'); old.replaceWith(fresh);
    fillAssistant();
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
  $('#open-settings').addEventListener('click', () => { S.alignNote = ''; fillSettings(); fillAbout(); dlg.showModal(); });
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
    }
    requestAnimationFrame(() => requestAnimationFrame(() => host.call('ready').catch(() => {})));
    if (host.native) {
      setInterval(pollStatus, 5000);
      document.addEventListener('visibilitychange', () => { if (!document.hidden) pollStatus(); });   // back from minimized: look now
    }
  })();
})();
