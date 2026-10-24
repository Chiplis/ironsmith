import test from 'node:test';
import assert from 'node:assert/strict';
import { chromium } from 'playwright';
import { createServer } from 'vite';

async function fixture(run) {
  const server = await createServer({ server: { host: '127.0.0.1', port: 0 }, logLevel: 'silent' });
  await server.listen();
  const browser = await chromium.launch();
  try {
    const context = await browser.newContext();
    await context.route('**/catalog/**', route => route.fulfill({ json: { decks: [] } }));
    const url = `http://127.0.0.1:${server.httpServer.address().port}/tests/lobby-deck-picker.html?save`;
    await run(context, url);
  } finally {
    await browser.close();
    await server.close();
  }
}

test('Save Deck validates names, saves, replaces, and restores partners and sideboards in a new session', () => fixture(async (context, url) => {
  let page = await context.newPage();
  await page.goto(url);
  await page.getByRole('button', { name: 'Save Deck', exact: true }).click();
  const dialog = page.getByRole('dialog');
  assert.equal(await dialog.getByRole('button', { name: 'Save', exact: true }).isDisabled(), true);
  await dialog.getByLabel('Deck name', { exact: true }).fill('   ');
  assert.equal(await dialog.getByRole('button', { name: 'Save', exact: true }).isDisabled(), true);
  await dialog.getByLabel('Deck name', { exact: true }).fill('Partner islands');
  await dialog.getByRole('button', { name: 'Save', exact: true }).click();
  await dialog.waitFor({ state: 'hidden' });
  await page.getByText('My decks', { exact: true }).click();
  await page.locator('[data-saved-deck]').waitFor();
  await page.getByRole('button', { name: 'Save Deck', exact: true }).click();
  await dialog.getByLabel('Deck name', { exact: true }).fill('partner ISLANDS');
  await dialog.getByRole('button', { name: 'Replace deck', exact: true }).click();
  await dialog.waitFor({ state: 'hidden' });
  assert.equal(await page.locator('[data-saved-deck]').count(), 1);
  await page.close();
  page = await context.newPage();
  await page.goto(url);
  await page.getByText('My decks', { exact: true }).click();
  await page.locator('[data-saved-deck]').getByRole('button', { name: 'Use', exact: true }).click();
  const applied = await page.evaluate(() => window.__applied);
  assert.match(applied.deckText, /98 Island/);
  assert.match(applied.deckText, /Sideboard\n1 Forest/);
  assert.doesNotMatch(applied.deckText, /Tymna/);
  assert.match(applied.commanderText, /Tymna the Weaver/);
  assert.match(applied.commanderText, /Kraum, Ludevic's Opus/);
}));

test('IndexedDB migrates legacy decks, has no five-deck cap, and serializes concurrent writes', () => fixture(async (context, url) => {
  const page = await context.newPage();
  await page.goto(url);
  const result = await page.evaluate(async () => {
    const api = await import('/src/lib/saved-decks.js');
    const key = 'ironsmith.savedDeckPresets.v3';
    localStorage.setItem(key, JSON.stringify([{name: 'Old local', texts: ['60 Island'], formats: ['modern'], updatedAt: 20} ]));
    sessionStorage.setItem(key, JSON.stringify([{name: 'Old session', texts: ['60 Forest'], formats: ['modern']}, {name: 'Old local', texts: ['60 Mountain'], formats: ['modern'], updatedAt: 10} ]));
    const migrated = await api.listSavedDeckPresets();
    const saves = await Promise.all(Array.from({length: 12}, (_, i) => api.saveSavedDeckPreset(`Deck ${i}`, ['60 Island'], ['Host'], ['paupercommander', 'bogus', 'pauper'])));
    const all = await api.listSavedDeckPresets();
    await api.removeSavedDeckPreset('Old session');
    const deleted = await api.listSavedDeckPresets();
    const invalid = await api.saveSavedDeckPreset('No format', ['1 Island'], [], []);
    return {migrated, saves: saves.map(s => s.saved), all, deleted, invalid};
  });
  assert.equal(result.migrated.length, 2);
  assert.deepEqual(result.migrated.find(d => d.name === 'Old local').texts, ['60 Island']);
  assert.ok(result.saves.every(Boolean));
  assert.equal(result.all.length, 14);
  assert.deepEqual(result.all.find(d => d.name === 'Deck 0').formats, ['pauper', 'paupercommander']);
  assert.equal(result.deleted.length, 13);
  assert.ok(!result.deleted.some(d => d.name === 'Old session'));
  assert.equal(result.invalid.reason, 'format');
  await page.reload();
  const count = await page.evaluate(async () => (await (await import('/src/lib/saved-decks.js')).listSavedDeckPresets()).length);
  assert.equal(count, 13);
}));

test('failed IndexedDB writes keep the modal open with an error and never report success', () => fixture(async (context, url) => {
  const page = await context.newPage();
  await page.goto(url);
  await page.getByRole('button', {name: 'Save Deck', exact: true}).click();
  await page.getByLabel('Deck name', {exact: true}).fill('Cannot save');
  await page.evaluate(() => {
    IDBObjectStore.prototype.put = function () { throw new DOMException('Storage full', 'QuotaExceededError'); };
  });
  await page.getByRole('button', {name: 'Save', exact: true}).click();
  await page.getByRole('alert').waitFor();
  assert.equal(await page.getByRole('dialog').isVisible(), true);
  assert.equal(await page.getByText('Deck saved', {exact: true}).count(), 0);
}));

test('My decks updates across tabs after saving and deleting', () => fixture(async (context, url) => {
  const first = await context.newPage();
  const second = await context.newPage();
  await first.goto(url);
  await second.goto(url);
  await second.getByText('My decks', {exact: true}).click();
  await first.evaluate(async () => {
    const api = await import('/src/lib/saved-decks.js');
    await api.saveSavedDeckPreset('Other tab', ['60 Island'], [], ['normal']);
  });
  await second.locator('[data-saved-deck]').waitFor();
  await first.evaluate(async () => {
    await (await import('/src/lib/saved-decks.js')).removeSavedDeckPreset('Other tab');
  });
  await second.locator('[data-saved-deck]').waitFor({state: 'hidden'});
}));
