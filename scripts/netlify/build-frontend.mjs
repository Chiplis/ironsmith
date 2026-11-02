// Deployment snapshot: build the UI using the synced, already-generated assets.
import { cp, mkdir, readFile, rm, writeFile } from 'node:fs/promises';
import { execFileSync } from 'node:child_process';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
const ui = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const home = path.resolve(ui, '../../..');
const run = (args) => execFileSync(process.execPath, args, { cwd: ui, stdio: 'inherit' });
const manifest = JSON.parse(await readFile(path.join(ui, 'card-glint-current.json'), 'utf8'));
await readFile(path.join(ui, 'public/card-glint', manifest.revision, 'complete.json'));
await mkdir(path.join(ui, 'node_modules/.cache/card-glint'), { recursive: true });
await writeFile(path.join(ui, 'node_modules/.cache/card-glint/current.json'), JSON.stringify(manifest));
run(['scripts/build-random-card-pool.mjs']);
run(['node_modules/vite/bin/vite.js', 'build']);
run(['scripts/verify-deployment.mjs']);
// Assemble the whole home website only after the frontend passes verification.
const publish = path.join(home, '.netlify/publish');
await rm(publish, { recursive: true, force: true });
await cp(path.join(home, 'public'), publish, {
  recursive: true,
  filter: (entry) => entry !== path.join(home, 'public/ironsmith'),
});
await cp(path.join(ui, 'dist'), path.join(publish, 'ironsmith'), { recursive: true });
console.log(`Complete home website ready in ${publish}`);
