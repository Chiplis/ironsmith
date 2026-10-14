import {createHash} from 'node:crypto';
import {createWriteStream} from 'node:fs';
import {readFile, writeFile, mkdir, mkdtemp, rm, copyFile, lstat} from 'node:fs/promises';
import {join, dirname, resolve} from 'node:path';
import {pipeline} from 'node:stream/promises';
import {execFile} from 'node:child_process';
import {promisify} from 'node:util';
import {readFontMetadata} from '../../src/lib/mse/font-metadata.mjs';
import {mseResourceFontPaths} from '../../src/lib/mse/font-resources.mjs';

const exec = promisify(execFile);
export const blobHash = bytes => createHash('sha1').update(`blob ${bytes.length}\0`).update(bytes).digest('hex');

// Keep every template and shared package. Only remove known desktop-only data.
export function runtimeFile(path) {
  return (path.startsWith('data/') || path.startsWith('Magic - Fonts/') || mseResourceFontPaths([path]).length>0)
    && !path.startsWith('data/dictionaries/')
    && !path.split('/')[1]?.endsWith('.mse-updater')
    && !/\.(psd|exe)$/i.test(path);
}

function validateTree(tree, lock) {
  if (tree.sha !== lock.revision || tree.truncated || !Array.isArray(tree.tree)) {
    throw new Error('Expected the complete pinned MSE tree');
  }
  for (const entry of tree.tree) {
    if (typeof entry.path !== 'string' || /[\\\0]/.test(entry.path)
      || entry.path.split('/').some(part => !part || part === '.' || part === '..')) {
      throw new Error('Invalid MSE asset path');
    }
  }
}

async function verified(root, entry) {
  const path = join(root, entry.path);
  try {
    if (!(await lstat(path)).isFile()) return false;
    return blobHash(await readFile(path)) === entry.sha;
  } catch { return false; }
}

export async function writeBrowserIndex(destination, tree, fontMetadataReader=readFontMetadata) {
  const packages=new Map(),fonts=[];
  for(const entry of tree.tree.filter(e=>e.type==='blob'&&runtimeFile(e.path))){
    const path=entry.path;
    if(mseResourceFontPaths([path]).length){
      const metadata=fontMetadataReader(await readFile(join(destination,path)));
      fonts.push({path,names:[...new Set([...metadata.families,metadata.fullName,metadata.postscriptName].filter(Boolean))]});
    }else if(path.startsWith('data/')&&(!path.split('/').at(-1).includes('.')||/\.(txt|mse-symbol)$/i.test(path))){
      const [,pkg,...rest]=path.split('/');
      if(!packages.has(pkg))packages.set(pkg,{});
      packages.get(pkg)[rest.join('/')]=await readFile(join(destination,path),'utf8');
    }
  }
  await mkdir(join(destination,'packages'),{recursive:true});
  for(const [pkg,files] of packages)await writeFile(join(destination,'packages',`${pkg}.json`),JSON.stringify(files));
  await writeFile(join(destination,'runtime-index.json'),JSON.stringify({revision:tree.sha,fonts,textPackages:[...packages.keys()]}));
}

export async function installCorpus({destination, cacheRoot, lock, fetcher = fetch,fontMetadataReader=readFontMetadata}) {
  if (!/^[a-f0-9]{40}$/.test(lock.revision) || !/^[\w-]+\/[\w.-]+$/.test(lock.repository)) {
    throw new Error('Invalid MSE upstream lock');
  }
  const cache = resolve(cacheRoot, lock.revision);
  await mkdir(cache, {recursive: true});
  async function request(url) {
    let error;
    for (let attempt = 0; attempt < 3; attempt++) {
      try {
        const response = await fetcher(url, {signal: AbortSignal.timeout(600000)});
        if (!response.ok) throw new Error(`MSE download returned HTTP ${response.status}`);
        return response;
      } catch (failure) { error = failure; }
    }
    throw error;
  }
  let tree;
  try {
    tree = JSON.parse(await readFile(join(cache, 'upstream-tree.json'), 'utf8'));
    validateTree(tree, lock);
  } catch {
    tree = await (await request(`https://api.github.com/repos/${lock.repository}/git/trees/${lock.revision}?recursive=1`)).json();
    validateTree(tree, lock);
  }
  const files = tree.tree.filter(entry => entry.type === 'blob' && runtimeFile(entry.path));
  if (!files.length) throw new Error('MSE runtime corpus is empty');
  let complete = true;
  for (const entry of files) {
    if (!await verified(cache, entry)) { complete = false; break; }
  }
  if (!complete) {
    console.log('Downloading pinned MSE archive (first build or cache repair)…');
    const scratch = await mkdtemp(join(cacheRoot, 'download-'));
    try {
      const archive = join(scratch, 'pack.tar.gz');
      const response = await request(`https://codeload.github.com/${lock.repository}/tar.gz/${lock.revision}`);
      await pipeline(response.body, createWriteStream(archive));
      const unpacked = join(scratch, 'unpacked');
      await mkdir(unpacked);
      await exec('tar', ['-xzf', archive, '--strip-components=1', '-C', unpacked]);
      for (const entry of files) {
        if (!await verified(unpacked, entry)) throw new Error(`MSE asset hash mismatch: ${entry.path}`);
        const target = join(cache, entry.path);
        await mkdir(dirname(target), {recursive: true});
        await copyFile(join(unpacked, entry.path), target);
      }
      await writeFile(join(cache, 'upstream-tree.json'), JSON.stringify(tree));
    } finally { await rm(scratch, {recursive: true, force: true}); }
  }
  // Publish only the allowlisted files, never stale cache contents or executables.
  await mkdir(destination, {recursive: true});
  for (const entry of files) {
    const target = join(destination, entry.path);
    await mkdir(dirname(target), {recursive: true});
    await copyFile(join(cache, entry.path), target);
  }
  // Write the manifest last; a failed installation must fail the deployment.
  await writeBrowserIndex(destination,tree,fontMetadataReader);
  await writeFile(join(destination, 'upstream-tree.json'), JSON.stringify(tree));
  const bytes = files.reduce((sum, entry) => sum + entry.size, 0);
  console.log(`MSE assets ready: ${files.length} files, ${(bytes / 1e6).toFixed(1)} MB, revision ${lock.revision}`);
  return {files: files.length, bytes, cached: complete};
}
