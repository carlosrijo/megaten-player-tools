// The quest panel (panel.html): the next step of the quests you follow, in a small window BABEL keeps above the game.
// It only shows what it is sent: the app reads the quest progress every few seconds while the panel is open and posts
// { progress } here (main.rs Panel; the same answer as the Quest Tracker's questProgress). Every quest in progress, one
// line each (those with something to do in your zone first, open), which the player opens, closes and hides
// (localStorage babel-panel-hidden / babel-panel-open, shared with the main window: same browser profile; the Quest
// Tracker's hide button writes the same list). The steps come from quest-guide.json, like the Quest Database.
(() => {
  'use strict';
  const wv = window.chrome && window.chrome.webview;
  const $ = (id) => document.getElementById(id);
  function h(tag, attrs, ...kids) {
    const el = document.createElement(tag);
    if (attrs) for (const [k, v] of Object.entries(attrs)) {
      if (v === null || v === undefined || v === false) continue;
      if (k === 'class') el.className = v;
      else if (k === 'text') el.textContent = v;
      else if (k.startsWith('on')) el.addEventListener(k.slice(2), v);
      else el.setAttribute(k, String(v));
    }
    for (const kid of kids.flat(3)) if (kid !== null && kid !== undefined && kid !== false) el.append(kid);
    return el;
  }
  const send = (cmd) => { if (wv) wv.postMessage({ cmd }); };
  // the app's line icons (app.js ICONS): shown / hidden
  const ICONS = {
    eye: '<path d="M2.5 12S6 5.5 12 5.5 21.5 12 21.5 12 18 18.5 12 18.5 2.5 12 2.5 12z"/><circle cx="12" cy="12" r="3"/>',
    'eye-off': '<path d="M2.5 12S6 5.5 12 5.5 21.5 12 21.5 12 18 18.5 12 18.5 2.5 12 2.5 12z"/><circle cx="12" cy="12" r="3"/><path d="M4 4l16 16"/>',
  };
  function icon(name) {
    const s = document.createElementNS('http://www.w3.org/2000/svg', 'svg');
    s.setAttribute('class', 'icon'); s.setAttribute('viewBox', '0 0 24 24'); s.setAttribute('aria-hidden', 'true');
    s.innerHTML = ICONS[name] || '';
    return s;
  }

  let guide = null, progress = null, faces = null;   // faces: npc-pictures.json (an NPC's id -> their picture)
  const zoneNames = new Map();

  // ---- what to do next on a quest's current step ----
  const at = (x) => (x.where ? ` in ${x.where}` : '') + (x.grid ? ` (${x.grid})` : '');
  const here = (x) => !!(x && x.zoneId && progress && progress.zone === x.zoneId);
  const picks = (x) => (x.pick && x.pick.length ? h('div', { class: 'p-picks' }, x.pick.map((p, i) => [i ? h('span', { class: 'q-arrow', text: '›' }) : null, h('span', { class: 'q-pick', text: p })])) : null);
  function place(x, verb) {
    const face = x.who && x.npc != null && faces ? faces[x.npc] : null;
    const who = x.who ? (face ? h('span', { class: 'q-who' }, h('img', { class: 'q-face', src: face, alt: '', onerror: (e) => e.target.remove() }), h('b', { text: x.who })) : h('b', { text: x.who })) : x.kind === 'spot' ? h('b', { text: 'the spot' }) : x.kind === 'object' ? h('b', { text: 'the quest object' }) : null;
    return h('div', { class: 'p-do' }, verb, who ? [' ', who] : null, at(x), here(x) ? [' ', h('span', { class: 'tag q-here-tag', text: 'here' })] : null);
  }
  function lead(b, first) {
    return [place(b, `${first ? 'First' : 'Then'}, ${b.kind === 'npc' && b.who ? 'talk to' : b.do ? b.do.toLowerCase() : b.kind === 'spot' ? 'walk to' : 'examine'}`), picks(b),
      b.fights && b.fights.length ? h('div', { class: 'p-note', text: `You fight ${b.fights.join(', ')}.` }) : null,
      b.result ? h('div', { class: 'p-note', text: b.result }) : null];
  }
  function nextStep(q, a) {
    const s = q.steps.find((x) => x.phase === a.step);
    if (!s) return [h('div', { class: 'p-note', text: 'The guide has no step for where you are in this quest.' })];
    const out = [];
    // kill goals with the game's counters
    (s.goals || []).forEach((g, i) => {
      const c = s.goalCount && s.goalCount[i];
      const have = c && c[2] === 'kill' ? Math.min(Number(a.counters[c[0]]) || 0, c[1]) : null;
      out.push(h('div', { class: have !== null && have >= c[1] ? 'p-goal-met' : null, text: have !== null ? `${g}: ${have}/${c[1]}${have >= c[1] ? ' ✓' : ''}` : g }));
    });
    const ways = s.ways || [];
    // the way in your zone first, else the first one
    const w = ways.find((x) => here(x) || (x.before || []).some(here) || (x.talks || []).some(here)) || ways[0];
    if (!w) {
      if (!out.length && s.objective) out.push(h('div', { class: 'p-do', text: s.objective }));
      return out;
    }
    if (w.talks) {
      // a multi-talk step: the talks not done yet (their hidden flag not held)
      const left = w.talks.filter((t) => !(t.flag && progress.flags && progress.flags.includes(t.flag)));
      // only talks with a hidden flag can be known done; the quest's own counter never reaches the client
      const known = w.talks.filter((t) => t.flag).length;
      out.push(h('div', { class: 'p-note', text: known ? `${w.do} (${w.talks.length - left.length} of ${known} done):` : `${w.do}:` }));
      if (w.talks.some((t) => !t.flag && !t.last)) out.push(h('div', { class: 'p-note', text: 'The game doesn\'t say which of these you have done.' }));
      // a person, or a quest object or spot (Act 6's blade lights, then Celu Tower); the object's lead-ins are left out
      // here: the panel is a reminder, the Quest Tracker has the whole step
      left.forEach((t) => out.push(place(t, t.who ? (t.last ? 'Last, talk to' : 'Talk to') : (t.last ? `Last, ${(t.do || '').toLowerCase()}` : t.do)), picks(t),
        t.times ? h('div', { class: 'p-note', text: `${t.times} times in all.` }) : null));
      (s.optional || []).forEach((x) => out.push(place(x, 'Optional: talk to')));
      return out;
    }
    // what comes first (getting there, the person appearing): inside the step's zone, only what happens there
    const before = (w.before || []).filter((b) => !(progress.zone && progress.zone === w.zoneId && b.zoneId !== w.zoneId));
    before.forEach((b, i) => out.push(...lead(b, i === 0)));
    const verb = w.who ? (before.length ? 'Then, talk to' : 'Talk to') : w.do;
    out.push(place(w.who ? w : { ...w, who: null }, w.who ? verb : `${before.length ? 'Then, ' : ''}${w.do}`), picks(w));
    if (w.fights && w.fights.length) out.push(h('div', { class: 'p-note', text: `You fight ${w.fights.join(', ')}.` }));
    if (w.times) out.push(h('div', { class: 'p-note', text: `Do this ${w.times}.` }));
    // several ways picked at random from the same talk: say so once
    if (ways.filter((x) => x.random && !x.finishes).length > 1) out.push(h('div', { class: 'p-note', text: 'The game picks what is asked at random.' }));
    if (w.finishes) out.push(h('div', { class: 'p-note', text: 'This finishes the quest.' }));
    return out;
  }

  // what the player chose, kept in the profile's storage (shared with the Quest Tracker's "Hide from the panel"):
  // hidden quests, and quests opened or closed by hand (otherwise: open when its step has something here)
  const load = (k, d) => { try { return JSON.parse(localStorage.getItem(k) || '') ?? d; } catch (e) { return d; } };
  const save = (k, v) => { try { localStorage.setItem(k, JSON.stringify(v)); } catch (e) { /* preferences only */ } };
  const hidden = () => load('babel-panel-hidden', []);
  const opened = () => load('babel-panel-open', {});
  let showHidden = false;
  const stepOf = (q, a) => q.steps.find((s) => s.phase === a.step);
  const hereNow = (q, a) => (stepOf(q, a)?.ways || []).some((w) => here(w) || (w.before || []).some(here) || (w.talks || []).some((t) => here(t) || (t.before || []).some(here)));

  function draw() {
    const body = $('body');
    $('zone').textContent = progress && progress.zone && zoneNames.has(progress.zone) ? `· ${zoneNames.get(progress.zone)}` : '';
    if (!guide) { body.replaceChildren(h('p', { class: 'p-empty', text: 'Loading the quests…' })); return; }
    if (!progress || !progress.active) {
      body.replaceChildren(h('p', { class: 'p-empty', text: 'Not following the game right now: with IMAGINE running and your character logged in, your quests show here.' }));
      return;
    }
    const hid = new Set(hidden()), open = opened();
    // every quest in progress the guide knows, those with something to do here first
    const quests = progress.active.filter((a) => guide.has(a.id)).map((a) => ({ q: guide.get(a.id), a, here: hereNow(guide.get(a.id), a) }))
      .sort((x, y) => (y.here - x.here) || x.q.title.localeCompare(y.q.title));
    const row = ({ q, a, here: isHere }) => {
      const steps = q.steps.filter((s) => s.phase >= 0), i = steps.findIndex((s) => s.phase === a.step);
      const isOpen = q.id in open ? open[q.id] : isHere, isHidden = hid.has(q.id);
      const toggle = () => { const o = opened(); o[q.id] = !isOpen; save('babel-panel-open', o); draw(); };
      const hide = (ev) => {
        ev.stopPropagation();
        const ids = hidden().filter((id) => id !== q.id);
        if (!isHidden) ids.push(q.id);
        save('babel-panel-hidden', ids);
        draw();
      };
      return h('div', { class: 'p-quest' + (isOpen ? ' open' : '') + (isHidden ? ' hidden' : '') },
        h('div', { class: 'p-title', role: 'button', tabindex: '0', 'aria-expanded': String(isOpen), onclick: toggle, onkeydown: (e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); toggle(); } } },
          h('span', { class: 'p-chev', text: isOpen ? '▾' : '▸' }),
          h('span', { class: 'p-name', text: q.title }),
          isHere ? h('span', { class: 'tag q-here-tag', text: 'here' }) : null,
          h('span', { class: 'tag q-live-tag', text: i >= 0 ? `${i + 1}/${steps.length}` : '…', title: i >= 0 ? `Step ${i + 1} of ${steps.length}` : 'In progress' }),
          // the icon is what a click does (as in the Quest Tracker): the crossed-out eye hides a shown quest, the eye
          // shows a hidden one again
          h('button', { class: 'p-btn p-hide', type: 'button', title: isHidden ? 'Show this quest again' : 'Hide this quest', 'aria-label': isHidden ? 'Show this quest again' : 'Hide this quest', onclick: hide }, icon(isHidden ? 'eye' : 'eye-off'))),
        isOpen && !isHidden ? h('div', { class: 'p-steps' }, nextStep(q, a)) : null);
    };
    // the hidden quests first, in a section that opens and closes; then the ones shown
    const hiddenQ = quests.filter((x) => hid.has(x.q.id)), shownQ = quests.filter((x) => !hid.has(x.q.id));
    const rows = [];
    if (hiddenQ.length) {
      const toggle = () => { showHidden = !showHidden; draw(); };
      rows.push(h('div', { class: 'p-section', role: 'button', tabindex: '0', 'aria-expanded': String(showHidden), onclick: toggle, onkeydown: (e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); toggle(); } } },
        h('span', { class: 'p-chev', text: showHidden ? '▾' : '▸' }), icon('eye-off'), `Hidden (${hiddenQ.length})`));
      if (showHidden) rows.push(h('div', { class: 'p-hidden' }, hiddenQ.map(row)));
    }
    rows.push(...shownQ.map(row));
    if (!shownQ.length) rows.push(h('p', { class: 'p-empty', text: quests.length ? 'Every quest in progress is hidden: open “Hidden” above to show one again.' : 'No quest in progress.' }));
    body.replaceChildren(...rows);
  }

  // ---- wiring ----
  $('head').addEventListener('mousedown', (e) => { if (e.button === 0 && !e.target.closest('button')) send('drag'); });
  $('close').addEventListener('click', () => (wv ? send('close') : window.close()));
  window.addEventListener('storage', (e) => { if (e.key === 'babel-panel-hidden') draw(); });   // hidden or shown again in the Quest Tracker
  if (wv) wv.addEventListener('message', (ev) => { if (ev.data && 'progress' in ev.data) { progress = ev.data.progress; draw(); } });
  fetch('npc-pictures.json').then((r) => (r.ok ? r.json() : null)).catch(() => null).then((f) => { faces = f; if (f && guide) draw(); });
  fetch('quest-guide.json', { cache: 'no-store' }).then((r) => (r.ok ? r.json() : null)).catch(() => null).then((d) => {
    guide = d ? new Map(d.quests.map((q) => [q.id, q])) : new Map();
    if (d) d.quests.forEach((q) => q.steps.forEach((s) => (s.ways || []).flatMap((w) => [w, ...(w.before || []), ...(w.talks || [])])
      .forEach((x) => { if (x.zoneId && x.where && !zoneNames.has(x.zoneId)) zoneNames.set(x.zoneId, x.where); })));
    draw();
    send('ready');
  });
  // the browser preview has no app: the same sample the Quest Tracker's stand-in uses (?quests=none: no game)
  if (!wv) {
    const Q = new URLSearchParams(location.search);
    // ?quests=extra: also Act 6 (one blade), Act.8's four residents and Let's Trade!, as in the Tracker's stand-in
    const extra = Q.get('quests') === 'extra'
      ? [{ id: 907, step: 0, counters: [0, 0, 0, 0, 0] }, { id: 909, step: 0, counters: [0, 0, 0, 0, 0] }, { id: 7, step: 0, counters: [0, 0, 0, 0, 0] }] : [];
    progress = Q.get('quests') === 'none' ? null : { active: [{ id: 902, step: 0, counters: [0, 0, 0, 0, 0] }, { id: 818, step: 0, counters: [7, 13, 5, 0, 0] }, { id: 312, step: 2, counters: [0, 0, 0, 0, 0] }, { id: 904, step: 0, counters: [0, 0, 0, 0, 0] }, ...extra],
      completed: [1, 2, 3, 4, 5, 901], flags: extra.length ? [82, 84, 124] : [82, 84], zone: Number(Q.get('zone')) || 50101 };
  }
})();
