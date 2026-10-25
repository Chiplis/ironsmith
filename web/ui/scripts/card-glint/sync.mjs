import {createHash} from 'node:crypto';
import {execFileSync} from 'node:child_process';
import {cp, mkdir, readFile, rm, writeFile, access, readdir, chmod} from 'node:fs/promises';
import path from 'node:path';
import {fileURLToPath} from 'node:url';

const ui = fileURLToPath(new URL('../../', import.meta.url));
const cache = path.join(ui, 'node_modules/.cache/card-glint');
const source = path.join(cache, 'source');
const output = path.join(ui, 'public/card-glint');
// Windows only resolves npm through its npm.cmd shim, which needs a shell.
const windows = process.platform === 'win32';
const run = (command, args, cwd = source, env = process.env) => (windows && command === 'npm'
  ? execFileSync('npm.cmd', args, {cwd, env, stdio: 'inherit', shell: true})
  : execFileSync(command, args, {cwd, env, stdio: 'inherit'}));
const capture = (command, args) => execFileSync(command, args, {cwd: source, encoding: 'utf8'}).trim();
const exists = async file => access(file).then(() => true, () => false);
await mkdir(cache, {recursive: true});
// Always contact upstream. A failed refresh must fail the build, even with a warm cache.
if (!await exists(path.join(source, '.git'))) {
  run('git', ['clone', '--depth', '1', '--branch', 'main', 'https://github.com/vishvananda/card-glint.git', source], cache);
} else {
  run('git', ['fetch', '--depth', '1', 'origin', 'main']);
  run('git', ['checkout', '--detach', 'FETCH_HEAD']);
}
const sourceRevision = capture('git', ['rev-parse', 'HEAD']);
const buildHash = createHash('sha256').update(await readFile(fileURLToPath(import.meta.url)))
  .update(await readFile(path.join(ui, 'scripts/card-glint/bundle.mjs')))
  .update(await readFile(path.join(ui, 'scripts/card-glint/material-worker-client.mjs')))
  .update(await readFile(path.join(ui, 'scripts/card-glint/material-worker.mjs'))).digest('hex');
// Host patches change browser assets even when upstream's revision is fixed.
// Keep their URLs distinct so a running/cached app cannot load a mixed build.
const revision = `${sourceRevision}-${buildHash.slice(0,12)}`;
const destination = path.join(output, revision);
const completed = await readFile(path.join(destination, 'complete.json'), 'utf8').then(JSON.parse, () => null);
if (completed?.buildHash !== buildHash) {
  const lock = await readFile(path.join(source, 'Cargo.lock'), 'utf8');
  // A Windows checkout may convert the lockfile to CRLF line endings.
  const version = lock.match(/name = "wasm-bindgen"\r?\nversion = "([^"]+)"/)?.[1];
  if (!version) throw new Error('Cannot determine card-glint wasm-bindgen version');
  const toolRoot = path.join(cache, 'tools', version);
  const bindgen = path.join(toolRoot, windows ? 'bin/wasm-bindgen.exe' : 'bin/wasm-bindgen');
  if (!await exists(bindgen)) run('cargo', ['install', 'wasm-bindgen-cli', '--version', version, '--locked', '--root', toolRoot]);
  run('npm', ['ci', '--no-audit', '--no-fund']);
  const env = {...process.env, PATH: `${path.join(toolRoot, 'bin')}${path.delimiter}${process.env.PATH}`};
  if (process.platform === 'darwin') {
    // Rust's macOS wasm linker needs the toolchain's LLVM library at runtime.
    // A wrapper sets it after npm's shell (macOS strips DYLD variables there).
    const sysroot = capture('rustc', ['--print', 'sysroot']);
    const host = capture('rustc', ['-vV']).match(/^host: (.+)$/m)[1];
    const quote = value => "'" + value.replaceAll("'", "'\\''") + "'";
    const linker = path.join(cache, 'wasm-linker.sh');
    await writeFile(linker, `#!/bin/sh\nexport DYLD_LIBRARY_PATH=${quote(path.join(sysroot, 'lib'))}\nexec ${quote(path.join(sysroot, 'lib/rustlib', host, 'bin/rust-lld'))} "$@"\n`);
    await chmod(linker, 0o755);
    env.CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_LINKER = linker;
  }
  run('npm', ['run', 'build:wasm'], source, env);
  run(process.execPath, [path.join(ui, 'scripts/card-glint/bundle.mjs')], source, env);
  await rm(destination, {recursive: true, force: true});
  await cp(path.join(source, 'dist/lib'), destination, {recursive: true});
  for (const file of ['browser.js', 'magic.js', 'magic-presentation.js', 'magic.css', 'renderer_bg.wasm', 'pixels_bg.wasm']) {
    if (!await exists(path.join(destination, file))) throw new Error(`card-glint build missing ${file}`);
  }
  for (const file of ['LICENSE-MIT', 'LICENSE-APACHE', 'NOTICE.md']) await cp(path.join(source, file), path.join(destination, file));
  await cp(path.join(source, 'games/magic/NOTICE.md'), path.join(destination, 'MAGIC-NOTICE.md'));
  await writeFile(path.join(destination, 'complete.json'), JSON.stringify({revision,sourceRevision,buildHash}));
}
for (const entry of await readdir(output)) {
  if (/^[a-f0-9]{40}(?:-[a-f0-9]{12})?$/.test(entry) && entry !== revision) await rm(path.join(output, entry), {recursive:true, force:true});
}
// Vite embeds this revision in the app, and copies its matching JS/WASM directory.
await writeFile(path.join(cache, 'current.json'), JSON.stringify({revision}));
console.log(`[card-glint] Ready: ${revision}`);
