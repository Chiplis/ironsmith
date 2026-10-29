import { readdir, readFile, stat } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const ui = fileURLToPath(new URL('../', import.meta.url));
const output = path.resolve(process.argv[2] || path.join(ui, 'dist'));
const publicRoot = path.join(ui, 'public');
async function files(directory, prefix = '') {
  const entries = await readdir(directory, { withFileTypes: true });
  const result = [];
  for (const entry of entries) {
    if (entry.name.startsWith('.')) continue;
    const name = path.join(prefix, entry.name);
    if (entry.isDirectory()) result.push(...await files(path.join(directory, entry.name), name));
    else if (entry.isFile()) result.push(name);
  }
  return result;
}

// Public assets are loaded at runtime and are not checked by Vite's module
// graph. A failed build can leave dist partly emptied but still containing an
// old index.html and app bundle, so checking those alone is not sufficient.
const required = await files(publicRoot);
required.push('index.html');
const html = await readFile(path.join(output, 'index.html'), 'utf8');
for (const match of html.matchAll(/(?:src|href)=["']\.\/(assets\/[^"']+)["']/g)) required.push(match[1]);
const missing = [];
for (const file of required) {
  const info = await stat(path.join(output, file)).catch(() => null);
  const emptySource = info?.size === 0
    && (await stat(path.join(publicRoot, file)).catch(() => null))?.size === 0;
  if (!info?.isFile() || (info.size === 0 && !emptySource)) missing.push(file);
}
if (missing.length) {
  console.error(`Incomplete deployment: ${missing.length} missing or empty files in ${output}`);
  console.error(missing.slice(0, 20).join('\n'));
  console.error('Do not publish this directory. Run a successful frontend build and copy the entire dist directory.');
  process.exitCode = 1;
} else {
  console.log(`Deployment assets verified: ${required.length} files in ${output}`);
}
