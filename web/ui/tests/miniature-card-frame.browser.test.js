import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile, mkdir, writeFile } from 'node:fs/promises';
import process from 'node:process';
import { chromium } from 'playwright';
import { createServer } from 'vite';
import corpus from './fixtures/bilingual-frames/corpus.js';

test('battlefield miniatures scale complete frames and retain image-only fallbacks', {timeout: 120000}, async t => {
  const entries = ['yawgmoth-thran-physician-es', 'ornithopter-es', 'swamp-es', 'leyline-of-anticipation-es', 'sigarda-host-of-herons-es', 'omniscience-es']
    .map(slug => corpus.find(entry => entry.slug === slug));
  const base = new URL('./fixtures/bilingual-frames/', import.meta.url);
  const vite = await createServer({server: {host: '127.0.0.1', port: 0}, logLevel: 'silent'});
  await vite.listen();
  const browser = await chromium.launch({args: process.env.CARD_FRAME_GPU === '1' ? ['--enable-unsafe-webgpu', ...(process.platform === 'darwin' ? ['--use-angle=metal'] : [])] : []});
  try {
    const page = await browser.newPage({viewport: {width: 950, height: 250}, reducedMotion: 'reduce'});
    if (process.env.CARD_FRAME_WORKER === '0') await page.addInitScript(() => {window.Worker = undefined;});
    if (process.env.CARD_FRAME_WORKER === 'fail') await page.addInitScript(() => {
      const NativeWorker = window.Worker;
      window.Worker = class extends NativeWorker {
        postMessage(...args) {super.postMessage(...args); this.dispatchEvent(new Event('error'));}
      };
    });
    const profiler = process.env.CARD_FRAME_PROFILE ? await page.context().newCDPSession(page) : null;
    if (profiler) {
      await profiler.send('Profiler.enable'); await profiler.send('Profiler.start');
      await page.addInitScript(() => {
        window.__frameLongTasks = [];
        new PerformanceObserver(list => window.__frameLongTasks.push(...list.getEntries().map(entry => entry.duration))).observe({type: 'longtask', buffered: true});
      });
    }
    const errors = [];
    page.on('pageerror', error => errors.push(error.message));
    await page.addInitScript(cards => {localStorage.setItem('ironsmith.locale', 'es'); window.__miniatureCards = cards;}, entries.map(entry => entry.printing));
    await page.route('https://**', async route => {
      const url = new URL(route.request().url());
      if (url.hostname === 'cards.scryfall.io') {
        const entry = corpus.find(entry => url.pathname.includes(entry.printing.id));
        if (!entry) return route.abort();
        return route.fulfill({contentType: 'image/jpeg', headers: {'Access-Control-Allow-Origin': '*'},
          body: await readFile(new URL(`${entry.slug}-${url.pathname.includes('/art_crop/') ? 'art_crop' : 'normal'}.jpg`, base))});
      }
      if (url.hostname === 'svgs.scryfall.io') return route.fulfill({contentType: 'image/svg+xml', headers: {'Access-Control-Allow-Origin': '*'}, body: await readFile(new URL(url.pathname.split('/').at(-1), base))});
      if (url.hostname === 'api.scryfall.com') {
        const entry = corpus.find(entry => url.pathname.endsWith(`/${entry.printing.id}`) || url.pathname.endsWith(`/${entry.printing.set}/${entry.printing.collector_number}/${entry.printing.lang}`));
        if (url.pathname.startsWith('/sets/')) return route.fulfill({json: {icon_svg_uri: `https://svgs.scryfall.io/sets/${url.pathname.split('/').at(-1)}.svg`}});
        if (url.pathname === '/cards/search') return route.fulfill({json: {data: corpus.filter(entry => url.searchParams.get('q')?.includes(entry.printing.name)).map(entry => entry.printing), has_more: false}});
        return route.fulfill({status: entry ? 200 : 404, json: entry?.printing || {}});
      }
      return route.abort();
    });
    await page.route('**/cards/*.json', route => route.fulfill({json: {}}));
    await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/miniature-card-frame.html`);
    await page.waitForFunction(() => document.querySelectorAll('.battlefield-prepared-frame [data-render-ready="true"]').length === 6, null, {timeout: 90000});
    await page.evaluate(async () => {await document.fonts.ready; await Promise.allSettled([...document.images].map(image => image.decode()));});
    const settle = () => page.evaluate(() => new Promise((resolve, reject) => {
      let previous = '', stable = 0, frames = 0;
      const check = () => {
        const next = JSON.stringify([...document.querySelectorAll('.battlefield-prepared-frame__composition,.interactive-card-frame__title')].map(node => [node.getBoundingClientRect().toJSON(), getComputedStyle(node).fontSize]));
        stable = next === previous ? stable + 1 : 0; previous = next;
        if (stable >= 5) return resolve();
        if (++frames > 180) return reject(Error('Miniature layout did not settle'));
        requestAnimationFrame(check);
      }; requestAnimationFrame(check);
    }));
    await settle();
    const backends = await page.locator('.battlefield-prepared-frame .interactive-card-frame-stage').evaluateAll(stages => stages.map(stage => stage.style.getPropertyValue('--frame-processing-backend')));
    assert.ok(backends.every(backend => ['0', 'fail'].includes(process.env.CARD_FRAME_WORKER) ? backend === 'main-cpu' : backend === 'webgpu' || backend === 'worker-cpu'), `expected processing backend: ${backends}`);
    if (process.env.CARD_FRAME_GPU === '1') assert.ok(backends.includes('webgpu'), 'hardware GPU must process the frames');
    const typography = await page.locator('.battlefield-prepared-frame .interactive-card-frame-stage').evaluateAll(stages => stages.map(stage => {
      const box = stage.querySelector('.interactive-card-frame__rules');
      const flavor = stage.querySelector('.inspector-flavor-text');
      return {scale: Number(box?.style.getPropertyValue('--card-rules-fit-scale')),
        leading: Number(stage.style.getPropertyValue('--printed-rules-line-height')),
        flavor: stage.style.getPropertyValue('--printed-flavor-first-line'),
        flavorOffset: stage.style.getPropertyValue('--printed-flavor-offset'),
        flavorLeading: flavor && getComputedStyle(flavor).lineHeight};
    }));
    assert.equal(typography[0].scale, 1, 'Spanish Yawgmoth must not shrink because of its keyword paragraph gap');
    assert.ok(typography[0].leading < 1.15, 'wrapped rules use printed leading');
    const reminder = await page.locator('.battlefield-prepared-frame .interactive-card-frame-stage').first().evaluate(async stage => {
      const {measureCardFrameLayout} = await import('/src/lib/card-frame-measurement.js');
      return measureCardFrameLayout(stage, () => {
        const node = stage.querySelector('.rules-reminder-text'), rows = [];
        const walker = document.createTreeWalker(node, NodeFilter.SHOW_TEXT);
        while (walker.nextNode()) for (let i=0; i<walker.currentNode.length; i++) {
          const range = document.createRange(); range.setStart(walker.currentNode,i); range.setEnd(walker.currentNode,i+1);
          const rect = range.getBoundingClientRect();
          let row = rows.find(row => Math.abs(row.y-rect.y)<1);
          if (!row) {row={y:rect.y,text:''};rows.push(row);}
          row.text += walker.currentNode.textContent[i];
        }
        const style = getComputedStyle(node), ctx = document.createElement('canvas').getContext('2d');
        ctx.font = `italic 400 ${style.fontSize} ${style.fontFamily}`;
        const metrics = ctx.measureText(rows[1].text.trim()), card = stage.querySelector('.interactive-card-frame').getBoundingClientRect();
        return {lines:rows.map(row=>row.text.trim()), inkY:(rows[1].y+metrics.fontBoundingBoxAscent-metrics.actualBoundingBoxAscent-card.top)/card.height*680};
      });
    });
    assert.deepEqual(reminder.lines, ['(Elige', 'cualquier cantidad de permanentes y/o jugadores,', 'luego pon sobre cada uno un contador de cada', 'tipo que ya tenga.)']);
    assert.ok(Math.abs(reminder.inkY-557)<3, `reminder aligns with the source scan: ${reminder.inkY}`);
    const omniscience = typography.at(-1);
    assert.equal(omniscience.scale, 1, 'Spanish Omniscience retains its printed size');
    assert.ok(JSON.parse(omniscience.flavor).line.startsWith('“Las cosas'), 'measure the printed italic quotation');
    assert.ok(omniscience.flavorOffset && omniscience.flavorLeading, 'flavor retains its own placement and leading');
    if (profiler) {
      const {profile} = await profiler.send('Profiler.stop');
      const nodes = new Map(profile.nodes.map(node => [node.id, node]));
      const timings = new Map();
      profile.samples.forEach((id, index) => {
        const frame = nodes.get(id).callFrame, key = `${frame.functionName} ${frame.url}:${frame.lineNumber + 1}`;
        timings.set(key, (timings.get(key) || 0) + profile.timeDeltas[index] / 1000);
      });
      const report = {longTasks: await page.evaluate(() => window.__frameLongTasks), selfTimeMs: [...timings].sort((a,b) => b[1]-a[1]).slice(0, 40)};
      await mkdir('test-results/miniature-frames', {recursive: true});
      await writeFile(`test-results/miniature-frames/${process.env.CARD_FRAME_PROFILE}.json`, JSON.stringify(report, null, 2));
    }
    assert.equal(await page.evaluate(() => window.__miniatureDetailsRequests), 0, 'miniatures do not request inspector details');
    await page.evaluate(() => {
      window.__frameLayoutChanges = 0;
      window.__frameLayoutObserver = new MutationObserver(records => {window.__frameLayoutChanges += records.length;});
      document.querySelectorAll('.battlefield-prepared-frame__composition').forEach(node => window.__frameLayoutObserver.observe(node, {subtree: true, attributes: true, attributeFilter: ['style']}));
    });
    await page.evaluate(() => window.__updateMiniatureSnapshot());
    await settle();
    assert.equal(await page.evaluate(() => window.__miniatureDetailsRequests), 0, 'game updates do not trigger per-card inspector requests');
    assert.equal(await page.evaluate(() => {window.__frameLayoutObserver.disconnect(); return window.__frameLayoutChanges;}), 0, 'unrelated game updates do not refit miniature text');
    const measure = () => page.locator('.battlefield-prepared-frame').evaluateAll(hosts => hosts.map(host => {
      const stage = host.querySelector('.interactive-card-frame-stage');
      const box = host.getBoundingClientRect();
      const title = stage.querySelector('.interactive-card-frame__title');
      const mana = [...stage.querySelectorAll('.interactive-card-frame__mana svg')].map(icon => icon.getBoundingClientRect().width);
      return {layoutWidth: stage.clientWidth, width: box.width, mana, titleSize: title ? getComputedStyle(title).fontSize : null,
        details: stage.querySelectorAll('details').length, stats: stage.querySelectorAll('.interactive-card-frame__art-stats').length,
        presentation: stage.dataset.framePresentation, mode: stage.dataset.frameMode};
    }));
    const before = await measure();
    assert.ok(before.some(card => card.mode === 'masked'), 'exercise generated masks');
    assert.ok(before.some(card => card.mana.length), 'exercise rendered mana costs');
    for (const card of before) {
      assert.equal(card.layoutWidth, 380);
      assert.equal(card.presentation, 'miniature');
      assert.equal(card.details, 0);
      assert.equal(card.stats, 0);
      assert.ok(card.mana.every(size => size > 0 && size < 7), JSON.stringify(card));
    }
    assert.equal(await page.locator('#fallback details').count(), 0);
    assert.equal(await page.locator('#fallback img').evaluate(image => image.complete && image.naturalWidth > 0), true);
    await mkdir('test-results/miniature-frames', {recursive: true});
    await page.locator('#miniatures').screenshot({path: 'test-results/miniature-frames/battlefield.png'});
    await page.locator('.game-card').evaluateAll(cards => cards.forEach(card => Object.assign(card.style, {width: '72px', minWidth: '72px', height: '101px', minHeight: '101px'})));
    await page.waitForFunction(() => document.querySelector('.battlefield-prepared-frame').getBoundingClientRect().width < 80);
    await settle();
    const after = await measure();
    for (let i = 0; i < before.length; i++) {
      assert.equal(after[i].layoutWidth, 380, 'resizing preserves canonical layout');
      assert.equal(after[i].titleSize, before[i].titleSize, 'resizing does not refit the title');
      for (let j = 0; j < before[i].mana.length; j++) assert.ok(after[i].mana[j] < before[i].mana[j], 'mana scales with the card');
    }
    await page.locator('.game-card').evaluateAll(cards => cards.forEach(card => {
      card.style.transition = 'none';
      card.style.transform = 'rotate(8deg) scale(1.04)';
    }));
    // A late font completion must fit in layout coordinates even while the
    // battlefield presentation is tapped or enlarged on hover.
    await page.evaluate(() => document.fonts.dispatchEvent(new Event('loadingdone')));
    await settle();
    const transformed = await measure();
    for (let i = 0; i < after.length; i++) if (after[i].titleSize) {
      assert.ok(Math.abs(parseFloat(transformed[i].titleSize) - parseFloat(after[i].titleSize)) < .05, 'tap and hover transforms do not alter fitted type');
    }
    await page.route('**/missing-frame.jpg', route => route.fulfill({status: 404, body: ''}));
    await page.locator('#fallback img').evaluate(image => {image.src = '/missing-frame.jpg';});
    await page.waitForFunction(() => document.querySelector('#fallback img').style.visibility === 'hidden');
    assert.equal(await page.locator('#fallback article').evaluate(node => getComputedStyle(node).backgroundColor), 'rgba(0, 0, 0, 0)', 'failed fallback keeps underlying art visible');
    assert.deepEqual(errors, []);
    if (process.env.CARD_FRAME_COMPARE === '1') {
      const differences = await page.evaluate(async cards => {
        const {cachedCardFrame} = await import('/src/lib/card-frame-preparation.js');
        const {sampleCardFrameColors} = await import('/src/lib/card-frame-colors.js');
        const {resolveScryfallSetSymbol} = await import('/src/lib/scryfall.js');
        const pixels = async style => {
          const image = new Image(); image.src = style['--source-frame-image'].slice(5, -2); await image.decode();
          const canvas = document.createElement('canvas'); canvas.width = image.width; canvas.height = image.height;
          const context = canvas.getContext('2d'); context.drawImage(image, 0, 0); return context.getImageData(0, 0, image.width, image.height).data;
        };
        const results = [];
        for (const card of cards) {
          const prepared = cachedCardFrame(card.image_uris.art_crop, card.type_line);
          if (!prepared?.style?.['--source-frame-image']) continue;
          const url = new URL(prepared.printing.image_uris.normal); url.searchParams.set('cpu-reference', '1');
          const reference = await sampleCardFrameColors(url.href, {typography: prepared.typography, printing: prepared.printing, setSymbolUrl: await resolveScryfallSetSymbol(prepared.printing)});
          if (!reference?.['--source-frame-image']) {results.push({name: card.name, missingReference: true}); continue;}
          const [cpu, gpu] = await Promise.all([pixels(reference), pixels(prepared.style)]);
          let maximum = 0, difference = 0, changed = 0;
          for (let i = 0; i < cpu.length; i++) {const delta = Math.abs(cpu[i] - gpu[i]); if (delta) {changed++; difference += delta; maximum = Math.max(maximum, delta);}}
          results.push({name: card.name, maximum, changed, meanChanged: changed ? difference / changed : 0});
        }
        return results;
      }, entries.map(entry => entry.printing));
      assert.ok(differences.length >= 3, 'compare real card masks against CPU references');
      for (const difference of differences) assert.ok(!difference.missingReference && difference.maximum <= 12 && difference.meanChanged <= 2, JSON.stringify(difference));
      t.diagnostic(JSON.stringify(differences));
    }
  } finally {await browser.close(); await vite.close();}
});
