import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {chromium} from 'playwright';
import {createServer} from 'vite';
import printing from './fixtures/frame-mask/thundering-giant.mjs';

const fixture = name => readFileSync(new URL(`./fixtures/frame-mask/${name}`, import.meta.url));

test('rounded panels mask complete initial letters and P/T never paints a scrollbar', {timeout: 90000}, async () => {
  const vite = await createServer({server: {host: '127.0.0.1', port: 0}, logLevel: 'silent'});
  await vite.listen();
  const browser = await chromium.launch();
  try {
    const page = await browser.newPage();
    await page.addInitScript(p => {
      window.__comparisonCards = [{...p, id: 1, sourceImageUrl: p.image_uris.art_crop, rules_text: 'Haste', power: 4, toughness: 3}];
    }, printing);
    await page.route('https://cards.scryfall.io/**', route => route.fulfill({
      contentType: 'image/jpeg', headers: {'Access-Control-Allow-Origin': '*'},
      body: fixture(`thundering-giant-${route.request().url().includes('/art_crop/') ? 'art_crop' : 'normal'}.jpg`),
    }));
    await page.route('https://api.scryfall.com/**', route => route.fulfill({
      json: route.request().url().includes('/sets/') ? {icon_svg_uri: 'https://svgs.scryfall.io/sets/w17.svg'} : printing,
    }));
    await page.route('https://svgs.scryfall.io/**', route => route.fulfill({
      contentType: 'image/svg+xml', headers: {'Access-Control-Allow-Origin': '*'}, body: fixture('w17.svg'),
    }));
    await page.route('**/cards/*.json', route => route.fulfill({json: {scryfall: printing}}));
    await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/card-frame-comparison.html`);
    await page.waitForFunction(() => document.querySelector('[data-render-ready="true"]'));
    const stats = await page.locator('.interactive-card-frame__stats-text').evaluate(node => ({
      overflow: getComputedStyle(node).overflow,
      height: node.clientHeight, inkHeight: node.scrollHeight,
    }));
    assert.equal(stats.overflow, 'visible', JSON.stringify(stats));
    await page.locator('[data-comparison-card]').screenshot({path: '/private/tmp/giant-render.png'});

    const result = await page.evaluate(async () => {
      const {sampleCardFramePixels} = await import('/src/lib/card-frame-colors.js');
      const {cardTypography} = await import('/src/lib/card-typography.js');
      const {manaTemplates} = await import('/src/lib/card-mana-match.js');
      const {default: printing} = await import('/tests/fixtures/frame-mask/thundering-giant.mjs');
      const typography = cardTypography(printing);
      await Promise.all(['title', 'type', 'rules', 'stats'].map(name => document.fonts.load(`${name === 'rules' ? 400 : 700} 100px ${typography[name]}`)));
      await document.fonts.load(`italic 400 100px ${typography.rules}`);
      const read = async url => {
        const image = new Image(); image.src = url; await image.decode();
        const canvas = document.createElement('canvas'); canvas.width = image.width; canvas.height = image.height;
        const context = canvas.getContext('2d'); context.drawImage(image, 0, 0); return context;
      };
      const scan = await read('/tests/fixtures/frame-mask/thundering-giant-normal.jpg');
      const art = await read('/tests/fixtures/frame-mask/thundering-giant-art_crop.jpg');
      const symbol = await read('/tests/fixtures/frame-mask/w17.svg');
      const pixels = context => context.getImageData(0, 0, context.canvas.width, context.canvas.height);
      const style = await sampleCardFramePixels({
        fullScan: pixels(scan), artScan: pixels(art), symbolScan: pixels(symbol),
        printing, typography, icons: await manaTemplates(printing.mana_cost),
      });
      if (style['--source-frame-status'] !== 'masked') return {status: style['--source-frame-status']};
      const clean = await read(style['--source-frame-image'].slice(5, -2));
      // Include the original initials, not just the detector's reported bounds.
      const residuals = [[39, 40, 208, 27], [40, 392, 169, 22], [400, 614, 40, 23]].map(bounds => {
        const before = scan.getImageData(...bounds).data, after = clean.getImageData(...bounds).data;
        let ink = 0, left = 0;
        for (let i = 0; i < before.length; i += 4) if (Math.max(...before.subarray(i, i + 3)) < 90) {
          ink++;
          if (Math.max(...after.subarray(i, i + 3)) < 90) left++;
        }
        return {ink, left};
      });
      let changed = 0;
      // Title/type rails, complete set logo, and artwork must remain exact.
      for (const bounds of [[25, 23, 435, 5], [25, 73, 435, 5], [27, 383, 430, 4], [27, 417, 430, 4], [400, 387, 54, 32], [38, 80, 410, 290]]) {
        const before = scan.getImageData(...bounds).data, after = clean.getImageData(...bounds).data;
        for (let i = 0; i < before.length; i++) if (before[i] !== after[i]) changed++;
      }
      return {
        status: style['--source-frame-status'], residuals, changed,
        rulesAlignment: style['--printed-rules-text-align'] || 'left',
        title: JSON.parse(style['--printed-title-text-bounds']),
        type: JSON.parse(style['--printed-type-text-bounds']),
      };
    });
    assert.equal(result.status, 'masked', JSON.stringify(result));
    assert.equal(result.rulesAlignment, 'left', 'recovering the type initial preserves left-aligned rules');
    assert.ok(result.title.x <= 42 && result.type.x <= 43, JSON.stringify(result));
    for (const residual of result.residuals) assert.ok(residual.ink > 50 && residual.left / residual.ink < .02, JSON.stringify(result));
    assert.equal(result.changed, 0, 'panel borders, set symbol and artwork remain unchanged');
  } finally { await browser.close(); await vite.close(); }
});
