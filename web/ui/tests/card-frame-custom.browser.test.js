import test from 'node:test';
import assert from 'node:assert/strict';
import {Buffer} from 'node:buffer';
import {deflateSync} from 'node:zlib';
import {dirname} from 'node:path';
import {fileURLToPath} from 'node:url';
import {chromium} from 'playwright';
import {createServer} from 'vite';

// A flat scan, so nothing at all can be detected in it: no mask can be made,
// and the card must fall back to its own frame around the art crop.
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
  name: 'Custom Frame Probe', layout: 'saga', frame: '2015', released_at: '2020-01-01',
  set: 'tst', collector_number: '1', border_color: 'black', colors: ['G'],
  type_line: 'Enchantment — Saga', oracle_text: 'Draw a card.\nYou gain 2 life.', mana_cost: '{1}{G}',
  power: null, toughness: null,
  image_uris: {
    normal: 'https://cards.scryfall.io/normal/front/0/f/0f9b2a3c-1d4e-4f5a-8b6c-7d8e9f0a1b2c.jpg',
    art_crop: 'https://cards.scryfall.io/art_crop/front/0/f/0f9b2a3c-1d4e-4f5a-8b6c-7d8e9f0a1b2c.jpg',
  },
};

async function openComparison(page, root, vite) {
  await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/card-frame-comparison.html`);
}

// Masking never runs for a Saga, and a flat scan yields no regions either, so
// this is the deepest fallback for a real printing: the card's own frame, in
// the card's color, with the art crop in the art box and the live text in
// the printing's own faces. The scan itself is never shown.
test('an unmasked printing renders the custom frame around its art crop', {timeout: 60000}, async () => {
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
    await openComparison(page, root, vite);
    const stage = page.locator('.interactive-card-frame-stage');
    await page.waitForFunction(() => document.querySelector('[data-render-ready="true"]'));

    assert.equal(await stage.getAttribute('data-frame-mode'), 'custom');
    assert.equal(await stage.getAttribute('data-frame-fallback-reason'), 'layout-saga');
    assert.equal(await stage.getAttribute('data-card-frame-tone'), 'green');
    assert.equal(await stage.getAttribute('data-card-frame-colors'), 'G');
    // The printing's geometry is not laid over anything: no scan, no containers.
    assert.equal(await stage.getAttribute('data-source-frame'), null);
    assert.equal(await stage.getAttribute('data-box-sizing'), null);
    assert.equal(await stage.locator('.original-card-fallback').count(), 0);
    assert.equal(await stage.locator('.original-card-details').count(), 0);

    const custom = await stage.evaluate(node => {
      const art = node.querySelector('.interactive-card-frame__art img');
      const artBox = node.querySelector('.interactive-card-frame__art').getBoundingClientRect();
      return {
        edge: node.style.getPropertyValue('--card-frame-edge'),
        printedLayout: node.style.getPropertyValue('--printed-layout'),
        frameBackground: getComputedStyle(node.querySelector('.interactive-card-frame')).backgroundImage,
        artSrc: art?.getAttribute('src'), artFit: art && getComputedStyle(art).objectFit,
        artFillsBox: art && Math.abs(art.getBoundingClientRect().width - artBox.width) < 1,
        titleBackground: getComputedStyle(node.querySelector('.interactive-card-frame__title-row')).backgroundColor,
        titleFont: getComputedStyle(node.querySelector('.interactive-card-frame__title')).fontFamily,
        rulesFont: getComputedStyle(node.querySelector('.interactive-card-frame__rule-line')).fontFamily,
        rulesText: [...node.querySelectorAll('.interactive-card-frame__rule-line')].map(line => line.textContent),
      };
    });
    assert.equal(custom.edge, '#3f7054', 'the frame is painted in the card\'s color');
    assert.equal(custom.printedLayout, '', 'no scan geometry reaches the custom frame');
    assert.ok(!custom.frameBackground.includes('/normal/'), `the full scan is not drawn: ${custom.frameBackground}`);
    assert.equal(custom.artSrc, printing.image_uris.art_crop);
    assert.equal(custom.artFit, 'cover');
    assert.ok(custom.artFillsBox, 'the art crop fills the art box');
    assert.notEqual(custom.titleBackground, 'rgb(8, 9, 11)', 'no opaque black container');
    // The faces stay the printing's.
    assert.match(custom.titleFont, /Beleren|Matrix|Goudy/);
    assert.match(custom.rulesFont, /MPlantin/);
    assert.deepEqual(custom.rulesText, ['Draw a card.', 'You gain 2 life.']);
    assert.deepEqual(errors, []);
  } finally { await browser.close(); await vite.close(); }
});

// A compiled custom card has no printing to look up at all: the frame must
// still appear, immediately and fully interactive, from the placeholder, and a
// two-color card splits its frame between both colors under gold bars.
test('a card with no art renders a live placeholder frame in its colors', {timeout: 60000}, async () => {
  const root = dirname(dirname(fileURLToPath(import.meta.url)));
  const vite = await createServer({root, server: {host: '127.0.0.1', port: 0}, logLevel: 'silent'});
  await vite.listen();
  const browser = await chromium.launch();
  try {
    const page = await browser.newPage({viewport: {width: 1800, height: 800}});
    const errors = []; page.on('pageerror', error => errors.push(error.message));
    await page.addInitScript(() => {
      window.__comparisonCards = [{
        id: 1, name: 'Zzz Compiled Probe', type_line: 'Creature — Elf Druid',
        oracle_text: 'Spells have split second.', mana_cost: '{G}', power: 1, toughness: 1,
      }, {
        id: 2, name: 'Zzz Two Color Probe', type_line: 'Creature — Wizard', colors: ['U', 'R'],
        oracle_text: 'Prowess', mana_cost: '{U}{R}', power: 2, toughness: 2,
      }, {
        id: 3, name: 'Zzz Devoid Probe', type_line: 'Creature — Eldrazi', colors: [],
        oracle_text: 'Devoid', mana_cost: '{2}{U}', power: 3, toughness: 3,
      }];
    });
    await page.route('https://cards.scryfall.io/**', route => route.abort());
    await page.route('https://api.scryfall.com/**', route => route.fulfill({status: 404, json: {}}));
    await page.route('https://svgs.scryfall.io/**', route => route.abort());
    await page.route('**/cards/*.json', route => route.fulfill({status: 404, json: {}}));
    await openComparison(page, root, vite);
    const stages = page.locator('.interactive-card-frame-stage');
    await page.waitForFunction(() => document.querySelectorAll('.interactive-card-frame-stage[data-render-ready="true"]').length === 3, null, {timeout: 30000});

    const single = stages.nth(0);
    assert.equal(await single.getAttribute('data-frame-mode'), 'placeholder');
    assert.equal(await single.getAttribute('data-card-frame-tone'), 'green');
    assert.equal(await single.locator('.original-card-fallback').count(), 0);
    assert.equal(await single.locator('.interactive-card-frame').count(), 1);
    assert.equal(await single.locator('.interactive-card-frame__art-fallback').count(), 1);
    assert.deepEqual(await single.locator('.interactive-card-frame__rule-line').allTextContents(), ['Spells have split second.']);
    assert.equal(await single.locator('.interactive-card-frame__title').textContent(), 'Zzz Compiled Probe');
    // The stage is only interactive once it is ready; a hidden one stays inert.
    assert.equal(await single.evaluate(node => node.inert), false);

    const dual = stages.nth(1);
    assert.equal(await dual.getAttribute('data-card-frame-tone'), 'gold');
    assert.equal(await dual.getAttribute('data-card-frame-colors'), 'UR');
    const split = await dual.evaluate(node => ({
      fill: node.style.getPropertyValue('--card-frame-edge-fill'),
      bar: node.style.getPropertyValue('--card-frame-bar'),
      inner: getComputedStyle(node.querySelector('.interactive-card-frame__inner')).backgroundImage,
    }));
    assert.match(split.fill, /#467995.*#8b473b/, 'blue on the left, red on the right');
    assert.equal(split.bar, '#b89b55', 'gold bars');
    assert.ok(split.inner.includes('rgb(70, 121, 149)') && split.inner.includes('rgb(139, 71, 59)'), `the inner frame shows both colors: ${split.inner}`);

    // The object's own colors win over its mana cost: devoid is colorless.
    const devoid = stages.nth(2);
    assert.equal(await devoid.getAttribute('data-card-frame-tone'), 'colorless');
    assert.equal(await devoid.getAttribute('data-card-frame-colors'), null);
    assert.deepEqual(errors, []);
  } finally { await browser.close(); await vite.close(); }
});

// Art the sampler can never read (a custom URL on another host) is still a
// printing when it is card-shaped: the custom frame shows its conventional art
// region in the art box rather than laying containers over the whole card.
test('an unsamplable card-shaped image is cropped to its art in the custom frame', {timeout: 60000}, async () => {
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
        id: 1, name: 'Zzz Foreign Scan', type_line: 'Artifact Creature — Golem', sourceImageUrl: url,
        oracle_text: 'Spells have split second.', mana_cost: '{3}', power: 1, toughness: 1,
      }];
    }, artUrl);
    // No Access-Control-Allow-Origin: the pixels are unreadable, exactly like a
    // player's own art host.
    await page.route('https://foreign.invalid/**', route => route.fulfill({contentType: 'image/png', body: flatPng(488, 680, [96, 104, 112])}));
    await page.route('https://cards.scryfall.io/**', route => route.abort());
    await page.route('https://api.scryfall.com/**', route => route.fulfill({status: 404, json: {}}));
    await page.route('https://svgs.scryfall.io/**', route => route.abort());
    await page.route('**/cards/*.json', route => route.fulfill({status: 404, json: {}}));
    await openComparison(page, root, vite);
    const stage = page.locator('.interactive-card-frame-stage');
    await page.waitForFunction(() => document.querySelector('.interactive-card-frame-stage[data-render-ready="true"]'), null, {timeout: 30000});

    assert.equal(await stage.getAttribute('data-frame-mode'), 'custom');
    assert.equal(await stage.getAttribute('data-frame-fallback-reason'), 'unsampled-art');
    assert.equal(await stage.getAttribute('data-art-source'), 'printing');
    assert.equal(await stage.getAttribute('data-card-frame-tone'), 'artifact');
    assert.equal(await stage.getAttribute('data-source-frame'), null);
    assert.equal(await stage.locator('.original-card-details').count(), 0);
    const crop = await stage.evaluate(node => {
      const art = node.querySelector('.interactive-card-frame__art img');
      const box = node.querySelector('.interactive-card-frame__art').getBoundingClientRect();
      const image = art.getBoundingClientRect();
      return {src: art.getAttribute('src'), scale: image.height / box.height, above: (box.top - image.top) / image.height};
    });
    assert.equal(crop.src, artUrl);
    // The whole card is 1/.4206 times taller than the art box, and the title
    // band above the art (12.65% of the card) sits outside the box.
    assert.ok(Math.abs(crop.scale - 1 / .4206) < .02, `card scaled to its art region: ${crop.scale}`);
    assert.ok(Math.abs(crop.above - .1265) < .01, `title band cropped away: ${crop.above}`);
    assert.deepEqual(await stage.locator('.interactive-card-frame__rule-line').allTextContents(), ['Spells have split second.']);
    assert.notEqual(await stage.evaluate(node => getComputedStyle(node.querySelector('.interactive-card-frame__title-row')).backgroundColor), 'rgb(8, 9, 11)');
    assert.deepEqual(errors, []);
  } finally { await browser.close(); await vite.close(); }
});
