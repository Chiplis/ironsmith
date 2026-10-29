import test from 'node:test';
import assert from 'node:assert/strict';
import { createServer } from 'vite';
import { chromium } from 'playwright';

test('Community editor combines sources, saves formats and reuses custom sets', async t => {
  const server = await createServer({ server: { host: '127.0.0.1', port: 0 }, logLevel: 'error' });
  await server.listen(); t.after(() => server.close());
  const browser = await chromium.launch(); t.after(() => browser.close());
  const page = await browser.newPage({ viewport: { width: 1100, height: 1000 } });
  const errors = []; page.on('pageerror', error => errors.push(error.message));
  await page.route('**/custommagic/pool.json', route => route.fulfill({ json: { cards: [{ id: 'mage', draft: { faces: [{ name: 'Community Mage' }] } }] } }));
  await page.route('https://api.scryfall.com/**', route => {
    const url = new URL(route.request().url());
    return route.fulfill({ json: url.pathname.startsWith('/sets/') ? { code: 'dom', name: 'Dominaria' }
      : url.pathname.endsWith('/named') ? { name: url.searchParams.get('exact') }
      : { data: [{ name: 'Island' }, { name: 'Opt' }], has_more: false } });
  });
  await page.goto(`http://127.0.0.1:${server.httpServer.address().port}/tests/community-editor.html`);
  await page.getByLabel('Community format name', { exact: true }).fill('Our Friday format');
  await page.getByLabel('Set code or card name').fill('dom');
  await page.getByRole('button', { name: 'Add', exact: true }).click();
  await page.getByText('Dominaria (DOM) · 2 cards', { exact: true }).waitFor();
  await page.getByLabel('Official card source').selectOption('excludedCards');
  await page.getByLabel('Set code or card name').fill('Opt');
  await page.getByRole('button', { name: 'Add', exact: true }).click();
  await page.getByRole('button', { name: 'Remove Opt', exact: true }).waitFor();
  await page.getByLabel('Community Mage', { exact: true }).check();
  await page.getByLabel('Custom set name', { exact: true }).fill('Our creations');
  await page.getByRole('button', { name: 'Save selected cards as a set' }).click();
  await page.getByRole('button', { name: 'Save format', exact: true }).click();
  await page.screenshot({ path: '/tmp/community-editor.png', fullPage: true });
  await page.reload();
  await page.getByLabel('Saved formats').selectOption('0');
  const format = await page.evaluate(() => window.__format);
  assert.equal(format.name, 'Our Friday format');
  assert.deepEqual(format.excludedCards, ['Opt']);
  assert.equal(format.customSets[0].cards[0].draft.faces[0].name, 'Community Mage');
  assert.deepEqual(errors, []);
});

test('Community tab creates a Trusted custom room from the lobby workspace', async t => {
  const server = await createServer({ server: { host: '127.0.0.1', port: 0 }, logLevel: 'error' });
  await server.listen(); t.after(() => server.close());
  const browser = await chromium.launch(); t.after(() => browser.close());
  const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });
  await page.route('**/custommagic/pool.json', route => route.fulfill({ json: { cards: [] } }));
  await page.goto(`http://127.0.0.1:${server.httpServer.address().port}/tests/fixtures/relay-lobby.html`);
  await page.getByRole('button', { name: 'Community', exact: true }).click();
  await page.getByLabel('Community format name').fill('Custom night');
  await page.screenshot({ path: '/tmp/community-lobby.png', fullPage: true });
  const layout = await page.locator('.community-format-editor').evaluate(el => ({ height: el.clientHeight, scrollHeight: el.scrollHeight, overflow: getComputedStyle(el).overflowY }));
  assert.equal(layout.overflow, 'auto');
  assert.ok(layout.height > 200 && layout.scrollHeight >= layout.height);
  await page.getByRole('button', { name: 'Create Lobby', exact: true }).click();
  const created = await page.evaluate(() => window.createdLobby);
  assert.equal(created.communityFormat.name, 'Custom night');
  assert.equal(created.securityMode, 'trusted');
  assert.equal(created.transport, 'peerjs');
});

test('exports include mechanics and import into a fresh browser without overwriting saved names', async t => {
  const { readFile } = await import('node:fs/promises');
  const server = await createServer({ server: { host: '127.0.0.1', port: 0 }, logLevel: 'error' });
  await server.listen(); t.after(() => server.close());
  const browser = await chromium.launch(); t.after(() => browser.close());
  const base = `http://127.0.0.1:${server.httpServer.address().port}/tests/community-editor.html`;
  const mechanics = [{ keyword: 'stargaze', reminder: 'Draw a card.', expansion: 'Draw a card.', version: 1, sourceCard: 'Star Grove' }];
  const set = { name: 'Sky set', cards: [{ id: 'grove', draft: { layout: 'single', customMechanics: mechanics, faces: [{ name: 'Star Grove', cardTypes: ['Land'], oracleText: '{T}: Stargaze.' }] } }] };
  const format = { name: 'Sky night', includedSets: [], excludedSets: [], includedCards: ['Plains'], excludedCards: ['Opt'], customSets: [set], customCards: [] };
  const source = await browser.newContext();
  // Exercise migration from the previously shipped localStorage schema.
  await source.addInitScript(legacy => localStorage.setItem('ironsmith-community-library-v1', JSON.stringify(legacy)), { formats: [format], sets: [set] });
  await source.route('**/custommagic/pool.json', route => route.fulfill({ json: { cards: [] } }));
  const page = await source.newPage(); await page.goto(base);
  await page.getByLabel('Saved formats').selectOption('0');
  const download = async button => {
    const pending = page.waitForEvent('download'); await page.getByRole('button', { name: button, exact: true }).click();
    const file = await pending; return { name: file.suggestedFilename(), buffer: await readFile(await file.path()), mimeType: 'application/json' };
  };
  const exportedFormat = await download('Export format');
  await page.getByLabel('Custom set to export').selectOption('saved:0');
  const exportedSet = await download('Export custom set');
  assert.deepEqual(JSON.parse(exportedFormat.buffer).data.customSets[0].cards[0].draft.customMechanics, mechanics);
  assert.deepEqual(JSON.parse(exportedSet.buffer).data.cards[0].draft.customMechanics, mechanics);
  const destination = await browser.newContext();
  await destination.route('**/custommagic/pool.json', route => route.fulfill({ json: { cards: [] } }));
  const imported = await destination.newPage(); await imported.goto(base);
  await imported.getByRole('button', { name: 'Import set or format' }).waitFor();
  await imported.waitForFunction(() => !document.querySelector('input[type=file]').disabled);
  await imported.getByLabel('Import Community file').setInputFiles(exportedFormat);
  await imported.getByRole('status').filter({ hasText: 'Imported Sky night.' }).waitFor();
  await imported.getByLabel('Import Community file').setInputFiles(exportedFormat);
  await imported.getByRole('status').filter({ hasText: 'Imported Sky night (imported 2).' }).waitFor();
  await imported.getByLabel('Import Community file').setInputFiles(exportedSet);
  await imported.getByRole('status').filter({ hasText: 'Imported Sky set.' }).waitFor();
  const library = await imported.evaluate(async () => (await import('/src/lib/community-library.js')).loadCommunityLibrary());
  assert.equal(library.formats.length, 2);
  assert.deepEqual(library.sets[0].cards[0].draft.customMechanics, mechanics);
  assert.deepEqual(library.formats[0].excludedCards, ['Opt']);
  await imported.getByLabel('Import Community file').setInputFiles({ name: 'invalid.json', mimeType: 'application/json', buffer: Buffer.from('{"schema":"nope"}') });
  await imported.getByRole('alert').filter({ hasText: 'Invalid Community file' }).waitFor();
  await imported.reload();
  await imported.getByLabel('Saved formats').selectOption('0');
  assert.deepEqual(await imported.evaluate(() => window.__format.customSets[0].cards[0].draft.customMechanics), mechanics);
  // Migration is idempotent and never re-adds/duplicates items on reload.
  await page.reload();
  await page.getByLabel('Saved formats').selectOption('0');
  assert.equal(await page.getByLabel('Saved formats').locator('option').count(), 2);
});
