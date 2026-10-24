import test from 'node:test';
import assert from 'node:assert/strict';
import {chromium} from 'playwright';
import {createServer} from 'vite';
import {parseSideboardList} from '../src/lib/decklists.js';

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
  collections: index < 3 ? ['mono-color'] : [],
  cards: ['Mountain'],
  cardNames: ['Mountain'],
  artCard: 'Atraxa',
  manaProfile: {
    colors: index < 3 ? ['R'] : ['R', 'W'],
    landCount: 24,
    predominantColors: ['R'],
    predominantLands: [{name: 'Mountain', count: 8}],
    metadataCoverage: {complete: true},
  },
  detail: `details/deck-${index}.json`,
}));
const index = {schemaVersion: 1, format: 'modern', generatedAt: '2026-09-18T00:00:00Z', decks};

test('Commander lobbies offer all 500 decks and apply partners separately', async () => {
  const server = await createServer({server: {host: '127.0.0.1', port: 0}, logLevel: 'silent'});
  await server.listen();
  const browser = await chromium.launch();
  try {
    const page = await browser.newPage({viewport: {width: 1000, height: 800}});
    const commanderDecks = Array.from({length: 500}, (_, i) => ({ ...decks[0], id: `commander-${i}`, format: 'commander', name: `Partners ${i}`, detail: `details/commander-${i}.json` }));
    await page.route('**/catalog/commander/index.json', (route) => route.fulfill({json: { ...index, format: 'commander', decks: commanderDecks }}));
    await page.route('**/catalog/commander/search-index.json', (route) => route.fulfill({status: 404, body: ''}));
    await page.route('**/catalog/commander/details/*.json', (route) => route.fulfill({json: {
      id: 'commander-0', format: 'commander', name: 'Partners', mainboard: [{name: 'Island', count: 98}],
      commander: [{name: "Kraum, Ludevic's Opus", count: 1}, {name: 'Tymna the Weaver', count: 1}], sideboard: [],
    }}));
    await page.route('**/cards/*.json', (route) => route.fulfill({status: 404, body: ''}));
    await page.goto(`http://127.0.0.1:${server.httpServer.address().port}/tests/lobby-deck-picker.html?format=commander`, {waitUntil: 'domcontentloaded'});
    await page.locator('[data-deck-row]').first().waitFor();
    await page.waitForFunction(() => document.querySelectorAll('[data-deck-row]').length === 500);
    assert.equal(await page.locator('[data-deck-row]').count(), 500);
    const formats = await page.locator('select option').evaluateAll((options) => options.map((option) => option.value));
    for (const format of ['commander', 'standard', 'vintage', 'legacy', 'pauper', 'pioneer', 'modern']) assert.ok(formats.includes(format));
    await page.locator('[data-deck-row]').first().getByRole('button', {name: 'Use'}).click();
    await page.waitForFunction(() => Boolean(window.__applied));
    const applied = await page.evaluate(() => window.__applied);
    assert.match(applied.deckText, /98 Island/);
    assert.match(applied.commanderText, /1 Tymna the Weaver/);
    assert.match(applied.commanderText, /1 Kraum, Ludevic's Opus/);
  } finally {
    await browser.close();
    await server.close();
  }
});

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

    const rows = page.locator('[data-deck-row]');
    await rows.first().waitFor();
    await page.waitForFunction(() => document.querySelectorAll('[data-deck-row]').length === 6);
    assert.equal(await rows.count(), 6);

    // Every deck is offered as art plus a single Use, like the workspace.
    await page.waitForFunction(() => document.querySelectorAll('[data-deck-row] [data-deck-art="loaded"]').length === 6);
    assert.equal(await page.locator('[data-deck-row] button').count(), 6);

    await page.locator('[data-catalog-tab="mono-color"]').click();
    assert.equal(await rows.count(), 3);
    await page.locator('[data-catalog-tab="all"]').click();
    await page.getByRole('button', {name: 'Filter by W mana', exact: true}).click();
    assert.equal(await rows.count(), 3);
    await page.getByRole('button', {name: 'Only these', exact: true}).click();
    assert.equal(await rows.count(), 0);
    await page.getByRole('button', {name: 'Clear', exact: true}).click();
    await page.waitForFunction(() => document.querySelectorAll('[data-deck-row]').length === 6);
    assert.equal(await rows.count(), 6);

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

test('lobby sections exclude illegal main decks and clean sideboards when using or copying decks', async () => {
  const server = await createServer({server: {host: '127.0.0.1', port: 0}, logLevel: 'silent'});
  await server.listen();
  const browser = await chromium.launch();
  try {
    const page = await browser.newPage();
    await page.addInitScript(() => {
      Object.defineProperty(navigator, 'clipboard', {value: {writeText: async text => { window.__copied = text; }}, configurable: true});
      localStorage.setItem('ironsmith.savedDeckPresets.v3', JSON.stringify([
        {name: 'Sideboard saved', texts: ['60 Mountain\nSideboard\n1 Black Lotus\n1 Island']},
        {name: 'Legal saved', texts: ['60 Mountain']},
        {name: 'Illegal saved', texts: ['59 Mountain\n1 Black Lotus']},
        {name: 'Too small', texts: ['10 Mountain']},
      ]));
      localStorage.setItem('ironsmith.playedDecks.v1', JSON.stringify([
        {id: 'bad-recent', name: 'Illegal recent', deckText: '60 Mountain\nSideboard\n1 Black Lotus'},
        {id: 'good-recent', name: 'Legal recent', deckText: '60 Mountain'},
      ]));
    });
    const entries = decks.slice(0, 3).map(entry => ({...entry, collections: ['mono-color', 'last-major-events', 'last-20-events']}));
    await page.route('**/catalog/modern/index.json', route => route.fulfill({json: {...index, decks: entries}}));
    await page.route('**/catalog/modern/search-index.json', route => route.fulfill({status: 404, body: ''}));
    await page.route('**/catalog/modern/details/*.json', route => {
      const id = route.request().url().match(/deck-(\d+)/)[1];
      return route.fulfill({json: {
        mainboard: id === '2' ? [{name: 'Mountain', count: 55}, {name: 'Lightning Bolt', count: 5}] : [{name: 'Mountain', count: 60}],
        sideboard: id === '1' ? [{name: 'Black Lotus', count: 1}] : [], commander: [],
      }});
    });
    await page.route('**/cards/*.json', route => route.fulfill({status: 404, body: ''}));
    await page.goto(`http://127.0.0.1:${server.httpServer.address().port}/tests/lobby-deck-picker.html`, {waitUntil: 'domcontentloaded'});
    await page.locator('[data-deck-row="deck-0"]').waitFor();
    for (const tab of ['all', 'last-major-events', 'last-20-events', 'mono-color']) {
      await page.locator(`[data-catalog-tab="${tab}"]`).click();
      assert.deepEqual(await page.locator('[data-deck-row]').evaluateAll(rows => rows.map(row => row.dataset.deckRow)), ['deck-1', 'deck-0']);
    }
    assert.equal(await page.locator('[data-featured-deck="bad-recent"]').count(), 1);
    assert.equal(await page.locator('[data-featured-deck="good-recent"]').count(), 1);
    assert.equal(await page.locator('[data-deck-row="deck-0"] [data-sideboard-notice]').count(), 0);
    for (const selector of ['[data-deck-row="deck-1"]', '[data-featured-deck="bad-recent"]']) {
      const notice = page.locator(`${selector} [data-sideboard-notice]`);
      assert.match(await notice.locator('summary').innerText(), /Sideboard adjusted · 1 removed/);
      await notice.locator('summary').click();
      assert.match(await notice.locator('li').innerText(), /1 × Black Lotus.*not legal in Modern/);
      assert.equal(await notice.locator('li').isVisible(), true);
    }
    await page.locator('[data-deck-row="deck-1"]').getByRole('button', {name: 'Use'}).click();
    assert.doesNotMatch(await page.evaluate(() => window.__applied.deckText), /Black Lotus/);
    await page.locator('[data-featured-deck="bad-recent"]').getByRole('button', {name: 'Use', exact: true}).click();
    assert.doesNotMatch(await page.evaluate(() => window.__applied.deckText), /Black Lotus/);
    await page.locator('[data-catalog-tab="saved"]').click();
    assert.deepEqual(await page.locator('[data-saved-deck]').evaluateAll(rows => rows.map(row => row.dataset.savedDeck)), ['Legal saved:0', 'Sideboard saved:0']);
    const savedNotice = page.locator('[data-saved-deck="Sideboard saved:0"] [data-sideboard-notice]');
    // Native disclosure works with the keyboard as well as touch/click.
    await savedNotice.locator('summary').focus();
    await page.keyboard.press('Enter');
    assert.equal(await savedNotice.locator('li').isVisible(), true);
    assert.match(await savedNotice.locator('li').innerText(), /Black Lotus.*not legal in Modern/);
    await page.locator('[data-saved-deck="Sideboard saved:0"]').getByRole('button', {name: 'Use', exact: true}).click();
    assert.doesNotMatch(await page.evaluate(() => window.__applied.deckText), /Black Lotus/);
    assert.match(await page.evaluate(() => window.__applied.deckText), /1 Island/);
    await page.locator('[data-featured-deck="bad-recent"]').getByRole('button', {name: 'Copy MTGO'}).click();
    assert.doesNotMatch(await page.evaluate(() => window.__copied), /Black Lotus/);
    assert.match(await page.evaluate(() => window.__copied), /60 Mountain/);
  } finally {
    await browser.close();
    await server.close();
  }
});

test('format and unreleased changes never commit stale decks, including during pending requests', async () => {
  const server = await createServer({server: {host: '127.0.0.1', port: 0}, logLevel: 'silent'});
  await server.listen();
  const browser = await chromium.launch();
  let releaseModern;
  let releasePioneer;
  let markModernStarted;
  let markPioneerStarted;
  const modernStarted = new Promise(resolve => { markModernStarted = resolve; });
  const pioneerStarted = new Promise(resolve => { markPioneerStarted = resolve; });
  const modernGate = new Promise(resolve => { releaseModern = resolve; });
  const pioneerGate = new Promise(resolve => { releasePioneer = resolve; });
  try {
    const page = await browser.newPage();
    await page.addInitScript(() => {
      // Exercise the loader's fetch fallback with a deterministic legality snapshot.
      window.Worker = undefined;
      const entries = [
        {id: 'base', deckText: '60 Mountain'},
        {id: 'modern-only', deckText: '59 Mountain\n1 Lightning Bolt'},
        {id: 'preview', deckText: '59 Mountain\n1 Preview Card'},
      ];
      localStorage.setItem('ironsmith.savedDeckPresets.v3', JSON.stringify(entries.map(entry => ({name: entry.id, texts: [entry.deckText]}))));
      localStorage.setItem('ironsmith.playedDecks.v1', JSON.stringify(entries.map(entry => ({...entry, name: entry.id}))));
      window.__illegalCommits = [];
      new MutationObserver(() => {
        const state = document.querySelector('[data-format-state]');
        if (!state) return;
        const ids = [...document.querySelectorAll('[data-deck-row], [data-featured-deck], [data-saved-deck]')]
          .map(node => node.dataset.deckRow || node.dataset.featuredDeck || node.dataset.savedDeck);
        if (ids.some(id => (id.includes('preview') && state.dataset.unreleasedState !== 'true')
          || (id.includes('modern-only') && state.dataset.formatState !== 'modern'))) {
          window.__illegalCommits.push({format: state.dataset.formatState, allow: state.dataset.unreleasedState, ids});
        }
      }).observe(document, {subtree: true, childList: true, attributes: true});
    });
    await page.route('**/format-catalog.generated.json*', route => route.fulfill({json: {
      sourceUpdatedAt: '2026-10-24', cards: {
        mountain: {name: 'Mountain', type: 'Basic Land — Mountain', rules: '', colors: [], legalities: {modern: 'legal', pioneer: 'legal'}},
        'lightning bolt': {name: 'Lightning Bolt', type: 'Instant', rules: '', colors: ['R'], legalities: {modern: 'legal', pioneer: 'not_legal'}},
        'preview card': {name: 'Preview Card', type: 'Creature', rules: '', colors: ['R'], unreleased: true, legalities: {modern: 'not_legal', pioneer: 'not_legal'}},
      },
    }}));
    await page.route('**/catalog/*/index.json', route => {
      const format = route.request().url().includes('/modern/') ? 'modern' : 'pioneer';
      return route.fulfill({json: {format, decks: ['base', 'modern-only', 'preview'].map(id => ({
        id, name: id, format, detail: `details/${id}.json`, collections: ['mono-color', 'last-major-events', 'last-20-events'],
      }))}});
    });
    await page.route('**/catalog/*/search-index.json', route => route.fulfill({status: 404, body: ''}));
    await page.route('**/catalog/*/details/*.json', async route => {
      const isModern = route.request().url().includes('/modern/');
      (isModern ? markModernStarted : markPioneerStarted)();
      await (isModern ? modernGate : pioneerGate);
      const id = route.request().url().split('/').pop().replace('.json', '');
      return route.fulfill({json: {mainboard: [
        {name: 'Mountain', count: id === 'base' ? 60 : 59},
        ...(id === 'base' ? [] : [{name: id === 'preview' ? 'Preview Card' : 'Lightning Bolt', count: 1}]),
      ]}});
    });
    await page.route('**/cards/*.json', route => route.fulfill({status: 404, body: ''}));
    await page.goto(`http://127.0.0.1:${server.httpServer.address().port}/tests/lobby-deck-picker.html?changing-format`, {waitUntil: 'domcontentloaded'});
    await modernStarted;
    const toggle = page.getByRole('checkbox', {name: 'Allow unreleased cards'});
    const format = page.getByRole('combobox', {name: 'Lobby format', exact: true});
    await toggle.check();
    await toggle.uncheck();
    assert.equal(await page.locator('[data-deck-row]').count(), 0);
    releaseModern();
    await page.locator('[data-deck-row="modern-only"]').waitFor();
    assert.equal(await page.locator('[data-deck-row="preview"]').count(), 0);
    await toggle.check();
    await page.locator('[data-deck-row="preview"]').waitFor();
    for (const tab of ['all', 'last-major-events', 'last-20-events', 'mono-color', 'saved']) {
      await page.locator(`[data-catalog-tab="${tab}"]`).click();
      const preview = page.locator(tab === 'saved' ? '[data-saved-deck="preview:0"]' : '[data-deck-row="preview"]');
      await preview.waitFor();
      await toggle.uncheck();
      await preview.waitFor({state: 'detached'});
      await toggle.check();
      await preview.waitFor();
    }
    // Leave a format request pending, change its rules, then return to a cached format.
    await format.selectOption('pioneer');
    await pioneerStarted;
    await toggle.uncheck();
    assert.equal(await page.locator('[data-deck-row]').count(), 0);
    await format.selectOption('modern');
    await page.locator('[data-deck-row="modern-only"]').waitFor();
    const lateResponses = Promise.all(['base', 'modern-only', 'preview'].map(id =>
      page.waitForResponse(response => response.url().endsWith(`/catalog/pioneer/details/${id}.json`))));
    releasePioneer();
    await lateResponses;
    await page.locator('[data-deck-row="modern-only"]').waitFor();
    await format.selectOption('pioneer');
    await page.locator('[data-deck-row="base"]').waitFor();
    assert.deepEqual(await page.locator('[data-deck-row]').evaluateAll(rows => rows.map(row => row.dataset.deckRow)), ['base']);
    await page.locator('[data-catalog-tab="saved"]').click();
    await page.locator('[data-saved-deck="base:0"]').waitFor();
    assert.equal(await page.locator('[data-saved-deck]').count(), 1);
    await toggle.check();
    await page.locator('[data-saved-deck="preview:0"]').waitFor();
    await toggle.uncheck();
    await page.locator('[data-saved-deck="preview:0"]').waitFor({state: 'detached'});
    assert.deepEqual(await page.evaluate(() => window.__illegalCommits), []);
  } finally {
    releaseModern();
    releasePioneer();
    await browser.close();
    await server.close();
  }
});

test('changing rules restores original sideboards and notices for catalog, saved, and recent decks', async () => {
  const server = await createServer({server: {host: '127.0.0.1', port: 0}, logLevel: 'silent'});
  await server.listen();
  const browser = await chromium.launch();
  const original = '60 Mountain\nSideboard\n2 Lightning Bolt\n1 Preview Card\n1 Island';
  try {
    const page = await browser.newPage();
    await page.addInitScript(deckText => {
      window.Worker = undefined;
      Object.defineProperty(navigator, 'clipboard', {value: {writeText: async text => { window.__copied = text; }}, configurable: true});
      localStorage.setItem('ironsmith.savedDeckPresets.v3', JSON.stringify([{name: 'Original saved', texts: [deckText]}]));
      localStorage.setItem('ironsmith.playedDecks.v1', JSON.stringify([{id: 'original-recent', name: 'Original recent', deckText}]));
      window.__staleNotices = [];
      new MutationObserver(() => {
        const state = document.querySelector('[data-format-state]');
        if (!state) return;
        const expected = (state.dataset.formatState === 'pioneer' ? 2 : 0) + (state.dataset.unreleasedState === 'true' ? 0 : 1);
        for (const row of document.querySelectorAll('[data-deck-row], [data-featured-deck], [data-saved-deck]')) {
          const summary = row.querySelector('[data-sideboard-notice] summary')?.textContent || '';
          if (summary !== (expected ? `Sideboard adjusted · ${expected} removed` : '')) {
            window.__staleNotices.push({expected, summary});
          }
        }
      }).observe(document, {subtree: true, childList: true, attributes: true, characterData: true});
    }, original);
    const common = {type: 'Basic Land', rules: '', colors: [], legalities: {modern: 'legal', pioneer: 'legal'}};
    await page.route('**/format-catalog.generated.json*', route => route.fulfill({json: {
      sourceUpdatedAt: '2026-10-24', cards: {
        mountain: {...common, name: 'Mountain'}, island: {...common, name: 'Island'},
        'lightning bolt': {...common, name: 'Lightning Bolt', type: 'Instant', legalities: {modern: 'legal', pioneer: 'not_legal'}},
        'preview card': {...common, name: 'Preview Card', type: 'Creature', unreleased: true, legalities: {}},
      },
    }}));
    await page.route('**/catalog/*/index.json', route => route.fulfill({json: {decks: [{id: 'original-catalog', name: 'Original catalog', detail: 'details/original.json'}]}}));
    await page.route('**/catalog/*/search-index.json', route => route.fulfill({status: 404, body: ''}));
    await page.route('**/catalog/*/details/original.json', route => route.fulfill({json: {
      mainboard: [{name: 'Mountain', count: 60}],
      sideboard: [{name: 'Lightning Bolt', count: 2}, {name: 'Preview Card', count: 1}, {name: 'Island', count: 1}],
    }}));
    await page.route('**/cards/*.json', route => route.fulfill({status: 404, body: ''}));
    await page.goto(`http://127.0.0.1:${server.httpServer.address().port}/tests/lobby-deck-picker.html?changing-format`, {waitUntil: 'domcontentloaded'});
    for (const [format, allow] of [['modern', false], ['pioneer', false], ['pioneer', true], ['modern', true], ['modern', false], ['modern', true]]) {
      await page.getByRole('combobox', {name: 'Lobby format', exact: true}).selectOption(format);
      await page.getByRole('checkbox', {name: 'Allow unreleased cards'}).setChecked(allow);
      const expectedSideboard = [...(format === 'modern' ? ['Lightning Bolt', 'Lightning Bolt'] : []), ...(allow ? ['Preview Card'] : []), 'Island'];
      const removed = 4 - expectedSideboard.length;
      await page.locator('[data-catalog-tab="all"]').click();
      for (const selector of ['[data-deck-row="original-catalog"]', '[data-featured-deck="original-recent"]', '[data-saved-deck="Original saved:0"]']) {
        if (selector.includes('data-saved-deck')) await page.locator('[data-catalog-tab="saved"]').click();
        const row = page.locator(selector);
        await row.waitFor();
        const notice = row.locator('[data-sideboard-notice]');
        if (removed) {
          assert.equal(await notice.locator('summary').innerText(), `Sideboard adjusted · ${removed} removed`);
          const reasons = (await notice.locator('li').allTextContents()).join(' ');
          if (format === 'pioneer') assert.match(reasons, /2 × Lightning Bolt.*not legal in Pioneer/);
          else assert.doesNotMatch(reasons, /Lightning Bolt/);
          if (!allow) assert.match(reasons, /1 × Preview Card.*unreleased/);
          else assert.doesNotMatch(reasons, /Preview Card/);
        } else assert.equal(await notice.count(), 0);
        await row.getByRole('button', {name: 'Use', exact: true}).click();
        assert.deepEqual(parseSideboardList(await page.evaluate(() => window.__applied.deckText)), expectedSideboard);
      }
      await page.locator('[data-featured-deck="original-recent"]').locator('button[title="Copy the list in MTGO format"]').click();
      assert.deepEqual(parseSideboardList(await page.evaluate(() => window.__copied)), expectedSideboard);
    }
    assert.deepEqual(await page.evaluate(() => window.__staleNotices), []);
    assert.equal(await page.evaluate(() => JSON.parse(localStorage.getItem('ironsmith.playedDecks.v1'))[0].deckText), original);
  } finally {
    await browser.close();
    await server.close();
  }
});

test('validation streams an ordered prefix and reprioritizes when sorting changes', async () => {
  const server = await createServer({server: {host: '127.0.0.1', port: 0}, logLevel: 'silent'});
  await server.listen();
  const browser = await chromium.launch();
  const releases = new Map();
  const gates = new Map(Array.from({length: 5}, (_, i) => [i, new Promise(resolve => releases.set(i, resolve))]));
  const requests = [];
  try {
    const page = await browser.newPage();
    const entries = [4, 1, 3, 0, 2].map(i => ({...decks[0], id: `stream-${i}`, name: `Deck ${i}`, date: `2026-10-${20 - i}`, placement: 5 - i, detail: `details/stream-${i}.json`}));
    await page.route('**/catalog/modern/index.json', route => route.fulfill({json: {...index, decks: entries}}));
    await page.route('**/catalog/modern/search-index.json', route => route.fulfill({status: 404, body: ''}));
    await page.route('**/catalog/modern/details/*.json', async route => {
      const i = Number(route.request().url().match(/stream-(\d+)/)[1]);
      requests.push(i);
      await gates.get(i);
      return route.fulfill({json: {mainboard: [{name: 'Mountain', count: i === 2 ? 10 : 60}]}});
    });
    await page.route('**/cards/*.json', route => route.fulfill({status: 404, body: ''}));
    await page.goto(`http://127.0.0.1:${server.httpServer.address().port}/tests/lobby-deck-picker.html`, {waitUntil: 'domcontentloaded'});
    const ids = () => page.locator('[data-deck-row]').evaluateAll(rows => rows.map(row => row.dataset.deckRow));
    const release = async i => {
      const response = page.waitForResponse(r => r.url().endsWith(`/details/stream-${i}.json`));
      releases.get(i)();
      await response;
    };
    // A later response cannot appear ahead of the first row.
    await release(1);
    assert.deepEqual(await ids(), []);
    await release(0);
    await page.locator('[data-deck-row="stream-1"]').waitFor();
    assert.deepEqual(await ids(), ['stream-0', 'stream-1']);
    assert.deepEqual(requests, [0, 1, 2, 3, 4]);
    assert.match(await page.getByRole('status').innerText(), /Checking decks/);
    const before = await page.locator('[data-deck-row="stream-0"]').boundingBox();
    const featuredBefore = await page.locator('[data-featured-deck]').evaluateAll(rows => rows.map(row => row.dataset.featuredDeck));
    await release(3);
    assert.deepEqual(await ids(), ['stream-0', 'stream-1']);
    // An invalid row is skipped, and the already-complete next row appends.
    await release(2);
    await page.locator('[data-deck-row="stream-3"]').waitFor();
    assert.deepEqual(await ids(), ['stream-0', 'stream-1', 'stream-3']);
    assert.equal((await page.locator('[data-deck-row="stream-0"]').boundingBox()).y, before.y);
    assert.deepEqual((await page.locator('[data-featured-deck]').evaluateAll(rows => rows.map(row => row.dataset.featuredDeck))).slice(0, 2), featuredBefore);
    // The new first row is still pending: none of the old order may leak in.
    await page.getByRole('combobox', {name: 'Sort', exact: true}).selectOption('placement');
    assert.deepEqual(await ids(), []);
    await release(4);
    await page.locator('[data-deck-row="stream-0"]').waitFor();
    assert.deepEqual(await ids(), ['stream-4', 'stream-3', 'stream-1', 'stream-0']);
    await page.getByRole('combobox', {name: 'Sort', exact: true}).selectOption('usage');
    await page.locator('[data-deck-row="stream-4"]').waitFor();
    assert.deepEqual(await ids(), ['stream-0', 'stream-1', 'stream-3', 'stream-4']);
  } finally {
    for (const resolve of releases.values()) resolve();
    await browser.close();
    await server.close();
  }
});
