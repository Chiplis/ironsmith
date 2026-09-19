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
  ...(index === 0 ? {artCard: 'Atraxa'} : {}),
  manaProfile: {
    colors: ['W', 'U', 'B', 'R'],
    landCount: 24,
    predominantColors: ['R'],
    predominantLands: [{name: 'Mountain', count: 8}],
    metadataCoverage: {complete: true},
  },
  detail: `details/deck-${group}-${index}.json`,
})));
const index = {schemaVersion: 1, format: 'modern', generatedAt: '2026-09-18T00:00:00Z', decks: entries};

async function openWorkspace(server, browser, {locale} = {}) {
  const page = await browser.newPage({viewport: {width: 1440, height: 900}});
  const errors = [];
  page.on('pageerror', (error) => errors.push(error.message));
  if (locale) await page.addInitScript((value) => window.localStorage.setItem('ironsmith.locale', value), locale);
  await page.route('**/catalog/modern/index.json', (route) => route.fulfill({json: index}));
  await page.route('**/catalog/modern/search-index.json', (route) => route.fulfill({status: 404, body: ''}));
  await page.route('**/catalog/modern/details/*.json', (route) => route.fulfill({json: {
    id: 'deck', format: 'modern', name: 'Boros Energy 4', archetype: 'Boros Energy',
    mainboard: [{name: 'Mountain', count: 60}], sideboard: [], commander: [],
  }}));
  await page.route('**/cards/*.json', (route) => route.fulfill({status: 404, body: ''}));
  await page.route('**/cards/atraxa.json', (route) => route.fulfill({json: {scryfall: {image_uris: {art_crop: 'data:image/svg+xml,%3Csvg xmlns=%22http://www.w3.org/2000/svg%22 width=%2210%22 height=%2210%22%3E%3C/svg%3E'}}}}));
  await page.goto(`http://127.0.0.1:${server.httpServer.address().port}/tests/deck-loading-layout.html`, {waitUntil: 'domcontentloaded'});
  await page.locator('[data-deck-workspace]').waitFor();
  return {page, errors};
}

test('the featured strip is the last-major-events collection and assigns to the selected player', async () => {
  const server = await createServer({server: {host: '127.0.0.1', port: 0}, logLevel: 'silent'});
  await server.listen();
  const browser = await chromium.launch();
  try {
    const {page, errors} = await openWorkspace(server, browser);
    const workspace = page.locator('[data-deck-workspace]');
    assert.equal(await workspace.locator('[data-deck-catalog]').count(), 1);
    assert.equal(await workspace.locator('[data-player-tabs]').count(), 1);

    // Featured shows only decks tagged last-major-events, newest first.
    const featured = page.locator('[data-featured-decks] [data-featured-deck]');
    await featured.first().waitFor();
    assert.equal(await featured.count(), 3);
    assert.deepEqual(
      await featured.locator('.text-\\[13px\\]').allTextContents(),
      ['Boros Energy 4', 'Boros Energy 3', 'Boros Energy 2'],
    );

    // The collection chips narrow the list below to that same collection.
    const list = page.locator('[data-deck-catalog-list]');
    assert.equal(await list.locator('[data-deck-row]').count(), 12);
    await page.locator('[data-collection="last-major-events"]').click();
    await page.waitForFunction(() => document.querySelectorAll('[data-deck-row]').length === 4);
    assert.deepEqual(
      await list.locator('[data-deck-row]').evaluateAll((rows) => rows.map((row) => row.querySelector('div > div').textContent)),
      ['Boros Energy 4', 'Boros Energy 3', 'Boros Energy 2', 'Boros Energy 1'],
    );
    assert.equal(await page.locator('[data-collection="last-major-events"]').getAttribute('aria-pressed'), 'true');

    // The player tabs pick who a catalog deck is assigned to.
    await page.getByRole('button', {name: 'x4'}).click();
    const tabs = page.locator('[data-player-tab]');
    assert.equal(await tabs.count(), 4);
    await tabs.nth(2).click();
    assert.equal(await page.locator('[data-player-panel="2"]').count(), 1);
    await featured.first().getByRole('button', {name: 'Use'}).click();
    // Assigning moves the target on to the next player still without a deck,
    // so the panel follows and the filled player keeps a marker on its tab.
    await page.waitForFunction(() => document.querySelector('[data-player-tab="0"]')?.getAttribute('aria-pressed') === 'true');
    assert.equal(await page.locator('[data-player-panel="0"]').count(), 1);
    await tabs.nth(2).click();
    const charlieDeck = page.getByLabel('Charlie decklist');
    await charlieDeck.waitFor();
    assert.match(await charlieDeck.inputValue(), /60 Mountain/);
    assert.equal(await page.getByLabel('Alice decklist').count(), 0);

    if (globalThis.process?.env?.DECK_LOADING_SCREENSHOT) {
      await tabs.nth(2).click();
      await page.screenshot({path: globalThis.process.env.DECK_LOADING_SCREENSHOT});
    }
    assert.deepEqual(errors, []);
  } finally {
    await browser.close();
    await server.close();
  }
});

test('the list shows at least five decks and the panel keeps copy and clear at the top', async () => {
  const server = await createServer({server: {host: '127.0.0.1', port: 0}, logLevel: 'silent'});
  await server.listen();
  const browser = await chromium.launch();
  try {
    const {page, errors} = await openWorkspace(server, browser);
    await page.locator('[data-deck-row]').first().waitFor();

    // The catalog column owns most of the width now that one player deck shows
    // at a time, and the freed vertical space has to reach five decks.
    const [catalogWidth, panelWidth] = await Promise.all([
      page.locator('[data-deck-catalog]').evaluate((node) => node.getBoundingClientRect().width),
      page.locator('[data-player-tabs]').evaluate((node) => node.closest('div.flex-1').getBoundingClientRect().width),
    ]);
    assert.ok(catalogWidth / (catalogWidth + panelWidth) > 0.6, `catalog should take the larger share (${Math.round(catalogWidth)} vs ${Math.round(panelWidth)})`);

    const fullyVisibleRows = await page.locator('[data-deck-catalog-list]').evaluate((list) => {
      const bounds = list.getBoundingClientRect();
      return [...list.querySelectorAll('[data-deck-row]')]
        .filter((row) => {
          const box = row.getBoundingClientRect();
          return box.top >= bounds.top - 1 && box.bottom <= bounds.bottom + 1;
        }).length;
    });
    assert.ok(fullyVisibleRows >= 5, `expected at least 5 visible decks, saw ${fullyVisibleRows}`);

    // No assignment tabs; the deck list is always the panel's body, with copy
    // and clear above it.
    assert.equal(await page.locator('[data-assign-tab]').count(), 0);
    const panel = page.locator('[data-player-panel="0"]');
    const [copyTop, textareaTop] = await Promise.all([
      panel.getByRole('button', {name: /Copy Alice/}).evaluate((node) => node.getBoundingClientRect().top),
      panel.getByLabel('Alice decklist').evaluate((node) => node.getBoundingClientRect().top),
    ]);
    assert.ok(copyTop < textareaTop, 'copy should sit above the deck list');
    await assert.doesNotReject(panel.getByRole('button', {name: /Clear Alice/}).waitFor());

    // A deck names the card its art comes from; one without art still gets a
    // tile rather than an empty hole.
    await page.waitForFunction(() => document.querySelectorAll('[data-deck-art="loaded"]').length > 0);
    assert.ok(await page.locator('[data-deck-art="placeholder"]').count() > 0);

    await page.locator('[data-catalog-tab="saved"]').click();
    await assert.doesNotReject(page.getByText('You have no saved decks in this session.').waitFor());
    await page.locator('[data-catalog-tab="catalog"]').click();
    await page.locator('[data-featured-decks]').waitFor();
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
    const {page, errors} = await openWorkspace(server, browser, {locale: 'es'});
    for (const label of ['Explorar mazos', 'Mazos destacados', 'Jugadores / asignación de mazos', 'Todos los mazos', 'Mis mazos']) {
      await assert.doesNotReject(page.getByText(label, {exact: true}).first().waitFor(), label);
    }
    await assert.doesNotReject(page.getByPlaceholder('Buscar por nombre, arquetipo, carta, evento…').waitFor());
    assert.deepEqual(errors, []);
  } finally {
    await browser.close();
    await server.close();
  }
});
