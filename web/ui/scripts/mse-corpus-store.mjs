import { readFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import { MsePackageStore } from '../src/lib/mse/packages.mjs';
import upstream from '../src/lib/mse/upstream.mjs';

export async function corpusStore(directory) {
  const tree = JSON.parse(await readFile(resolve(directory, 'upstream-tree.json'), 'utf8'));
  if (tree.sha !== upstream.revision || tree.truncated) throw new Error('Corpus file index must be complete and match the pinned upstream revision');
  // The index describes assets available from the pinned pack, including files
  // not yet downloaded by the text-only sync. Pixel rendering still loads them.
  const files = tree.tree.filter(entry => entry.type === 'blob' && entry.path.startsWith('data/')).map(entry => entry.path.slice(5));
  return new MsePackageStore({ files, readText: path => readFile(resolve(directory, 'data', path.key), 'utf8') });
}
