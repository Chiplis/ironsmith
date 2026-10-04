import { setupIncrementalPriorityFixture } from './fixtures/incremental-priority-scenario.mjs';
import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { chromium } from 'playwright';
import { createServer } from 'vite';
import initEngine, { WasmGame } from '../../wasm_demo/pkg/engine.js';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const refs = actions => actions.map(a => JSON.stringify(a.action_ref)).sort();

test('lands publish before Warp checks finish, full menus agree, and a blocked analysis worker cannot block commands', { timeout: 180000 }, async t => {
  await initEngine({ module_or_path: await readFile(new URL('../../wasm_demo/pkg/engine_bg.wasm', import.meta.url)) });
  const sources = await Promise.all(['mountain', 'sunbillow-verge', 'nova-hellkite', 'magmatic-hellkite', 'icetill-explorer', 'ornithopter'].map(async route =>
    JSON.parse(await readFile(new URL(`../public/cards/${route}.json`, import.meta.url)))));
  const rejectedSource = { canonicalName: 'Rejected Analysis Source', group: { kind: 'single',
    name: 'Rejected Analysis Source', block: 'Type: Creature — Test\nMana Cost: {0}\nPower/Toughness: 1/1\nDo an unsupported test thing.' } };
  const game = new WasmGame();
  let expected, land, secondLand, secondSpell;
  try {
    assert.ok(JSON.parse(game.registerExternalCardSourcesJson(JSON.stringify([rejectedSource]))).failed.length);
    game.registerExternalCardSourcesJson(JSON.stringify(sources));
    const fixture = await setupIncrementalPriorityFixture((method, ...args) => game[method](...args));
    ({ land, secondLand, secondSpell } = fixture);
    expected = refs(game.uiState().decision.actions);
  } finally { game.free(); }

  const server = await createServer({ root, configFile: path.join(root, 'vite.config.js'), logLevel: 'error', server: { host: '127.0.0.1', port: 0, hmr: false, watch: null } });
  await server.listen();
  const browser = await chromium.launch();
  const page = await browser.newPage();
  t.after(async () => {
    await page.unrouteAll({ behavior: 'wait' });
    await browser.close();
    await server.close();
  });
  page.on('console', message => { if (message.type() === 'error') console.error(message.text()); });
  const errors = []; page.on('pageerror', error => errors.push(error.message));
  await page.route('**/analysis-test', route => route.fulfill({ contentType: 'text/html', body: '<title>Analysis isolation regression</title>' }));
  // Inject a deterministic slow slice into the actual child worker, after its
  // first partial result. The authoritative worker must remain responsive.
  await page.route('**/src/workers/priorityAnalysisWorker.js*', async route => {
    const response = await route.fetch();
    let body = await response.text();
    const marker = 'complete = decision.analysis_complete === true;';
    assert.ok(body.includes(marker));
    body = body.replace(marker, `if (!complete && decision.actions.length > 0 && !self.__stalledToken?.has(token)) { (self.__stalledToken ||= new Set()).add(token); const end = performance.now() + 1500; while (performance.now() < end) {} }\n${marker}`);
    await route.fulfill({ response, body });
  });
  await page.goto(`http://127.0.0.1:${server.httpServer.address().port}/analysis-test`);
  const result = await page.evaluate(async ({ sources, rejectedSource, land, secondLand, secondSpell }) => {
    const { createSnapshotDecoder } = await import('/src/lib/snapshot-channel.js');
    const { serializePriorityCommand } = await import('/src/lib/sync-commands.js');
    const decoder = createSnapshotDecoder(), requests = new Map(), messages = [], waiters = [];
    const worker = new Worker('/src/workers/wasmGameWorker.js', { type: 'module' });
    let nextId = 0;
    const ready = new Promise((resolve, reject) => {
      worker.onerror = e => reject(new Error(e.message));
      worker.onmessage = ({ data }) => {
        if (data.type === 'ready') return resolve();
        if (data.type === 'error' || data.type === 'priorityAnalysisError') {
          const error = new Error(`${data.type}: ${data.error?.stack || data.error?.message || 'worker failed'}`);
          reject(error); for (const entry of waiters) entry.reject(error); return;
        }
        if (data.type === 'priorityAnalysis') {
          messages.push(data);
          for (const entry of [...waiters]) if (entry.match(data)) { waiters.splice(waiters.indexOf(entry), 1); entry.resolve(data); }
        }
        if (data.type === 'result') {
          const entry = requests.get(data.id); if (!entry) return;
          requests.delete(data.id);
          if (!data.ok) entry.reject(new Error(data.error.message));
          else entry.resolve(data.snapshot ? decoder.decode(data.snapshot) : data.result);
        }
      };
    });
    const call = (method, args = [], runtimeBranch) => new Promise((resolve, reject) => {
      const id = ++nextId; requests.set(id, { resolve, reject }); worker.postMessage({ type: 'call', id, method, args, runtimeBranch });
    });
    const wait = match => {
      const found = messages.find(match); if (found) return Promise.resolve(found);
      return new Promise((resolve, reject) => waiters.push({ match, resolve, reject }));
    };
    worker.postMessage({ type: 'init', assetBaseUrl: location.origin + '/' });
    try {
      await ready;
      await call('registerExternalCardSourcesJson', [JSON.stringify([rejectedSource, ...sources])]);
      const { setupIncrementalPriorityFixture, advancePriorityFixture } = await import('/tests/fixtures/incremental-priority-scenario.mjs');
      const fixtureCall = (method, ...args) => call(method, args);
      const coldStarted = performance.now();
      const { state } = await setupIncrementalPriorityFixture(fixtureCall);
      const main = await call('createRuntimeSavepoint');
      const revision = state.__priority_revision;
      const first = await wait(m => m.revision === revision && m.decision.actions.some(a => a.action_ref?.kind === 'play_land' && Number(a.object_id) === land));
      const coldHighlightMs = performance.now() - coldStarted;
      const started = performance.now();
      const handle = await call('createRuntimeSavepoint');
      await call('releaseRuntimeSavepoint', [handle]);
      const commandMs = performance.now() - started;
      const final = await wait(m => m.revision === revision && m.decision.analysis_complete === true);
      // Restore, then execute a confirmed partial action while the child is
      // busy. Action refs must remain valid without its completed menu.
      await new Promise(resolve => setTimeout(resolve, 100));
      const warmStarted = performance.now();
      await call('dispatch', [{ type: 'priority_action', action_ref: { kind: 'pass_priority' } }]);
      const reset = await call('copyRuntimeSavepoint', [main]);
      const partial = await wait(m => m.revision === reset.__priority_revision && m.decision.actions.some(a => a.action_ref?.kind === 'play_land' && Number(a.object_id) === land));
      const warmHighlightMs = performance.now() - warmStarted;
      const action = partial.decision.actions.find(a => a.action_ref?.kind === 'play_land' && Number(a.object_id) === land);
      const playStarted = performance.now();
      const live = await call('uiState');
      const synced = serializePriorityCommand({ type: 'priority_action', action_ref: action.action_ref }, live.decision);
      const played = await call('dispatch', [synced]);
      const playMs = performance.now() - playStarted;
      const after = await call('getHiddenCardState');
      let staleRejected = false;
      try { await call('dispatch', [synced]); } catch { staleRejected = true; }
      await advancePriorityFixture(fixtureCall, 1);
      await call('setPerspective', [1]);
      const branch = await call('createRuntimeSavepoint');
      const visible = await call('uiState');
      const secondMenu = await Promise.race([
        wait(m => m.revision === visible.__priority_revision && m.decision.analysis_complete),
        new Promise((_, reject) => setTimeout(() => reject(new Error('second seat never completed analysis')), 15000)),
      ]);
      const { mergePriorityAnalysis } = await import('/src/lib/priority-analysis-scheduler.js');
      const merged = mergePriorityAnalysis(visible, secondMenu);
      const guestLand = merged.decision.actions.find(a => a.action_ref?.kind === 'play_land' && Number(a.object_id) === secondLand);
      const guestSpell = merged.decision.actions.find(a => a.action_ref?.kind === 'cast_spell' && Number(a.object_id) === secondSpell);
      const guestMetadata = await call('getHiddenCardState');
      const guestLandStableId = guestMetadata.objects.find(object => Number(object.id) === secondLand).stableId;
      const secondPlayable = Boolean(guestLand && guestSpell);
      let guestPlayedLand = false, guestCastSpell = false;
      if (secondPlayable) {
        await call('dispatch', [serializePriorityCommand({ type: 'priority_action', action_ref: guestSpell.action_ref }, (await call('uiState')).decision)]);
        const spellAfter = await call('getHiddenCardState');
        guestCastSpell = spellAfter.objects.some(o => o.name === 'Ornithopter' && o.zone === 'stack');
        // Land and spell are independent actions from the same guest prompt.
        // Restore it because casting transfers priority to the other seat.
        await call('copyRuntimeSavepoint', [branch]);
        await call('dispatch', [serializePriorityCommand({ type: 'priority_action', action_ref: guestLand.action_ref }, (await call('uiState')).decision)]);
        const landAfter = await call('getHiddenCardState');
        guestPlayedLand = landAfter.objects.some(o => o.stableId === guestLandStableId && o.name === 'Mountain' && o.zone === 'battlefield');
      }
      await call('releaseRuntimeSavepoint', [branch]);
      await call('releaseRuntimeSavepoint', [main]);
      return { secondPlayable, guestPlayedLand, guestCastSpell, first: first.decision, final: final.decision, commandMs, playMs, coldHighlightMs, warmHighlightMs, staleRejected,
        played: after.objects.some(o => o.name === 'Sunbillow Verge' && o.zone === 'battlefield'), newRevision: played.__priority_revision !== reset.__priority_revision };
    } finally { worker.terminate(); }
  }, { sources, rejectedSource, land, secondLand, secondSpell });
  assert.equal(result.secondPlayable, true);
  assert.equal(result.guestPlayedLand, true);
  assert.equal(result.guestCastSpell, true);
  assert.equal(result.first.analysis_complete, false);
  assert.equal(result.first.actions.some(a => a.kind === 'cast_spell'), false);
  assert.deepEqual(refs(result.final.actions), expected);
  assert.ok(result.commandMs < 1000, `savepoint commands waited ${result.commandMs} ms behind analysis`);
  assert.ok(result.playMs < 1000, `land command waited ${result.playMs} ms behind analysis`);
  assert.equal(result.staleRejected, true);
  assert.ok(result.warmHighlightMs < 1000, `warm highlight took ${result.warmHighlightMs} ms`);
  assert.equal(result.played, true); assert.equal(result.newRevision, true); assert.deepEqual(errors, []);
  t.diagnostic(JSON.stringify({ coldHighlightMs: result.coldHighlightMs, warmHighlightMs: result.warmHighlightMs, savepointCommandsMs: result.commandMs, landPlayMs: result.playMs, injectedAnalysisStallMs: 1500, actions: expected.length }));
});
