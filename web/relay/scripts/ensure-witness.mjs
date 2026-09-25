#!/usr/bin/env node
// Makes sure the deployed relay has a tournament witness key and pins its
// public key into the production UI build. Idempotent; runs before `pnpm build`.
//
//   1. GET <relay>/witness/key.
//   2. 404: the deployed relay predates the witness, so deploy it once.
//   3. 503: no signing key yet. Generate one (or reuse the local backup),
//      upload it with `wrangler secret put`, and keep the backup offline at
//      ~/.config/ironsmith/witness-signing-key.json (override with
//      IRONSMITH_WITNESS_KEY_FILE). Worker secrets cannot be read back, so
//      that file is the only copy.
//   4. Write VITE_WITNESS_PUBLIC_KEYS to web/ui/.env.production.local
//      (git-ignored), which Vite loads for production builds.
//
// Once a key exists, builds only need network access to the relay: no
// wrangler login and no private key. IRONSMITH_SKIP_WITNESS=1 skips all of it;
// IRONSMITH_WITNESS_DRY_RUN=1 prints the wrangler commands instead of running them.
import { execFileSync } from 'node:child_process';
import { existsSync, mkdirSync, readFileSync, writeFileSync, chmodSync } from 'node:fs';
import { homedir } from 'node:os';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { webcrypto } from 'node:crypto';

const relayDir = join(dirname(fileURLToPath(import.meta.url)), '..');
const uiDir = join(relayDir, '..', 'ui');
const keyFile = process.env.IRONSMITH_WITNESS_KEY_FILE || join(homedir(), '.config', 'ironsmith', 'witness-signing-key.json');
const envFile = join(uiDir, '.env.production.local');
const log = message => console.log(`[witness] ${message}`);
const fail = message => { console.error(`[witness] ${message}`); process.exit(1); };

function readEnvValue(file, name) {
  if (!existsSync(file)) return '';
  const line = readFileSync(file, 'utf8').split('\n').find(entry => entry.startsWith(`${name}=`));
  return line ? line.slice(name.length + 1).trim() : '';
}

function relayConfig() {
  const url = (process.env.VITE_LOBBY_RELAY_URL || readEnvValue(join(uiDir, '.env.production'), 'VITE_LOBBY_RELAY_URL')).replace(/\/$/, '');
  if (!url) fail('VITE_LOBBY_RELAY_URL is not set (web/ui/.env.production)');
  // The relay rejects requests without an allowlisted Origin.
  const wrangler = readFileSync(join(relayDir, 'wrangler.jsonc'), 'utf8').replace(/^\s*\/\/.*$/gm, '');
  const origins = String(JSON.parse(wrangler).vars?.ALLOWED_ORIGINS || '').split(',').map(origin => origin.trim());
  return { url, origin: origins.find(origin => origin.startsWith('https://')) || origins[0] };
}

function wrangler(args, input) {
  if (process.env.IRONSMITH_WITNESS_DRY_RUN === '1') { log(`dry run: would run wrangler ${args.join(' ')}`); return; }
  const local = join(relayDir, 'node_modules', '.bin', 'wrangler');
  const [command, prefix] = existsSync(local) ? [local, []] : ['npx', ['--yes', 'wrangler']];
  return execFileSync(command, [...prefix, ...args], { cwd: relayDir, input, stdio: [input ? 'pipe' : 'inherit', 'inherit', 'inherit'] });
}

async function publicKeyFromJwk(jwk) {
  const key = await webcrypto.subtle.importKey('jwk', { kty: jwk.kty, crv: jwk.crv, x: jwk.x, y: jwk.y, ext: true },
    { name: 'ECDSA', namedCurve: 'P-256' }, true, ['verify']);
  return Buffer.from(await webcrypto.subtle.exportKey('raw', key)).toString('hex');
}

async function localKey({ create }) {
  if (existsSync(keyFile)) return JSON.parse(readFileSync(keyFile, 'utf8'));
  if (!create) return null;
  const { privateKey } = await webcrypto.subtle.generateKey({ name: 'ECDSA', namedCurve: 'P-256' }, true, ['sign', 'verify']);
  const jwk = await webcrypto.subtle.exportKey('jwk', privateKey);
  mkdirSync(dirname(keyFile), { recursive: true, mode: 0o700 });
  writeFileSync(keyFile, JSON.stringify(jwk), { mode: 0o600 });
  chmodSync(keyFile, 0o600);
  log(`generated a new witness signing key; backup at ${keyFile} (keep it private, never commit it)`);
  return jwk;
}

async function fetchKey({ url, origin }) {
  const response = await fetch(`${url}/witness/key`, { headers: { Origin: origin }, signal: AbortSignal.timeout(15_000) });
  if (response.ok) return { status: 200, publicKey: String((await response.json()).publicKey || '').toLowerCase() };
  return { status: response.status };
}

async function waitForKey(relay, label) {
  for (let attempt = 0; attempt < 20; attempt++) {
    const result = await fetchKey(relay);
    if (result.status === 200) return result.publicKey;
    await new Promise(resolve => setTimeout(resolve, 1500));
  }
  fail(`the relay still does not serve a witness key after ${label}`);
}

function pin(publicKey) {
  const lines = existsSync(envFile) ? readFileSync(envFile, 'utf8').split('\n').filter(line => line && !line.startsWith('VITE_WITNESS_PUBLIC_KEYS=')) : [];
  writeFileSync(envFile, [...lines, `VITE_WITNESS_PUBLIC_KEYS=${publicKey}`, ''].join('\n'));
  log(`pinned witness key ${publicKey.slice(0, 16)}… into ${envFile}`);
}

if (process.env.IRONSMITH_SKIP_WITNESS === '1') { log('skipped (IRONSMITH_SKIP_WITNESS=1)'); process.exit(0); }
const relay = relayConfig();
let result;
try { result = await fetchKey(relay); }
catch (error) { fail(`could not reach ${relay.url}: ${error.message}`); }

if (result.status === 404) {
  log('the deployed relay predates the tournament witness; deploying it once');
  wrangler(['deploy']);
  result = await fetchKey(relay);
}
if (result.status === 503) {
  const jwk = await localKey({ create: true });
  log('uploading the witness signing key as the WITNESS_SIGNING_KEY Worker secret');
  wrangler(['secret', 'put', 'WITNESS_SIGNING_KEY'], JSON.stringify(jwk));
  result = { status: 200, publicKey: await waitForKey(relay, 'uploading the secret') };
}
if (result.status !== 200) fail(`unexpected ${result.status} from ${relay.url}/witness/key`);

// A backup that disagrees with the deployed key means one of them was
// replaced; builds must not silently pin a key the organizer does not hold.
const backup = await localKey({ create: false });
if (backup && await publicKeyFromJwk(backup) !== result.publicKey) {
  fail(`the relay's witness key differs from the backup at ${keyFile}; resolve this before building`);
}
pin(result.publicKey);
