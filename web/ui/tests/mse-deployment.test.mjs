import test from 'node:test';
import assert from 'node:assert/strict';
import {mkdtemp, mkdir, writeFile, readFile, rm, access} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import {join} from 'node:path';
import {execFile} from 'node:child_process';
import {promisify} from 'node:util';
import {installCorpus, blobHash} from '../scripts/mse-deploy/install.mjs';
import {mseDeploymentPlugin} from '../scripts/mse-deploy/vite-plugin.mjs';

const exec = promisify(execFile);
test('archive install, exclusions, verified cache reuse, repair, and failed download', async () => {
  const root = await mkdtemp(join(tmpdir(), 'mse-deployment-'));
  try {
    const lock = {repository: 'example/pack', revision: 'a'.repeat(40)};
    const sources = {'data/magic.mse-game/game': 'game', 'data/test.mse-style/style': 'style',
      'data/test.mse-style/image.png': 'pixels', 'Magic - Fonts/font.ttf': 'font',
      'data/dictionaries/en.dic': 'dictionary', 'data/test.mse-style/source.psd': 'source',
      'data/test.mse-updater/update': 'updater'};
    const tree = {sha: lock.revision, truncated: false, tree: []};
    for (const [path, content] of Object.entries(sources)) {
      const bytes = Buffer.from(content);
      tree.tree.push({path, type: 'blob', sha: blobHash(bytes), size: bytes.length});
      const target = join(root, 'source', 'pack', path);
      await mkdir(join(target, '..'), {recursive: true});
      await writeFile(target, bytes);
    }
    const archive = join(root, 'pack.tar.gz');
    await exec('tar', ['-czf', archive, '-C', join(root, 'source'), 'pack']);
    let requests = 0;
    const fetcher = async url => {
      requests++;
      return new Response(url.includes('api.github.com') ? JSON.stringify(tree) : await readFile(archive));
    };
    const options = {fontMetadataReader:()=>({families:['Test']}),lock, cacheRoot: join(root, 'cache'), destination: join(root, 'publish'), fetcher};
    assert.equal((await installCorpus(options)).files, 4);
    assert.equal(requests, 2);
    assert.equal(await readFile(join(options.destination, 'data/test.mse-style/style'), 'utf8'), 'style');
    await assert.rejects(access(join(options.destination, 'data/dictionaries/en.dic')));
    await rm(options.destination, {recursive: true});
    assert.equal((await installCorpus({...options, fetcher: () => {throw new Error('offline');}})).cached, true);
    await writeFile(join(options.cacheRoot, lock.revision, 'data/test.mse-style/style'), 'corrupted');
    assert.equal((await installCorpus(options)).cached, false);
    await writeFile(join(options.cacheRoot, lock.revision, 'data/test.mse-style/style'), 'corrupted');
    await assert.rejects(installCorpus({...options, fetcher: async () => new Response('', {status: 503})}), /503/);
  } finally { await rm(root, {recursive: true, force: true}); }
});

test('Vite emits a self-contained Netlify plugin', async () => {
  const files = new Map();
  await mseDeploymentPlugin().generateBundle.call({emitFile: asset => files.set(asset.fileName, asset.source)});
  for (const name of ['index.js', 'install.mjs', 'upstream.json', 'package.json', 'manifest.yml']) {
    assert.ok(files.has(`deploy/mse/${name}`), name);
  }
  assert.equal(JSON.parse(files.get('deploy/mse/package.json')).type, 'commonjs');
  assert.match(files.get('deploy/mse/index.js'), /onPostBuild/);
});

test('emitted Netlify hook installs under publish/ironsmith and propagates failures', async () => {
  const root = await mkdtemp(join(tmpdir(), 'mse-netlify-hook-'));
  try {
    const files = [];
    await mseDeploymentPlugin().generateBundle.call({emitFile: asset => files.push(asset)});
    for (const asset of files) {
      const target = join(root, asset.fileName);
      await mkdir(join(target, '..'), {recursive: true});
      await writeFile(target, asset.source);
    }
    const installer=await import(join(root,'deploy/mse/install.mjs'));
    assert.equal(typeof installer.installCorpus,'function');
    await writeFile(join(root, 'deploy/mse/install.mjs'), `
      export async function installCorpus(options) {
        if (!options.destination.endsWith('/ironsmith/mse/' + options.lock.revision)) throw Error('Wrong destination');
        if (!options.cacheRoot.endsWith('/.cache/ironsmith-mse')) throw Error('Wrong cache');
        if (options.destination.includes('fail-install')) throw Error('download failed');
      }
    `);
    await writeFile(join(root,'deploy/mse/index.js'),(await readFile(join(root,'deploy/mse/index.js'),'utf8')).replace("pathToFileURL(join(__dirname, 'install.mjs')).href","pathToFileURL(join(__dirname, 'install.mjs')).href + '?hook-test'"));
    const {default: plugin} = await import(join(root, 'deploy/mse/index.js'));
    const events = [];
    const utils = {cache: {
      restore: async path => events.push(['restore', path]),
      save: async path => events.push(['save', path]),
    }, build: {failBuild: (message, {error}) => {throw new Error(message, {cause: error});}}};
    await plugin.onPostBuild({constants: {PUBLISH_DIR: join(root, 'public')}, utils});
    assert.deepEqual(events.map(e => e[0]), ['restore', 'save']);
    assert.equal(events[0][1], events[1][1]);
    events.length = 0;
    await assert.rejects(plugin.onPostBuild({constants: {PUBLISH_DIR: join(root, 'fail-install')}, utils}), /Could not install/);
    assert.deepEqual(events.map(e => e[0]), ['restore']);
  } finally { await rm(root, {recursive: true, force: true}); }
});
