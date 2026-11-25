// Makes the key pair for feedback attachments (run once; again only to replace a lost or leaked key):
//   node feedback-worker/tools/make-files-key.mjs <private key file>
// The private key (RSA 3072, PKCS#8 PEM) is written to the file given, never printed; it must not exist yet. The public
// key is printed: it goes into app.js (FEEDBACK_FILES_KEY). The app encrypts attachments with it; only the private key
// opens them (open-files.mjs). A new pair means files sent with the old public key need the old private key.
import { generateKeyPairSync } from 'node:crypto';
import { writeFileSync, existsSync } from 'node:fs';

const out = process.argv[2];
if (!out) { console.error('Give the file to write the private key to.'); process.exit(1); }
if (existsSync(out)) { console.error(`${out} already exists: not replacing a key.`); process.exit(1); }
const { publicKey, privateKey } = generateKeyPairSync('rsa', {
  modulusLength: 3072,
  publicKeyEncoding: { type: 'spki', format: 'der' },
  privateKeyEncoding: { type: 'pkcs8', format: 'pem' },
});
writeFileSync(out, privateKey, { flag: 'wx' });
console.log(`Private key written to ${out}`);
console.log(`Public key (SPKI, base64) for app.js:\n${publicKey.toString('base64')}`);
