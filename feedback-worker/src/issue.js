// What a player sent, checked, and the GitHub issue made from it.

export const KINDS = {
  bug: { word: 'Bug', label: 'bug' },
  idea: { word: 'Idea', label: 'enhancement' },
  question: { word: 'Question', label: 'question' },
};


/** Plain text: no control characters except new lines and tabs, Windows line ends made plain, trimmed. */
export function clean(v, max) {
  if (typeof v !== 'string') return '';
  return v.replace(/\r\n?/g, '\n').replace(/[\u0000-\u0008\u000b-\u001f\u007f]/g, '').trim().slice(0, max);
}
/** Keep the text as written, but never let it ping someone (@name) or start HTML (<tag>). */
export function quiet(text) {
  return text.replace(/@(?=[A-Za-z0-9_-])/g, '@​').replace(/</g, '&lt;');
}

export function check(input) {
  const kind = typeof input.kind === 'string' && KINDS[input.kind] ? input.kind : null;
  if (!kind) return { error: 'Pick what kind of feedback it is: a bug, an idea or a question.' };
  const message = clean(input.message, 4000);
  // sections: feedback on one page or tool each ({ page: its name in the app, text })
  const sections = [];
  if (Array.isArray(input.sections)) {
    if (input.sections.length > 20) return { error: 'That is more pages and tools than the app has.' };
    for (const s of input.sections) {
      const page = clean(s && s.page, 40).replace(/[#*_`[\]<>|]/g, '').replace(/\s+/g, ' ').trim();
      const text = clean(s && s.text, 4000);
      if (!page) return { error: 'Pick a page or tool for each section.' };
      if (!text) return { error: `Write something about ${page}, or remove it.` };
      sections.push({ page, text });
    }
  }
  if (message.length + sections.reduce((n, s) => n + s.text.length, 0) < 10) return { error: 'Write a little more: at least 10 characters.' };
  const about = {};
  if (input.about && typeof input.about === 'object') {
    for (const k of ['version', 'commit', 'built', 'windows', 'webview2']) {
      const v = clean(input.about[k], 80).replace(/\n/g, ' ');
      if (v) about[k] = v;
    }
  }
  return {
    kind,
    title: clean(input.title, 100).replace(/\n/g, ' '),
    message,
    sections,
    contact: clean(input.contact, 60).replace(/\n/g, ' '),
    about,
    test: input.test === true,
    honeypot: typeof input.website === 'string' && input.website.trim() !== '',
  };
}

export function makeIssue(f) {
  const k = KINDS[f.kind];
  const first = f.sections[0];
  let title = f.title || (f.message ? f.message.split('\n')[0] : `${first.page}: ${first.text.split('\n')[0]}`);
  if (title.length > 80) title = title.slice(0, 77).trimEnd() + '…';
  const lines = [];
  if (f.message) lines.push(quiet(f.message), '');
  for (const s of f.sections) lines.push(`### ${s.page}`, '', quiet(s.text), '');
  if (f.attachment) {
    const a = f.attachment, kb = a.bytes ? `, ${Math.max(1, Math.round(a.bytes / 1024))} KB` : '';
    lines.push(`📎 **Diagnostics attached**, encrypted (only BABEL's developer can open them): \`${a.id}\``
      + (a.files ? ` (${a.files} file${a.files === 1 ? '' : 's'}${kb})` : ''), '');
  }
  lines.push('---', `*Sent from BABEL's feedback form (${k.word.toLowerCase()}).*`);
  if (f.contact) lines.push('', `**Contact:** ${quiet(f.contact)}`);
  const a = f.about;
  const rows = [
    ['Version', [a.version, a.commit && `commit ${a.commit}`, a.built && `built ${a.built}`].filter(Boolean).join(', ')],
    ['Windows', a.windows],
    ['WebView2', a.webview2],
  ].filter((r) => r[1]);
  if (rows.length) lines.push('', '| | |', '|---|---|', ...rows.map(([n, v]) => `| ${n} | ${quiet(v).replace(/\|/g, '\\|')} |`));
  return { title: `[${k.word}] ${title}`, body: lines.join('\n'), labels: ['feedback', k.label] };
}
