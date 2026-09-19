import test from 'node:test';
import assert from 'node:assert/strict';
import {chromium} from 'playwright';
import {createServer} from 'vite';

const decks = Array.from({length: 6}, (_, index) => ({
  id: `deck-${index}`,
  format: 'modern',
  name: `Boros Energy ${index + 1}`,
  archetype: 'Boros Energy',
  event: 'MTGO Challenge',
  date: `2026-09-1${index}`,
  placement: index + 1,
  mainboardCount: 60,
  sideboardCount: 15,
  cards: ['Mountain'],
  cardNames: ['Mountain'],
  artCard: 'Atraxa',
  manaProfile: {
    colors: ['R', 'W'],
    landCount: 24,
    predominantColors: ['R'],
    predominantLands: [{name: 'Mountain', count: 8}],
    metadataCoverage: {complete: true},
  },
  detail: `details/deck-${index}.json`,
}));
const index = {schemaVersion: 1, format: 'modern', generatedAt: '2026-09-18T00:00:00Z', decks};

test('the lobby picker shows catalog decks with art and applies one flatly', async () => {
  const server = await createServer({server: {host: '127.0.0.1', port: 0}, logLevel: 'silent'});
  await server.listen();
  const browser = await chromium.launch();
  try {
    const page = await browser.newPage({viewport: {width: 1000, height: 800}});
    const errors = [];
    page.on('pageerror', (error) => errors.push(error.message));
    await page.route('**/catalog/modern/index.json', (route) => route.fulfill({json: index}));
    await page.route('**/catalog/modern/search-index.json', (route) => route.fulfill({status: 404, body: ''}));
    await page.route('**/catalog/modern/details/*.json', (route) => route.fulfill({json: {
      id: 'deck-0', format: 'modern', name: 'Boros Energy 1',
      mainboard: [{name: 'Mountain', count: 60}], sideboard: [], commander: [],
    }}));
    await page.route('**/cards/*.json', (route) => route.fulfill({status: 404, body: ''}));
    await page.route('**/cards/atraxa.json', (route) => route.fulfill({json: {scryfall: {image_uris: {art_crop: 'data:image/svg+xml,%3Csvg xmlns=%22http://www.w3.org/2000/svg%22 width=%2210%22 height=%2210%22%3E%3C/svg%3E'}}}}));
    await page.goto(`http://127.0.0.1:${server.httpServer.address().port}/tests/lobby-deck-picker.html`, {waitUntil: 'domcontentloaded'});

    const rows = page.locator('[data-lobby-deck-row]');
    await rows.first().waitFor();
    assert.equal(await rows.count(), 6);

    // Every deck is offered as art plus a single Use, like the workspace.
    await page.waitForFunction(() => document.querySelectorAll('[data-deck-art="loaded"]').length === 6);
    assert.equal(await page.locator('[data-lobby-deck-row] button').count(), 6);

    // The sheet's chrome draws borders on controls with !important; the flat
    // scope has to win inside it.
    const strokes = await page.locator('.lobby-deck-picker, .lobby-deck-picker *').evaluateAll((nodes) => nodes.filter((node) => {
      const style = getComputedStyle(node);
      return ['Top', 'Right', 'Bottom', 'Left'].some((side) => parseFloat(style[`border${side}Width`]) > 0);
    }).length);
    assert.equal(strokes, 0);
    assert.equal(
      await page.locator('.lobby-deck-picker input').evaluate((node) => getComputedStyle(node).backgroundColor),
      'rgb(5, 6, 7)',
    );

    await rows.first().getByRole('button', {name: 'Use'}).click();
    await page.waitForFunction(() => Boolean(window.__applied));
    assert.match(await page.evaluate(() => window.__applied.deckText), /60 Mountain/);
    assert.deepEqual(errors, []);
  } finally {
    await browser.close();
    await server.close();
  }
});
