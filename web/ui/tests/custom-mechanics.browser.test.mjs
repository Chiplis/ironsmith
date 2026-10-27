import test from 'node:test';
import assert from 'node:assert/strict';
import { chromium } from 'playwright';
import { createServer } from 'vite';
import path from 'node:path';

test('failed-card definitions persist, conflicts explain repairs, and Variuk has a reminder tooltip', {timeout: 180000}, async () => {
  const vite = await createServer({ server: { host: '127.0.0.1', port: 0, hmr: false, watch: null }, logLevel: 'error' });
  await vite.listen();
  const browser = await chromium.launch();
  try {
    const page = await browser.newPage();
    const errors = [];
    page.on('pageerror', error => errors.push(error.message));
    const url = `http://127.0.0.1:${vite.httpServer.address().port}/tests/custom-mechanics-preview.html`;
    const initialize = async () => {
      await page.goto(url);
      await page.evaluate(async moduleUrl => {
        const { default: init, WasmGame } = await import(moduleUrl);
        await init();
        window.mechanicTestGame = new WasmGame();
      }, `/@fs/${path.resolve('../wasm_demo/pkg/engine.js')}`);
    };
    await initialize();
    await page.getByRole('button', {name:'Save broken card', exact:true}).click();
    await page.getByRole('status').filter({hasText:'Save failed:'}).waitFor();
    assert.deepEqual(await page.evaluate(async () => {
      const lib = await import('/src/lib/custom-card-library.js');
      return [(await lib.listSavedCustomCards()).length, (await lib.listCustomMechanics()).map(def => def.keyword)];
    }), [0, ['shift']]);
    // Reload removes all in-memory definitions. Conflict must come from storage.
    await initialize();
    await page.getByRole('button', {name:'Save conflicting card', exact:true}).click();
    await page.getByRole('status').filter({hasText:'could not be added'}).waitFor();
    const message = await page.getByRole('status').innerText();
    assert.match(message, /Variuk, Head of Fear Slaying/);
    assert.match(message, /cost 1 less/);
    assert.match(message, /cost 2 less/);
    assert.match(message, /rename this mechanic/);
    assert.match(message, /fails to compile/);
    await page.getByRole('button', {name:'Save Variuk', exact:true}).click();
    await page.getByRole('status').filter({hasText:'Saved Variuk'}).waitFor();
    const card = page.getByTestId('compiled-card');
    assert.match(await card.textContent(), /shift, then put/);
    assert.ok(!(await card.textContent()).includes('Noncreature'));
    await card.getByRole('button', {name:'shift rules helper'}).click();
    const tooltip = page.locator('[data-ui-layer="tooltip"][data-state="open"]');
    await tooltip.waitFor();
    assert.match(await tooltip.innerText(), /Noncreature, nonartifact spells you cast cost 1 less to cast until end of turn/);
    const concurrent = await page.evaluate(async () => {
      const lib = await import('/src/lib/custom-card-library.js');
      const results = await Promise.allSettled([
        lib.reserveCustomMechanics([{keyword:'custompulse',reminder:'Draw a card.',sourceCard:'First contender'}]),
        lib.reserveCustomMechanics([{keyword:'custompulse',reminder:'You gain 2 life.',sourceCard:'Second contender'}]),
      ]);
      return results.map(result => result.status).sort();
    });
    assert.deepEqual(concurrent, ['fulfilled','rejected'], 'competing reservations cannot overwrite one another');
    const renamed = await page.evaluate(async () => {
      const { saveCustomCard } = await import('/src/lib/custom-card-library.js');
      const { variukDraft } = await import('/tests/custom-mechanics-fixture.mjs');
      const draft = variukDraft();
      draft.faces[0].oracleText = draft.faces[0].oracleText.replace(/\bshift\b/g, 'drift').replace('cost 1 less', 'cost 2 less');
      const saved = await saveCustomCard(draft, {compile: input => window.mechanicTestGame.previewCustomCard(input)});
      return saved.draft.customMechanics.map(definition => definition.keyword);
    });
    assert.deepEqual(renamed, ['drift'], 'renaming the mechanic allows a different definition');
    assert.deepEqual(errors, []);
  } finally { await browser.close(); await vite.close(); }
});
