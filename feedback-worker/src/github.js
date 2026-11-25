// Signing in to GitHub as the "BABEL feedback" app, and calling its API.

const enc = new TextEncoder();
const b64url = (bytes) => btoa(String.fromCharCode(...new Uint8Array(bytes))).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '');

function derLength(n) {
  if (n < 128) return [n];
  const out = [];
  while (n > 0) { out.unshift(n & 255); n >>= 8; }
  return [0x80 | out.length, ...out];
}
/** GitHub's key file is PKCS#1 ("BEGIN RSA PRIVATE KEY"); Web Crypto reads PKCS#8, which is the same key wrapped. */
export function pkcs8Of(pem) {
  const der = Uint8Array.from(atob(pem.replace(/-----[^-]+-----/g, '').replace(/\s+/g, '')), (c) => c.charCodeAt(0));
  if (!/BEGIN RSA PRIVATE KEY/.test(pem)) return der;   // already PKCS#8
  const head = [0x02, 0x01, 0x00,                                                         // version 0
    0x30, 0x0d, 0x06, 0x09, 0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x01, 0x05, 0x00,  // rsaEncryption, NULL
    0x04, ...derLength(der.length)];                                                      // OCTET STRING { PKCS#1 }
  const body = head.length + der.length;
  const outer = [0x30, ...derLength(body)];
  const out = new Uint8Array(outer.length + body);
  out.set(outer, 0); out.set(head, outer.length); out.set(der, outer.length + head.length);
  return out;
}

export async function appJwt(env) {
  if (!env.GITHUB_APP_KEY) throw new Error('The GITHUB_APP_KEY secret is not set.');
  const key = await crypto.subtle.importKey('pkcs8', pkcs8Of(env.GITHUB_APP_KEY), { name: 'RSASSA-PKCS1-v1_5', hash: 'SHA-256' }, false, ['sign']);
  const now = Math.floor(Date.now() / 1000);
  const part = (o) => b64url(enc.encode(JSON.stringify(o)));
  const unsigned = `${part({ alg: 'RS256', typ: 'JWT' })}.${part({ iat: now - 60, exp: now + 540, iss: String(env.APP_ID) })}`;
  const sig = await crypto.subtle.sign('RSASSA-PKCS1-v1_5', key, enc.encode(unsigned));
  return `${unsigned}.${b64url(sig)}`;
}

export function gh(url, token, init = {}) {
  return fetch(url, {
    ...init,
    headers: {
      'Accept': 'application/vnd.github+json',
      'Authorization': `Bearer ${token}`,
      'User-Agent': 'babel-feedback',
      'X-GitHub-Api-Version': '2022-11-28',
      ...(init.body ? { 'Content-Type': 'application/json' } : {}),
    },
  });
}

// An installation token lasts an hour; keep it while this Worker instance lives, with five minutes to spare.
let cached = { token: '', until: 0 };
export async function installationToken(env) {
  if (cached.token && Date.now() < cached.until) return cached.token;
  const res = await gh(`https://api.github.com/app/installations/${env.INSTALLATION_ID}/access_tokens`, await appJwt(env), { method: 'POST' });
  if (!res.ok) throw new Error(`GitHub said ${res.status} to the app's sign-in: ${(await res.text()).slice(0, 300)}`);
  const t = await res.json();
  cached = { token: t.token, until: Date.parse(t.expires_at) - 5 * 60 * 1000 };
  return t.token;
}
