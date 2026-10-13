import test from 'node:test';
import assert from 'node:assert/strict';
import { MsePackageStore } from '../src/lib/mse/packages.mjs';
import { MseScriptLoader } from '../src/lib/mse/script-loader.mjs';

test('script includes expand inside functions and retain cross-package scope', async () => {
  const files = {
    'shared.mse-include/first': 'include file: nested/second',
    'shared.mse-include/nested/second': 'f := { include_file("third") }',
    'shared.mse-include/third': '42',
  };
  let reads = 0;
  const loader = new MseScriptLoader(new MsePackageStore({ readText: async path => { reads++; return files[path.key]; } }));
  const source = 'include file: /shared.mse-include/first\nf()';
  const ast = await loader.compile(source, { package: 'a.mse-style' });
  assert.equal(ast.left.right.body.value, 42);
  assert.equal(ast.left.right.body.location.package, 'shared.mse-include');
  await loader.compile(source, { package: 'a.mse-style' });
  assert.equal(reads, 3);
});

test('recursive script includes report cycles without awaiting their own promise', async () => {
  const loader = new MseScriptLoader(new MsePackageStore({ readText: async () => 'include file: script' }));
  await assert.rejects(loader.compile('include file: script', { package: 'a.mse-style' }), { code: 'script-include-cycle' });
});

test('real-sized game initialization sequences expand without host stack overflow', async () => {
  const store = new MsePackageStore({ readText: async () => Array.from({ length: 12000 }, (_, i) => `v${i} := ${i}`).join('\n') });
  const ast = await new MseScriptLoader(store).compile('include file: script', { package: 'game.mse-game' });
  assert.equal(ast.type, 'sequence');
  assert.equal(ast.right.left.name, 'v11999');
});

test('Cajun localized and dark includes select the same variants in records and scripts', async () => {
  const requests = [];
  const store = new MsePackageStore({ locale: 'es', dark: true, readText: async path => {
    requests.push(path.key);
    return path.file.startsWith('fields') ? 'name: Included' : '1';
  } });
  const { parseMseDocument } = await import('../src/lib/mse/document.mjs');
  const record = parseMseDocument('include localized file: fields\ninclude dark file: fields', { package: 'a.mse-style' });
  await store.expandRecord(record);
  await new MseScriptLoader(store).compile('include localized file: script\ninclude dark file: script', { package: 'a.mse-style' });
  assert.deepEqual(requests, ['a.mse-style/fields_es', 'a.mse-style/fields_dark', 'a.mse-style/script_es', 'a.mse-style/script_dark']);
});
