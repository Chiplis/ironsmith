import test from 'node:test';
import assert from 'node:assert/strict';
import { chromium } from 'playwright';
import { createServer } from 'vite';
import { mkdir } from 'node:fs/promises';
import path from 'node:path';

test('Build Sandbox is configurable on desktop and mobile; actual WASM accepts its setup', { timeout: 240000 }, async () => {
  const vite = await createServer({ server: { host: '127.0.0.1', port: 0, hmr: false, watch: null }, logLevel: 'error' });
  await vite.listen();
  const browser = await chromium.launch();
  try {
    const page = await browser.newPage({ viewport: { width: 1280, height: 900 } });
    const errors = [];
    page.on('pageerror', error => errors.push(error.message));
    await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/sandbox-preview.html`);
    await page.getByRole('button', { name: 'Build', exact: true }).click();
    await page.getByRole('tab', { name: 'Sandbox', exact: true }).click();
    await page.getByRole('button', { name: 'Use current card draft' }).click();
    for (const label of ['Cast without paying mana costs', 'Unlimited land plays', 'Cast from your graveyard', 'Cast from exile', 'Set any player’s life total', 'No maximum hand size']) {
      await page.getByRole('checkbox', { name: label, exact: true }).check();
    }
    await page.getByRole('checkbox', { name: 'Give each player a Sandboxer', exact: true }).uncheck();
    const preview = page.getByRole('region', { name: 'Sandboxer rules preview' });
    assert.ok((await preview.innerText()).includes('You may cast spells from your graveyard.'));
    await page.getByRole('checkbox', { name: 'Cast from exile', exact: true }).uncheck();
    assert.ok(!(await preview.innerText()).includes('You may cast spells from exile.'));
    await page.getByRole('tab', { name: 'Card', exact: true }).click();
    await page.getByRole('tab', { name: 'Sandbox', exact: true }).click();
    assert.equal(await page.getByRole('checkbox', { name: 'Cast without paying mana costs', exact: true }).isChecked(), true);
    assert.equal(await page.getByRole('spinbutton', { name: 'Copies of Custom Card' }).inputValue(), '4');
    await mkdir('test-results', { recursive: true });
    await page.screenshot({ path: 'test-results/sandbox-desktop.png' });
    await page.setViewportSize({ width: 390, height: 844 });
    assert.equal(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth), true);
    await page.screenshot({ path: 'test-results/sandbox-mobile.png' });
    // The engine may succeed even when optional display metadata cannot persist.
    await page.evaluate(() => {
      const setItem = Storage.prototype.setItem;
      Storage.prototype.setItem = function(key, value) {
        if (key === 'ironsmith-compiled-card-names' || key === 'ironsmith-custom-card-art-urls') {
          throw new DOMException('Storage quota exceeded', 'QuotaExceededError');
        }
        return setItem.call(this, key, value);
      };
    });
    await page.getByRole('button', { name: 'Start sandbox', exact: true }).click();
    await page.waitForFunction(() => window.sandboxPlan);
    await page.getByRole('dialog').waitFor({ state: 'hidden' });
    assert.equal(await page.evaluate(async () => {
      const { isCompiledCardName } = await import('/src/lib/scryfall.js');
      return isCompiledCardName('IronSmith Sandboxer');
    }), true, 'the sandbox finishes and its live text remains authoritative with storage full');
    const plan = await page.evaluate(() => window.sandboxPlan);
    assert.equal(plan.players[0].hand.length, 7);
    assert.equal(plan.sandboxer.faces[0].name, 'IronSmith Sandboxer');
    assert.equal(plan.players[0].hand[0].draft.faces[0].name, 'Custom Card');
    const assertPreferences = async () => {
      for (const label of ['Cast without paying mana costs', 'Unlimited land plays', 'Cast from your graveyard', 'Set any player’s life total', 'No maximum hand size']) {
        assert.equal(await page.getByRole('checkbox', { name: label, exact: true }).isChecked(), true, label);
      }
      for (const label of ['Cast from exile', 'Repeatable mana ability', 'Draw cards on demand', 'Give each player a Sandboxer']) {
        assert.equal(await page.getByRole('checkbox', { name: label, exact: true }).isChecked(), false, label);
      }
    };
    const openSandbox = async () => {
      await page.getByRole('button', { name: 'Build', exact: true }).click();
      await page.getByRole('tab', { name: 'Sandbox', exact: true }).click();
    };
    await openSandbox();
    await assertPreferences();
    await page.reload();
    await openSandbox();
    await assertPreferences();
    await page.evaluate(() => {
      Storage.prototype.setItem = () => { throw new DOMException('Full', 'QuotaExceededError'); };
    });
    await page.getByRole('checkbox', { name: 'Draw cards on demand', exact: true }).check();
    await page.keyboard.press('Escape');
    await openSandbox();
    assert.equal(await page.getByRole('checkbox', { name: 'Draw cards on demand', exact: true }).isChecked(), true);
    const freshPage = await browser.newPage();
    await freshPage.goto(page.url());
    await freshPage.getByRole('button', { name: 'Build', exact: true }).click();
    await freshPage.getByRole('tab', { name: 'Sandbox', exact: true }).click();
    assert.equal(await freshPage.getByRole('checkbox', { name: 'Cast without paying mana costs', exact: true }).isChecked(), false);
    assert.equal(await freshPage.getByRole('checkbox', { name: 'Give each player a Sandboxer', exact: true }).isChecked(), true);
    await freshPage.close();
    assert.deepEqual(errors, []);
    const result = await page.evaluate(async wasmUrl => {
      const { buildSandboxPlan, SANDBOX_RULES, sandboxerDraft, startSandbox } = await import('/src/lib/sandbox.js');
      const { prepareCustomDraftForCreation } = await import('/src/lib/custom-card-library.js');
      const { normalizeDraftForApi } = await import('/src/lib/custom-card-draft.js');
      const mod = await import(wasmUrl);
      await mod.default();
      const game = new mod.WasmGame();
      const sources = await Promise.all(['forest', 'grizzly-bears'].map(async route => {
        const response = await fetch(`/cards/${route}.json`);
        if (!response.ok) throw new Error(`Missing ${route}`);
        return response.json();
      }));
      game.registerExternalCardSourcesJson(JSON.stringify(sources));
      const rawDraft = { layout: 'single', hasFuse: false, faces: [{ name: 'Sandbox Test Bear', manaCost: '{1}{G}', colorIndicator: [], supertypes: [], cardTypes: ['Creature'], subtypes: ['Bear'], oracleText: '', power: '2', toughness: '2', loyalty: '', defense: '' }] };
      const { draft } = await prepareCustomDraftForCreation(rawDraft, input => game.previewCustomCard(normalizeDraftForApi(input)));
      const options = Object.fromEntries(SANDBOX_RULES.map(rule => [rule.id, true]));
      const preview = game.previewCustomCard(sandboxerDraft(options));
      if (!preview.canCreate) throw new Error(JSON.stringify(preview));
      const plan = buildSandboxPlan({ selections: [{ id: 'bear', draft, copies: 4 }], testedId: 'bear', options });
      const created = await startSandbox(game, plan);
      const state = game.uiState();
      const original = JSON.stringify(state);
      plan.players[0].library[0] = { name: 'Definitely Missing Sandbox Card' };
      let restored = false;
      try { await startSandbox(game, plan); } catch { restored = JSON.stringify(game.uiState()) === original; }
      return { created: created.created.length, state, restored };
    }, `/@fs/${path.resolve('../wasm_demo/pkg/ironsmith.js')}`);
    assert.ok(result.created >= 6);
    assert.equal(result.state.players.length, 2);
    assert.equal(result.restored, true, 'a failed real engine setup restores the original table');

  } finally { await browser.close(); await vite.close(); }
});
