import test from 'node:test';
import assert from 'node:assert/strict';
import { webkit, devices } from 'playwright';
import { createServer } from 'vite';

test('iOS recovery avoids heap transfers and verifier retries retain one heap', { timeout: 120000 }, async () => {
  const vite = await createServer({ server: { host: '127.0.0.1', port: 0 }, logLevel: 'silent' });
  await vite.listen();
  const browser = await webkit.launch();
  try {
    const page = await browser.newPage(devices['iPhone 13 landscape']);
    const wasmRequests = [];
    page.on('request', request => { if (request.url().includes('.wasm') && !request.url().includes('?import')) wasmRequests.push(request.url()); });
    await page.addInitScript(() => {
      const NativeWorker = window.Worker;
      window.memoryAudit = { created: 0, terminated: 0, live: 0, peak: 0, transferredSnapshots: 0 };
      window.Worker = class {
        constructor(url, options) {
          if (!String(url).includes('ziffleWorker')) {
            const worker = new NativeWorker(url, options), post = worker.postMessage.bind(worker);
            worker.postMessage = (...args) => {
              if (/^(capture|restore)ExactBuildSnapshot$/.test(args[0]?.method)) window.memoryAudit.transferredSnapshots++;
              return post(...args);
            };
            return worker;
          }
          const stats = window.memoryAudit;
          stats.created++; stats.live++; stats.peak = Math.max(stats.peak, stats.live);
          let worker;
          if (stats.created === 1) {
            worker = new EventTarget();
            worker.postMessage = () => queueMicrotask(() => worker.dispatchEvent(new MessageEvent('message', {
              data: { type: 'error', error: { message: 'fixture verifier initialization failure' } },
            })));
            worker.terminate = () => {};
          } else worker = new NativeWorker(url, options);
          const terminate = worker.terminate.bind(worker);
          let retired = false;
          worker.terminate = () => {
            if (!retired) { retired = true; stats.live--; stats.terminated++; }
            terminate();
          };
          return worker;
        }
      };
    });
    await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/ios-engine-memory.html`);
    await page.waitForFunction(() => window.auditHook?.game && !window.auditHook.loading, null, { timeout: 90000 });
    const result = await page.evaluate(async () => {
      const game = window.auditHook.game;
      const errors = [];
      for (const method of ['captureExactBuildSnapshot', 'restoreExactBuildSnapshot']) {
        // An uncloneable argument proves the proxy rejects before postMessage.
        try { await game[method]({ memory: () => {} }); } catch (error) { errors.push(error.message); }
      }
      await game.resetEmpty(['Alice', 'Bob'], 20);
      await game.createCustomCard({ playerIndex: 0, zoneName: 'hand', skipTriggers: true, draft: {
        layout: 'single', hasFuse: false, faces: [{ name: 'Memory Audit Spell', manaCost: '{U}',
          colorIndicator: [], supertypes: [], cardTypes: ['Instant'], subtypes: [],
          oracleText: 'Draw a card.', power: null, toughness: null, loyalty: null, defense: null }],
      } });
      const compiled = (await game.uiState()).players[0].hand_cards.some(card => card.name === 'Memory Audit Spell');
      await game.resetEmpty(['Alice', 'Bob'], 20);
      const input = { deckCount: 4, context: 'ios-memory-audit', entropyHex: '11'.repeat(32) };
      let failure;
      try { await game.ziffleKeygen(input); } catch (error) { failure = error.message; }
      const keys = await Promise.all([game.ziffleKeygen(input), game.ziffleKeygen({ ...input, entropyHex: '22'.repeat(32) })]);
      const validKeys = keys.every(key => key.publicKeyHex && key.ownershipProofHex);
      window.auditRoot.unmount();
      return { compiled, errors, failure, validKeys, exact: game.supportsExactBuildSnapshots, stats: window.memoryAudit };
    });
    assert.equal(result.exact, false);
    assert.equal(result.compiled, true);
    assert.equal(result.errors.length, 2);
    assert.ok(result.errors.every(message => message.includes('unavailable for this runtime')));
    assert.match(result.failure, /fixture verifier initialization failure/);
    assert.equal(result.validKeys, true);
    assert.deepEqual(result.stats, { created: 2, terminated: 2, live: 0, peak: 1, transferredSnapshots: 0 });
    assert.equal(wasmRequests.filter(url => url.includes('engine_bg')).length, 1);
    assert.equal(wasmRequests.filter(url => url.includes('compiler_bg')).length, 0);
    assert.equal(wasmRequests.filter(url => url.includes('verifier_bg')).length, 1);
  } finally { await browser.close(); await vite.close(); }
});
