// Download a feedback attachment from the service's store and open it with the private key:
//   node feedback-worker/tools/open-files.mjs <id> [--key <private key file>] [--out <folder>]
// <id> is the one in the issue ("Diagnostics attached ... `id`"). Wrangler must be signed in to the Cloudflare account
// (npx wrangler login). The files go to <repo>\.secrets\feedback-files\<id>\ unless --out says otherwise (.secrets\ is
// git-ignored: these are other people's logs and settings).
import { execFileSync } from 'node:child_process';
import { readFileSync, writeFileSync, mkdirSync, existsSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { openEnvelope, safeName } from './envelope.mjs';

const here = dirname(fileURLToPath(import.meta.url));
const worker = resolve(here, '..');
const repo = resolve(worker, '..');
const args = process.argv.slice(2);
const opt = (name, dflt) => { const i = args.indexOf(name); return i >= 0 ? args.splice(i, 2)[1] : dflt; };
// the main checkout's .secrets\ (this script may run from a worktree under .claude\worktrees\)
const mainRepo = repo.replace(/[\\/]\.claude[\\/]worktrees[\\/][^\\/]+$/, '');
const keyFile = opt('--key', join(mainRepo, '.secrets', 'feedback-files-private.pem'));
const id = (args[0] || '').trim().replace(/^`|`$/g, '');
if (!/^(test-)?[a-z0-9]{16}$/.test(id)) { console.error('Give the attachment id from the issue, like: node open-files.mjs k3m9x2p7q4r8t6w2'); process.exit(1); }
const out = resolve(opt('--out', join(mainRepo, '.secrets', 'feedback-files', id)));
if (!existsSync(keyFile)) { console.error(`No private key at ${keyFile}. Give it with --key.`); process.exit(1); }

// wrangler.toml is git-ignored and kept in the main checkout only: a worktree uses that one
const config = [join(worker, 'wrangler.toml'), join(mainRepo, 'feedback-worker', 'wrangler.toml')].find((p) => existsSync(p));
if (!config) { console.error(`No wrangler.toml in ${worker} or the main checkout's feedback-worker\\ (see wrangler.example.toml).`); process.exit(1); }
let envelope;
try {
  envelope = execFileSync(process.execPath, [join(worker, 'node_modules', 'wrangler', 'bin', 'wrangler.js'), 'kv', 'key', 'get', `files/${id}`, '--binding', 'FEEDBACK_FILES', '--remote', '--text', '--config', config],
    { cwd: worker, encoding: 'utf8', maxBuffer: 64 * 1024 * 1024, stdio: ['ignore', 'pipe', 'pipe'] });
} catch (e) {
  console.error(`Could not download ${id}: ${String(e.stderr || e.message).trim().split('\n').slice(-3).join(' ')}`);
  process.exit(1);
}
if (!envelope.trim().startsWith('{')) { console.error(`Nothing is stored under ${id}: it may have expired (files are kept 180 days, test sends an hour).`); process.exit(1); }

let opened;
try { opened = openEnvelope(envelope.trim(), readFileSync(keyFile, 'utf8')); }
catch (e) { console.error(`Could not open ${id} with ${keyFile}: ${e.message}`); process.exit(1); }

mkdirSync(out, { recursive: true });
for (const f of opened.files || []) {
  const p = join(out, safeName(f.name));
  mkdirSync(dirname(p), { recursive: true });
  writeFileSync(p, String(f.text ?? ''));
}
writeFileSync(join(out, '_sent.txt'), `Attachment ${id}\r\nSent: ${opened.made || '?'}\r\nCharacter names: ${opened.includeNames ? 'included' : 'left out'}\r\nFiles: ${(opened.files || []).map((f) => f.name).join(', ')}\r\n`);
console.log(`${(opened.files || []).length} files in ${out}`);
