import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { chromium } from 'playwright';
import { createServer } from 'vite';
import initEngine, { WasmGame } from '../../wasm_demo/pkg/engine.js';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
test('both seats keep combat priority until their instants are highlighted', { timeout: 60000 }, async t => {
  await initEngine({ module_or_path: await readFile(new URL('../../wasm_demo/pkg/engine_bg.wasm', import.meta.url)) });
  const sources = await Promise.all(['swamp', 'grizzly-bears', 'shoot-the-sheriff', 'requiting-hex', 'thoughtseize'].map(async route =>
    JSON.parse(await readFile(new URL(`../public/cards/${route}.json`, import.meta.url)))));
  const server = await createServer({ root, logLevel: 'error', server: { host: '127.0.0.1', port: 0, hmr: false, watch: null } });
  await server.listen(); t.after(() => server.close());
  const browser = await chromium.launch(); t.after(() => browser.close());
  for (const seat of [0, 1]) {
    const game = new WasmGame();
    let checkpoint;
    try {
      game.registerExternalCardSourcesJson(JSON.stringify(sources));
      game.resetEmpty(['Host', 'Guest'], 20);
      for (let i = 0; i < 3; i++) game.addCardToZone(seat, 'Swamp', 'battlefield', true);
      game.addCardToZone(1 - seat, 'Grizzly Bears', 'battlefield', true);
      for (const name of ['Shoot the Sheriff', 'Requiting Hex', 'Thoughtseize', 'Swamp']) game.addCardToZone(seat, name, 'hand', true);
      game.finishPuzzleSetup();
      for (let i = 0; i < 4; i++) {
        const action = game.uiState().decision.actions.find(a => ['keep_opening_hand', 'continue_pregame', 'begin_game'].includes(a.action_ref?.kind));
        assert.ok(action); game.dispatch({ type: 'priority_action', action_ref: action.action_ref });
      }
      checkpoint = game.exportSyncCheckpoint();
      checkpoint.perspective = seat;
      checkpoint.turn = { ...checkpoint.turn, activePlayer: 1 - seat, priorityPlayer: seat, turnNumber: 3, phase: 'combat', step: 'declare_attackers' };
      checkpoint.priorityRuntime.turnRunnerState = 'declare_attackers_priority';
    } finally { game.free(); }
    const page = await browser.newPage({ viewport: { width: 1400, height: 900 } });
    const errors = []; page.on('pageerror', error => errors.push(error.message));
    await page.addInitScript(data => { window.__combatFixture = data; }, { sources, checkpoint, seat });
    // A slow first analysis guarantees the UI sees the pass-only snapshot.
    await page.route('**/src/workers/priorityAnalysisWorker.js*', async route => {
      const response = await route.fetch();
      const body = (await response.text()).replace("  phase('search');", "  await new Promise(resolve => setTimeout(resolve, 200));\n  phase('search');");
      await route.fulfill({ response, body });
    });
    await page.goto(`http://127.0.0.1:${server.httpServer.address().port}/tests/combat-hand-highlights.html`);
    await page.waitForFunction(() => window.__combatState?.decision?.analysis_complete === true || window.__combatError || window.__combatAutoPassed);
    assert.equal(await page.evaluate(() => window.__combatAutoPassed || false), false, 'analysis must finish before automatic passing');
    assert.equal(await page.evaluate(() => window.__combatError || null), null);
    for (const name of ['Shoot the Sheriff', 'Requiting Hex']) {
      const card = page.getByRole('button', { name: `${name}, playable`, exact: true });
      await card.waitFor({ state: 'visible' });
      assert.match(await card.getAttribute('class'), /\bglow-instant\b/);
    }
    for (const name of ['Thoughtseize', 'Swamp']) assert.doesNotMatch(await page.getByRole('button', { name, exact: true }).getAttribute('class'), /\bglow-(instant|sorcery|land)\b/);
    assert.deepEqual(errors, []);
    await page.close();
  }
});
