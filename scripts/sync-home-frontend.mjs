#!/usr/bin/env node
// Copy source + existing generated assets; no frontend or Rust build, commit, or push.
import { execFileSync } from 'node:child_process';
import { access, copyFile, mkdir, readFile, rm, symlink, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { homedir } from 'node:os';
import { fileURLToPath } from 'node:url';
const repo = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const source = path.join(repo, 'web/ui');
const home = path.resolve(process.argv[2] || path.join(homedir(), 'home'));
const target = path.join(home, 'ironsmith/web/ui');
await access(path.join(home, '.git'));
const manifest = JSON.parse(await readFile(path.join(source, 'node_modules/.cache/card-glint/current.json'), 'utf8'));
await access(path.join(source, 'public/card-glint', manifest.revision, 'complete.json'));
for (const file of ['engine_bg.wasm', 'compiler_bg.wasm', 'verifier_bg.wasm']) {
  await access(path.join(repo, 'web/wasm_demo/pkg', file));
}
const sync = async (from, to) => {
  await mkdir(to, { recursive: true });
  execFileSync('rsync', ['-aL', '--delete', '--exclude=.DS_Store', '--exclude=__pycache__', `${from}/`, `${to}/`], { stdio: 'inherit' });
};
await mkdir(target, { recursive: true });
for (const dir of ['src', 'scripts']) await sync(path.join(source, dir), path.join(target, dir));
for (const file of ['index.html', 'vite.config.js', 'jsconfig.json', 'pnpm-lock.yaml', 'pnpm-workspace.yaml']) {
  await copyFile(path.join(source, file), path.join(target, file));
}
const pkg = JSON.parse(await readFile(path.join(source, 'package.json'), 'utf8'));
// Keep the dependency graph/lockfile, but avoid repository-only prebuild hooks.
pkg.packageManager = 'pnpm@10.29.2';
pkg.scripts = { build: 'node scripts/netlify-build.mjs' };
await writeFile(path.join(target, 'package.json'), JSON.stringify(pkg, null, 2) + '\n');
await copyFile(path.join(repo, 'scripts/netlify/build-frontend.mjs'), path.join(target, 'scripts/netlify-build.mjs'));
await writeFile(path.join(target, 'card-glint-current.json'), JSON.stringify(manifest) + '\n');
// Only copy the two public browser settings; never copy private/local env files.
const allowed = new Set(['VITE_LOBBY_RELAY_URL', 'VITE_WITNESS_PUBLIC_KEYS']);
const settings = new Map();
for (const file of ['.env.production', '.env.production.local']) {
  const text = await readFile(path.join(source, file), 'utf8').catch(error => { if (error.code === 'ENOENT') return ''; throw error; });
  for (const line of text.split('\n')) {
    const key = line.split('=', 1)[0];
    if (allowed.has(key)) settings.set(key, line);
  }
}
for (const key of allowed) if (!settings.has(key)) throw new Error(`Missing ${key}; prepare the local production assets first.`);
await writeFile(path.join(target, '.env.production'), [...settings.values()].join('\n') + '\n');
// Store assets once, at their existing tracked location in the home repository.
await sync(path.join(source, 'public'), path.join(home, 'public/ironsmith'));
await mkdir(path.join(home, 'ironsmith/web/wasm_demo'), { recursive: true });
for (const [link, destination] of [
  [path.join(target, 'public'), '../../../public/ironsmith'],
  [path.join(home, 'ironsmith/web/wasm_demo/pkg'), '../../../public/ironsmith/pkg'],
]) {
  await rm(link, { force: true });
  await symlink(destination, link);
}
await writeFile(path.join(home, 'ironsmith/.gitignore'), 'node_modules/\ndist/\n*.local\n.DS_Store\n');
console.log(`Frontend source and assets synced to ${home}. Commit and push that repository to deploy.`);
