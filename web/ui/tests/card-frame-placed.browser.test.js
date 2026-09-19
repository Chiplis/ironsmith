import test from 'node:test';
import assert from 'node:assert/strict';
import {Buffer} from 'node:buffer';
import {deflateSync} from 'node:zlib';
import {dirname} from 'node:path';
import {fileURLToPath} from 'node:url';
import {chromium} from 'playwright';
import {createServer} from 'vite';

// A flat scan, so nothing at all can be detected in it: the containers must
// still be placed, from the conventional proportions.
function flatPng(width, height, [red, green, blue]) {
  const stride = width * 3 + 1;
  const raw = Buffer.alloc(stride * height);
  for (let y = 0; y < height; y++) for (let x = 0; x < width; x++) {
    const at = y * stride + 1 + x * 3;
    raw[at] = red; raw[at + 1] = green; raw[at + 2] = blue;
  }
  const table = Array.from({length: 256}, (_, index) => {
    let value = index;
    for (let bit = 0; bit < 8; bit++) value = value & 1 ? 0xedb88320 ^ (value >>> 1) : value >>> 1;
    return value >>> 0;
  });
  const crc = bytes => {
    let value = 0xffffffff;
    for (const byte of bytes) value = table[(value ^ byte) & 0xff] ^ (value >>> 8);
    return (value ^ 0xffffffff) >>> 0;
  };
  const chunk = (type, data) => {
    const length = Buffer.alloc(4); length.writeUInt32BE(data.length);
    const body = Buffer.concat([Buffer.from(type, 'latin1'), data]);
    const checksum = Buffer.alloc(4); checksum.writeUInt32BE(crc(body));
    return Buffer.concat([length, body, checksum]);
  };
  const header = Buffer.alloc(13);
  header.writeUInt32BE(width, 0); header.writeUInt32BE(height, 4);
  header[8] = 8; header[9] = 2;
  return Buffer.concat([Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]),
    chunk('IHDR', header), chunk('IDAT', deflateSync(raw)), chunk('IEND', Buffer.alloc(0))]);
}

const printing = {
  id: '0f9b2a3c-1d4e-4f5a-8b6c-7d8e9f0a1b2c', oracle_id: '0f9b2a3c-1d4e-4f5a-8b6c-7d8e9f0a1b2c', lang: 'en',
  name: 'Placed Container Probe', layout: 'saga', frame: '2015', released_at: '2020-01-01',
  set: 'tst', collector_number: '1', border_color: 'black',
  type_line: 'Enchantment — Saga', oracle_text: 'Draw a card.\nYou gain 2 life.',
  power: null, toughness: null,
  image_uris: {
    normal: 'https://cards.scryfall.io/normal/front/0/f/0f9b2a3c-1d4e-4f5a-8b6c-7d8e9f0a1b2c.jpg',
    art_crop: 'https://cards.scryfall.io/art_crop/front/0/f/0f9b2a3c-1d4e-4f5a-8b6c-7d8e9f0a1b2c.jpg',
  },
};

// Masking never runs for a Saga, and a flat scan yields no regions either, so
// this exercises both fallbacks at once: placed containers over conventional
// proportions, carrying the live text in the printing's own faces.
test('an unmasked printing still carries placed containers in the card\'s own type', {timeout: 60000}, async () => {
  const root = dirname(dirname(fileURLToPath(import.meta.url)));
  const vite = await createServer({root, server: {host: '127.0.0.1', port: 0}, logLevel: 'silent'});
  await vite.listen();
  const browser = await chromium.launch();
  try {
    const page = await browser.newPage({viewport: {width: 900, height: 800}});
    const errors = []; page.on('pageerror', error => errors.push(error.message));
    await page.addInitScript(p => {
      window.__comparisonCards = [{...p, id: 1, sourceImageUrl: p.image_uris.art_crop, rules_text: p.oracle_text}];
    }, printing);
    await page.route('https://cards.scryfall.io/**', route => route.fulfill({
      contentType: 'image/png', headers: {'Access-Control-Allow-Origin': '*'},
      body: route.request().url().includes('/art_crop/') ? flatPng(488, 372, [104, 108, 112]) : flatPng(488, 680, [104, 108, 112]),
    }));
    await page.route('https://api.scryfall.com/**', route => route.fulfill({json: printing}));
    await page.route('https://svgs.scryfall.io/**', route => route.abort());
    await page.route('**/cards/*.json', route => route.fulfill({json: {scryfall: printing}}));
    await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/card-frame-comparison.html`);
    const stage = page.locator('.interactive-card-frame-stage');
    await page.waitForFunction(() => document.querySelector('[data-render-ready="true"]'));

    assert.equal(await stage.getAttribute('data-frame-mode'), 'placed');
    assert.equal(await stage.getAttribute('data-frame-fallback-reason'), 'layout-saga');
    assert.equal(await stage.locator('.original-card-fallback').count(), 0);
    // The containers hold the live text, so no second copy of it is stacked
    // over the frame.
    assert.equal(await stage.locator('.original-card-details').count(), 0);

    const placed = await stage.evaluate(node => {
      const frame = node.querySelector('.interactive-card-frame').getBoundingClientRect();
      const measure = selector => {
        const element = node.querySelector(selector);
        const box = element.getBoundingClientRect();
        const style = getComputedStyle(element);
        return {
          x: (box.x - frame.x) / frame.width, y: (box.y - frame.y) / frame.height,
          width: box.width / frame.width, height: box.height / frame.height,
          background: style.backgroundColor, backgroundImage: style.backgroundImage,
        };
      };
      return {
        layout: JSON.parse(node.style.getPropertyValue('--printed-layout')),
        source: node.style.getPropertyValue('--source-frame-image'),
        title: measure('.interactive-card-frame__title-row'),
        type: measure('.interactive-card-frame__type-row'),
        rules: measure('.interactive-card-frame__rules'),
        titleFont: getComputedStyle(node.querySelector('.interactive-card-frame__title')).fontFamily,
        rulesFont: getComputedStyle(node.querySelector('.interactive-card-frame__rule-line')).fontFamily,
        rulesText: [...node.querySelectorAll('.interactive-card-frame__rule-line')].map(line => line.textContent),
      };
    });

    // The printing itself is untouched behind the containers.
    assert.equal(placed.source, `url("${printing.image_uris.normal}")`);
    assert.deepEqual(Object.keys(placed.layout).sort(), ['art', 'rules', 'title', 'type']);
    for (const section of ['title', 'type', 'rules']) {
      assert.equal(placed[section].background, 'rgb(8, 9, 11)', `${section} container is opaque`);
      assert.equal(placed[section].backgroundImage, 'none', `${section} container hides the printed lettering`);
      const expected = placed.layout[section];
      assert.ok(Math.abs(placed[section].y - expected.y / 680) < .02, `${section} sits on its box: ${JSON.stringify(placed[section])}`);
      assert.ok(Math.abs(placed[section].width - expected.width / 488) < .02, `${section} spans its box`);
    }
    // Only the panel behind the text is ours; the faces stay the printing's.
    assert.match(placed.titleFont, /Beleren|Matrix|Goudy/);
    assert.match(placed.rulesFont, /MPlantin/);
    assert.deepEqual(placed.rulesText, ['Draw a card.', 'You gain 2 life.']);
    assert.deepEqual(errors, []);
  } finally { await browser.close(); await vite.close(); }
});

// A compiled custom card has no printing to look up at all: the frame must
// still appear, immediately and fully interactive, from the placeholder.
test('a card with no art renders a live placeholder frame instead of an invisible stage', {timeout: 60000}, async () => {
  const root = dirname(dirname(fileURLToPath(import.meta.url)));
  const vite = await createServer({root, server: {host: '127.0.0.1', port: 0}, logLevel: 'silent'});
  await vite.listen();
  const browser = await chromium.launch();
  try {
    const page = await browser.newPage({viewport: {width: 900, height: 800}});
    const errors = []; page.on('pageerror', error => errors.push(error.message));
    await page.addInitScript(() => {
      window.__comparisonCards = [{
        id: 1, name: 'Zzz Compiled Probe', type_line: 'Creature — Elf Druid',
        oracle_text: 'Spells have split second.', mana_cost: '{G}', power: 1, toughness: 1,
      }];
    });
    await page.route('https://cards.scryfall.io/**', route => route.abort());
    await page.route('https://api.scryfall.com/**', route => route.fulfill({status: 404, json: {}}));
    await page.route('https://svgs.scryfall.io/**', route => route.abort());
    await page.route('**/cards/*.json', route => route.fulfill({status: 404, json: {}}));
    await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/card-frame-comparison.html`);
    const stage = page.locator('.interactive-card-frame-stage');
    await page.waitForFunction(() => document.querySelector('.interactive-card-frame-stage[data-render-ready="true"]'), null, {timeout: 30000});

    assert.equal(await stage.getAttribute('data-frame-mode'), 'placeholder');
    assert.equal(await stage.locator('.original-card-fallback').count(), 0);
    assert.equal(await stage.locator('.interactive-card-frame').count(), 1);
    assert.equal(await stage.locator('.interactive-card-frame__art-fallback').count(), 1);
    assert.deepEqual(await stage.locator('.interactive-card-frame__rule-line').allTextContents(), ['Spells have split second.']);
    assert.equal(await stage.locator('.interactive-card-frame__title').textContent(), 'Zzz Compiled Probe');
    // The stage is only interactive once it is ready; a hidden one stays inert.
    assert.equal(await stage.evaluate(node => node.inert), false);
    assert.deepEqual(errors, []);
  } finally { await browser.close(); await vite.close(); }
});

// Art the sampler can never read (a custom URL on another host) is still a
// printing when it is card-shaped: the containers go over it at conventional
// proportions rather than dropping the frame.
test('an unsamplable card-shaped image still carries placed containers', {timeout: 60000}, async () => {
  const root = dirname(dirname(fileURLToPath(import.meta.url)));
  const vite = await createServer({root, server: {host: '127.0.0.1', port: 0}, logLevel: 'silent'});
  await vite.listen();
  const browser = await chromium.launch();
  try {
    const page = await browser.newPage({viewport: {width: 900, height: 800}});
    const errors = []; page.on('pageerror', error => errors.push(error.message));
    const artUrl = 'https://foreign.invalid/custom-card.png';
    await page.addInitScript(url => {
      window.__comparisonCards = [{
        id: 1, name: 'Zzz Foreign Scan', type_line: 'Creature — Elf Druid', sourceImageUrl: url,
        oracle_text: 'Spells have split second.', mana_cost: '{G}', power: 1, toughness: 1,
      }];
    }, artUrl);
    // No Access-Control-Allow-Origin: the pixels are unreadable, exactly like a
    // player's own art host.
    await page.route('https://foreign.invalid/**', route => route.fulfill({contentType: 'image/png', body: flatPng(488, 680, [96, 104, 112])}));
    await page.route('https://cards.scryfall.io/**', route => route.abort());
    await page.route('https://api.scryfall.com/**', route => route.fulfill({status: 404, json: {}}));
    await page.route('https://svgs.scryfall.io/**', route => route.abort());
    await page.route('**/cards/*.json', route => route.fulfill({status: 404, json: {}}));
    await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/card-frame-comparison.html`);
    const stage = page.locator('.interactive-card-frame-stage');
    await page.waitForFunction(() => document.querySelector('.interactive-card-frame-stage[data-render-ready="true"]'), null, {timeout: 30000});

    assert.equal(await stage.getAttribute('data-frame-mode'), 'placed');
    assert.equal(await stage.getAttribute('data-frame-fallback-reason'), 'unsampled-art');
    assert.equal(await stage.evaluate(node => node.style.getPropertyValue('--source-frame-image')), `url("${artUrl}")`);
    const boxes = await stage.evaluate(node => JSON.parse(node.style.getPropertyValue('--printed-layout')));
    assert.deepEqual(Object.keys(boxes).sort(), ['art', 'rules', 'title', 'type']);
    assert.equal(await stage.locator('.original-card-details').count(), 0);
    assert.deepEqual(await stage.locator('.interactive-card-frame__rule-line').allTextContents(), ['Spells have split second.']);
    assert.equal(await stage.evaluate(node => getComputedStyle(node.querySelector('.interactive-card-frame__title-row')).backgroundColor), 'rgb(8, 9, 11)');
    assert.deepEqual(errors, []);
  } finally { await browser.close(); await vite.close(); }
});
