import test from 'node:test';
import assert from 'node:assert/strict';
import {chromium} from 'playwright';
import {createServer} from 'vite';

const COLLECTIONS = [['mono-color', 'Mono Red Burn'], ['last-major-events', 'Boros Energy'], ['last-20-events', 'Dimir Control']];
const entries = COLLECTIONS.flatMap(([collection, name], group) => Array.from({length: 4}, (_, index) => ({
  id: `deck-${group}-${index}`,
  format: 'modern',
  name: `${name} ${index + 1}`,
  archetype: name,
  event: 'MTGO Challenge',
  date: `2026-09-1${index}`,
  placement: index + 1,
  mainboardCount: 60,
  sideboardCount: 0,
  collections: [collection],
  cards: [name],
  cardNames: ['Mountain'],
  detail: `details/deck-${group}-${index}.json`,
})));
const index = {schemaVersion: 1, format: 'modern', generatedAt: '2026-09-18T00:00:00Z', decks: entries};

test('the load-decks workspace holds the catalog and every player deck in one container', async () => {
  const server = await createServer({server: {host: '127.0.0.1', port: 0}, logLevel: 'silent'});
  await server.listen();
  const browser = await chromium.launch();
  try {
    const page = await browser.newPage({viewport: {width: 1440, height: 900}});
    const errors = [];
    page.on('pageerror', (error) => errors.push(error.message));
    await page.route('**/catalog/modern/index.json', (route) => route.fulfill({json: index}));
    await page.route('**/catalog/modern/search-index.json', (route) => route.fulfill({status: 404, body: ''}));
    await page.route('**/catalog/modern/details/*.json', (route) => route.fulfill({json: {
      id: 'deck', format: 'modern', name: 'Mono Red Burn 1', archetype: 'Mono Red Burn',
      mainboard: [{name: 'Mountain', count: 60}], sideboard: [], commander: [],
    }}));
    await page.route('**/cards/*.json', (route) => route.fulfill({status: 404, body: ''}));
    await page.goto(`http://127.0.0.1:${server.httpServer.address().port}/tests/deck-loading-layout.html`, {waitUntil: 'domcontentloaded'});

    const workspace = page.locator('[data-deck-workspace]');
    await workspace.waitFor();
    // The catalog and the player editors share one container.
    assert.equal(await workspace.locator('[data-deck-catalog]').count(), 1);
    assert.equal(await workspace.locator('[data-player-grid]').count(), 1);

    await page.getByRole('button', {name: 'x4'}).click();
    const slots = page.locator('[data-player-slot]');
    await assert.doesNotReject(slots.nth(3).waitFor());
    assert.deepEqual(
      await workspace.locator('[data-player-slot] .truncate').evaluateAll((nodes) => nodes.map((node) => node.textContent).filter((text) => /^(Alice|Bob|Charlie|Diana)$/.test(text))),
      ['Alice', 'Bob', 'Charlie', 'Diana'],
    );

    // Four players sit in a 2x2 square: two distinct columns, two distinct rows.
    const boxes = await slots.evaluateAll((nodes) => nodes.map((node) => {
      const box = node.getBoundingClientRect();
      return {left: Math.round(box.left), top: Math.round(box.top)};
    }));
    assert.equal(new Set(boxes.map((box) => box.left)).size, 2);
    assert.equal(new Set(boxes.map((box) => box.top)).size, 2);

    // The catalog scrolls vertically inside its column, never sideways.
    const list = page.locator('[data-deck-catalog-list]');
    await list.locator('article').first().waitFor();
    const scroll = await list.evaluate((node) => ({
      vertical: node.scrollHeight > node.clientHeight,
      horizontal: node.scrollWidth > node.clientWidth,
    }));
    assert.equal(scroll.vertical, true);
    assert.equal(scroll.horizontal, false);
    assert.equal(await page.locator('[data-deck-group]').count(), 3);

    // The selected player is the catalog's target, and "Usar" fills that slot.
    await slots.nth(2).click();
    assert.equal(await slots.nth(2).getAttribute('data-catalog-target'), 'true');
    await list.getByRole('button', {name: 'Use'}).first().click();
    const charlieDeck = page.getByLabel('Charlie decklist');
    await charlieDeck.waitFor();
    await page.waitForFunction(() => document.querySelectorAll('textarea')[2]?.value.includes('60 Mountain'));
    assert.match(await charlieDeck.inputValue(), /60 Mountain/);
    assert.equal(await page.getByLabel('Alice decklist').inputValue(), '');

    const screenshotPath = globalThis.process?.env?.DECK_LOADING_SCREENSHOT;
    if (screenshotPath) await page.screenshot({path: screenshotPath});

    // Fewer players keep the square: two side by side, one filling the row.
    const rowsAndColumns = async () => slots.evaluateAll((nodes) => {
      const boxes = nodes.map((node) => node.getBoundingClientRect());
      return {
        count: boxes.length,
        columns: new Set(boxes.map((box) => Math.round(box.left))).size,
        rows: new Set(boxes.map((box) => Math.round(box.top))).size,
      };
    });
    await page.getByRole('button', {name: 'x2'}).click();
    assert.deepEqual(await rowsAndColumns(), {count: 2, columns: 2, rows: 1});
    await page.getByRole('button', {name: 'x1'}).click();
    assert.deepEqual(await rowsAndColumns(), {count: 1, columns: 1, rows: 1});
    const [gridWidth, slotWidth] = await Promise.all([
      page.locator('[data-player-grid]').evaluate((node) => Math.round(node.getBoundingClientRect().width)),
      slots.first().evaluate((node) => Math.round(node.getBoundingClientRect().width)),
    ]);
    assert.ok(slotWidth > gridWidth * 0.9, `a single player should fill the grid (${slotWidth} of ${gridWidth})`);

    assert.deepEqual(errors, []);
  } finally {
    await browser.close();
    await server.close();
  }
});

test('the workspace reads its copy from the translation catalog', async () => {
  const server = await createServer({server: {host: '127.0.0.1', port: 0}, logLevel: 'silent'});
  await server.listen();
  const browser = await chromium.launch();
  try {
    const page = await browser.newPage({viewport: {width: 1440, height: 900}});
    const errors = [];
    page.on('pageerror', (error) => errors.push(error.message));
    await page.addInitScript(() => window.localStorage.setItem('ironsmith.locale', 'es'));
    await page.route('**/catalog/modern/index.json', (route) => route.fulfill({json: index}));
    await page.route('**/catalog/modern/search-index.json', (route) => route.fulfill({status: 404, body: ''}));
    await page.route('**/cards/*.json', (route) => route.fulfill({status: 404, body: ''}));
    await page.goto(`http://127.0.0.1:${server.httpServer.address().port}/tests/deck-loading-layout.html`, {waitUntil: 'domcontentloaded'});

    const workspace = page.locator('[data-deck-workspace]');
    await workspace.waitFor();
    await assert.doesNotReject(workspace.getByText('Mazos de los jugadores').waitFor());
    await assert.doesNotReject(workspace.getByText('Sin mazo asignado').first().waitFor());
    await assert.doesNotReject(page.getByLabel('Buscar en el catálogo').waitFor());
    await assert.doesNotReject(page.locator('[data-deck-group="Monocolor"]').waitFor());
    assert.equal(await page.locator('[data-deck-catalog-list] button', {hasText: 'Usar'}).first().textContent(), 'Usar');
    assert.deepEqual(errors, []);
  } finally {
    await browser.close();
    await server.close();
  }
});
