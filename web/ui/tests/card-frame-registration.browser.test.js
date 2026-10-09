import test from 'node:test';
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {readFile, writeFile, mkdir} from 'node:fs/promises';
import {join} from 'node:path';
import {fileURLToPath} from 'node:url';
import {createServer} from 'vite';
import {chromium} from 'playwright';
import registrations from '../src/lib/card-region-history.generated.js';

const cache = process.env.FRAME_HISTORY_CACHE;
test('every history face selects a registration through production preparation', {skip: !cache, timeout: 300000}, async () => {
  const root = fileURLToPath(new URL('../', import.meta.url));
  const corpus = JSON.parse(await readFile(join(root, 'tests/frame-history-corpus.json')));
  const cases = corpus.cases.filter(c => registrations.some(r => r.id === c.id && r.face === c.face));
  assert.equal(cases.length, registrations.length);
  const printings = new Map(await Promise.all([...new Set(cases.map(c => c.id))].map(async id =>
    [id, JSON.parse(await readFile(join(cache, id + '.json')))])));
  const vite = await createServer({root, server: {host: '127.0.0.1', port: 0, hmr: false}, logLevel: 'silent'});
  await vite.listen();
  const browser = await chromium.launch();
  try {
    const page = await browser.newPage();
    await page.route('https://**', async route => {
      const url = route.request().url();
      if (url.startsWith('https://cards.scryfall.io/')) {
        const c = cases.find(c => url.includes(c.id) && url.includes(c.source.includes('/back/') ? '/back/' : '/front/'));
        if (!c) return route.abort();
        return route.fulfill({contentType: 'image/jpeg', headers: {'Access-Control-Allow-Origin': '*'},
          body: await readFile(join(cache, c.slug + (url.includes('/art_crop/') ? '-art_crop.jpg' : '-normal.jpg')))});
      }
      if (url.startsWith('https://api.scryfall.com/cards/')) {
        const printing = printings.get(new URL(url).pathname.split('/').at(-1));
        return printing ? route.fulfill({json: printing}) : route.abort();
      }
      // Set symbols are unnecessary for the registered path. Keep this audit offline.
      return route.fulfill({json: {}});
    });
    await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/card-frame-comparison.html`);
    const results = await page.evaluate(async cases => {
      const {prepareCardFrame} = await import('/src/lib/card-frame-preparation.js');
      const pending = [...cases], results = [];
      await Promise.all(Array.from({length: 8}, async () => {
        while (pending.length) {
          const c = pending.shift();
          try {
            const frame = await prepareCardFrame(c.source.replace('/normal/', '/art_crop/'));
            results.push({slug: c.slug, name: c.name, registered: !!frame.registration,
              registrationId: frame.registration?.id, face: frame.registration?.face ?? null});
          } catch (error) { results.push({slug: c.slug, name: c.name, registered: false, error: error.message}); }
        }
      }));
      return results;
    }, cases);
    for (const result of results) {
      const c = cases.find(c => c.slug === result.slug);
      const registration = registrations.find(r => r.id === c.id && r.face === c.face);
      result.registrationSha256 = createHash('sha256').update(JSON.stringify(registration)).digest('hex');
    }
    const out = join(root, 'test-results/frame-history-production-selection');
    await mkdir(out, {recursive: true});
    await writeFile(join(out, 'results.json'), JSON.stringify(results, null, 2));
    assert.equal(results.length, cases.length);
    assert.deepEqual(results.filter(r => !r.registered), []);
    for (const result of results) {
      const c = cases.find(c => c.slug === result.slug);
      assert.equal(result.registrationId, c.id, c.name);
      assert.equal(result.face, c.face, c.name);
    }
  } finally { await browser.close(); await vite.close(); }
});
