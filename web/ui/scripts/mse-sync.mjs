// Fetch the pinned corpus for compatibility development. --assets additionally
// downloads the complete data/font pack; --fonts fetches font binaries without
// the image assets. No executable from upstream is run.
import { readFile, writeFile, mkdir } from 'node:fs/promises';
import { resolve, dirname, join } from 'node:path';
import { createHash } from 'node:crypto';
import { fileURLToPath } from 'node:url';
import lock from '../src/lib/mse/upstream.mjs';

const args = process.argv.slice(2);
const destination = args.find(arg => !arg.startsWith('--'));
if (!destination || args.some(arg => arg.startsWith('--') && !['--assets','--fonts'].includes(arg))) {
  throw new Error('Usage: node scripts/mse-sync.mjs DESTINATION [--assets] [--fonts]');
}
const root = resolve(destination);
const treeFile = join(root, 'upstream-tree.json');
await mkdir(root, { recursive: true });
async function fetchBytes(url) {
  let failure;
  for (let attempt = 0; attempt < 3; attempt++) {
    try {
      const response = await fetch(url, { signal: AbortSignal.timeout(45000) });
      if (!response.ok) throw new Error(`HTTP ${response.status}: ${url}`);
      return Buffer.from(await response.arrayBuffer());
    } catch (error) { failure = error; }
  }
  throw failure;
}
let tree;
try { tree = JSON.parse(await readFile(treeFile, 'utf8')); } catch { /* first sync */ }
if (tree?.sha !== lock.revision || tree.truncated) {
  tree = JSON.parse(await fetchBytes(`https://api.github.com/repos/${lock.repository}/git/trees/${lock.revision}?recursive=1`));
  if (tree.truncated || tree.sha !== lock.revision) throw new Error('Upstream tree is incomplete or not pinned');
  await writeFile(treeFile, JSON.stringify(tree));
}
const isText = path => !path.split('/').at(-1).includes('.') || /\.(txt|mse-symbol|aff|dic)$/i.test(path);
const isFont = path => /\.(ttf|otf|ttc|woff2?)$/i.test(path);
const files = tree.tree.filter(entry => entry.type === 'blob'
  && (entry.path.startsWith('data/') || entry.path.startsWith('Magic - Fonts/'))
  && (args.includes('--assets') || isText(entry.path) || (args.includes('--fonts') && isFont(entry.path))));
const hash = bytes => createHash('sha1').update(`blob ${bytes.length}\0`).update(bytes).digest('hex');
let cursor = 0, cached = 0, fetched = 0;
await Promise.all(Array.from({ length: 8 }, async () => {
  while (cursor < files.length) {
    const entry = files[cursor++];
    const path = resolve(root, entry.path);
    if (!path.startsWith(root + '/')) throw new Error(`Invalid upstream path ${entry.path}`);
    let bytes;
    try { bytes = await readFile(path); } catch { /* cache miss */ }
    if (bytes && hash(bytes) === entry.sha) { cached++; continue; }
    const url = `https://raw.githubusercontent.com/${lock.repository}/${lock.revision}/${entry.path.split('/').map(encodeURIComponent).join('/')}`;
    bytes = await fetchBytes(url);
    if (hash(bytes) !== entry.sha) throw new Error(`Upstream hash mismatch: ${entry.path}`);
    await mkdir(dirname(path), { recursive: true });
    await writeFile(path, bytes);
    fetched++;
    if (fetched % 100 === 0) console.log(`${fetched} downloaded, ${cached} cached, ${files.length} total`);
  }
}));
await writeFile(join(root, 'source.json'), JSON.stringify({ ...lock, assets: args.includes('--assets'), fonts: args.includes('--assets') || args.includes('--fonts'), files: files.length }, null, 2) + '\n');
console.log(JSON.stringify({ root, fetched, cached, files: files.length, script: fileURLToPath(import.meta.url) }));
