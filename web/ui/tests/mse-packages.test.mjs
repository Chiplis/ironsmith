import test from 'node:test';
import assert from 'node:assert/strict';
import { parseMseDocument } from '../src/lib/mse/document.mjs';

test('missing colons retain records and children with a located warning', () => {
  const doc = parseMseDocument('card style:\n\tpaintbrush\n\t\tleft: 325\n\t\ttop: 484\n\tnext: ok', {file:'style'});
  const styles = doc.last('card style').record();
  assert.equal(styles.last('paintbrush').record().text('left'),'325');
  assert.equal(styles.text('next'),'ok');
  assert.equal(styles.warnings.length,1);
  assert.equal(styles.warnings[0].code,'missing-colon');
  assert.equal(styles.warnings[0].location.line,2);
});
import { MsePackageStore, resolveMsePath, compareMseVersions, createFetchMseStore } from '../src/lib/mse/packages.mjs';

test('package existence checks use a complete file index and distinguish missing packages', () => {
  const store = new MsePackageStore({ files: ['test.mse-style/style', 'test.mse-style/images/frame.png'] });
  assert.equal(store.exists('/test.mse-style/images/frame.png'), true);
  assert.equal(store.exists('//test.mse-style/images/frame.png'), true);
  assert.equal(store.exists('/test.mse-style/images/absent.png'), false);
  assert.throws(() => store.exists('/missing.mse-style/file.png'), { code: 'package-missing' });
  assert.throws(() => store.exists('images/frame.png'), { code: 'package-path' });
  assert.throws(() => new MsePackageStore({}).exists('/test.mse-style/file'), { code: 'package-index' });
});

test('MSE keeps repeated records and literal script/text blocks separate', () => {
  const document = parseMseDocument('\uFEFFmse version: 2.6.0\r\ndepends on:\r\n\tpackage: magic.mse-game\r\n\tversion: 2026-09-06\r\ndepends on:\r\n\tpackage: shared.mse-include\r\ninit script:\r\n\t# comment inside script\r\n\tf := {\r\n\t\t[card: "cards/", pt: "pts/"]\r\n\t}\r\n\r\n\tf()\r\ncard style:\r\n\tname:\r\n\t\tfont:\r\n\t\t\tname: Beleren\r\n', { file: 'style' });
  assert.equal(document.all('depends on').length, 2);
  assert.equal(document.all('depends on')[0].record().text('version'), '2026-09-06');
  assert.equal(document.text('init script'), '# comment inside script\nf := {\n\t[card: "cards/", pt: "pts/"]\n}\n\nf()');
  assert.equal(document.last('card style').record().last('name').record().last('font').record().text('name'), 'Beleren');
  assert.equal(document.last('init script').location.line, 7);
});

test('inline values preserve colons, trailing spaces and case; multiline retains tagged text', () => {
  const doc = parseMseDocument('Name:  A: B  \ntext:\n\t<sym>W</sym>\n\t\tIndented\nnext: ok');
  assert.equal(doc.text('Name'), 'A: B  ');
  assert.equal(doc.text('name', 'missing'), 'missing');
  assert.equal(doc.text('text'), '<sym>W</sym>\n\tIndented');
  assert.throws(() => parseMseDocument('card:\n  name: wrong').last('card').record(), /./);
});

test('package paths are rooted in the owning package, with explicit cross-package switching', () => {
  assert.deepEqual(resolveMsePath('images/../frame.png', 'a.mse-style'), { package: 'a.mse-style', file: 'frame.png', key: 'a.mse-style/frame.png' });
  assert.equal(resolveMsePath('/shared.mse-include/cards/gold.png', 'a.mse-style').key, 'shared.mse-include/cards/gold.png');
  assert.throws(() => resolveMsePath('../../outside', 'a.mse-style'), { code: 'invalid-path' });
  assert.throws(() => resolveMsePath('frame.png', '../a'), { code: 'invalid-package' });
  assert.equal(compareMseVersions('2.10', '2.6.0'), 1);
  assert.equal(compareMseVersions('2026-09-06', '2026-09-06'), 0);
});

function memoryStore(files) {
  const reads = [];
  const store = new MsePackageStore({
    readText: async path => { reads.push(path.key); if (!(path.key in files)) throw new Error(`missing ${path.key}`); return files[path.key]; },
    assetUrl: path => `asset:${path.key}`,
  });
  return { store, reads };
}

test('includes preserve order and asset ownership without expanding script includes as records', async () => {
  const { store } = memoryStore({
    'a.mse-style/style': 'card style:\n\tname:\n\t\tleft: 12\n\tinclude file: /shared.mse-include/fields\ninit script:\n\tinclude file: /shared.mse-include/scripts',
    'shared.mse-include/fields': 'mse version: 2.0\nimage:\n\timage: cards/art.png\ninclude file: tail',
    'shared.mse-include/tail': 'power:\n\tleft: 250',
  });
  const doc = await store.document('style', 'a.mse-style');
  const expanded = await store.expandRecord(doc.last('card style').record());
  assert.deepEqual(expanded.entries.map(entry => entry.key), ['name', 'image', 'power']);
  const image = expanded.last('image').record().last('image');
  assert.equal(store.asset(image.text(), image.location.package), 'asset:shared.mse-include/cards/art.png');
  assert.equal(doc.text('init script'), 'include file: /shared.mse-include/scripts');
});

test('dependency closure loads once, checks every required version and permits mutual dependencies', async () => {
  const files = {
    'a.mse-style/style': 'version: 1\ndepends on:\n\tpackage: b.mse-include\n\tversion: 2\ndepends on:\n\tpackage: c.mse-include',
    'b.mse-include/include': 'version: 2',
    'c.mse-include/include': 'version: 1\ndepends on:\n\tpackage: b.mse-include\n\tversion: 2',
  };
  const { store, reads } = memoryStore(files);
  assert.deepEqual((await store.dependencyClosure('a.mse-style')).map(p => p.name), ['b.mse-include', 'c.mse-include', 'a.mse-style']);
  await store.dependencyClosure('a.mse-style');
  assert.equal(reads.length, 3);
  files['c.mse-include/include'] = 'depends on:\n\tpackage: b.mse-include\n\tversion: 3';
  await assert.rejects(memoryStore(files).store.dependencyClosure('a.mse-style'), { code: 'dependency-version' });
  files['c.mse-include/include'] = 'depends on:\n\tpackage: a.mse-style';
  assert.equal((await memoryStore(files).store.dependencyClosure('a.mse-style')).length, 3);
});

test('unindented comments do not terminate compound records', () => {
  const doc = parseMseDocument('card style:\n\ttext:\n\t\tleft: 6\n# disabled top setting\n\t\tright: 4\n# next field\n\tname:\n\t\tleft: 12\nversion: 1');
  const style = doc.last('card style').record();
  assert.equal(style.last('text').record().text('right'), '4');
  assert.equal(style.last('name').record().text('left'), '12');
  assert.equal(doc.text('version'), '1');
});

test('legacy dependency declarations and optional suggestions load', async () => {
  const { store } = memoryStore({
    'a.mse-style/style': 'depends on: b.mse-include 2\ndepends on:\n\tpackage: absent.mse-include\n\tsuggests: true',
    'b.mse-include/include': 'version: 2',
  });
  assert.deepEqual((await store.dependencyClosure('a.mse-style')).map(pkg => pkg.name), ['b.mse-include', 'a.mse-style']);
});

test('failed reads are retried; recursive includes fail with an actionable chain', async () => {
  const files = {};
  const { store } = memoryStore(files);
  await assert.rejects(store.document('style', 'a.mse-style'), /missing/);
  files['a.mse-style/style'] = 'include file: other';
  files['a.mse-style/other'] = 'include file: style';
  const doc = await store.document('style', 'a.mse-style');
  await assert.rejects(store.expandRecord(doc), { code: 'include-cycle' });
});

test('browser transport escapes literal filenames and rejects failed responses', async () => {
  const urls = [];
  const store = createFetchMseStore('https://example.test/mse/', async url => {
    urls.push(url); return { ok: false, status: 404 };
  });
  assert.equal(store.asset('cards/a #1.png', 'a.mse-style'), 'https://example.test/mse/a.mse-style/cards/a%20%231.png');
  await assert.rejects(store.document('style', 'a.mse-style'), { code: 'package-fetch' });
  assert.deepEqual(urls, ['https://example.test/mse/a.mse-style/style']);
});
