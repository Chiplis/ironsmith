import test from "node:test";
import assert from "node:assert/strict";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";
import { createServer } from "vite";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
// Exercise the real App, hand, floating inspector, and WASM engine together.
// Inject only scenario setup/readback into the existing test-only API.
const scenarioApi = `const e2eApi = {
  scenarioState: () => state,
  setupAmuletLand: () => runWasmInteraction(async () => {
    setAutoPassEnabled(false);
    setHoldRule("always");
    await game.resetEmpty(["Alice", "Bob"], 20);
    await game.addCardToZone(0, "Amulet of Vigor", "battlefield", true);
    await game.addCardToZone(0, "Forest", "battlefield", true);
    for (const player of [0, 1]) {
      for (let i = 0; i < 5; i++) await game.addCardToZone(player, "Forest", "library", true);
    }
    await game.addCardToZone(0, "Simic Growth Chamber", "hand", true);
    await game.finishPuzzleSetup();
    let next = await game.uiState();
    for (let i = 0; i < 30; i++) {
      if (next.phase === "first main phase" && next.active_player === 0) break;
      const action = next.decision?.actions?.find(action =>
        ["keep_opening_hand", "begin_game", "continue_pregame", "pass_priority"].includes(action.kind));
      if (!action) throw new Error("Cannot advance scenario: " + JSON.stringify(next.decision));
      next = await game.dispatch({ type: "priority_action", action_ref: action.action_ref });
    }
    await finalizeState(game, next);
  }),`;

test("clicking Amulet-untapped Growth Chamber's inspector adds mana before its return trigger resolves", { timeout: 90000 }, async () => {
  process.env.VITE_E2E_TEST = "true";
  const server = await createServer({
    root,
    logLevel: "error",
    server: { host: "127.0.0.1", port: 0, hmr: false },
    plugins: [{
      name: "amulet-land-scenario",
      transform(code, id) {
        if (!id.endsWith("/context/GameContext.jsx")) return;
        assert.ok(code.includes("const e2eApi = {"));
        return code.replace("const e2eApi = {", scenarioApi);
      },
    }],
  });
  await server.listen();
  const browser = await chromium.launch();
  try {
    const page = await browser.newPage({ viewport: { width: 1600, height: 1000 } });
    const errors = [];
    page.on("pageerror", error => errors.push(error.message));
    await page.goto(`http://127.0.0.1:${server.httpServer.address().port}/`);
    await page.waitForFunction(() => window.__ironsmithE2E?.snapshot()?.loading === false);
    await page.waitForTimeout(1500); // Let the initial board presentation finish before replacing the game.
    await page.evaluate(() => window.__ironsmithE2E.setupAmuletLand());
    await page.waitForTimeout(150);
    await page.evaluate(() => window.__ironsmithE2E.dispatch({
      type: "priority_action",
      action_ref: { kind: "play_land", land_id: window.__ironsmithE2E.scenarioState().players[0].hand_cards[0].id },
    }));
    await page.waitForFunction(() => window.__ironsmithE2E.scenarioState()?.decision?.kind === "select_options");
    await page.waitForTimeout(150);
    await page.evaluate(() => {
      const options = window.__ironsmithE2E.scenarioState().decision.options;
      const amulet = options.find(option => option.description.includes("Amulet of Vigor"));
      const bounce = options.find(option => option !== amulet);
      // Leftmost is the top of the stack: Amulet must resolve first.
      return window.__ironsmithE2E.dispatch({ type: "select_options", option_indices: [amulet.index, bounce.index] });
    });
    for (let i = 0; i < 2; i++) {
      await page.waitForTimeout(150);
      await page.evaluate(() => window.__ironsmithE2E.dispatch({ type: "priority_action", action_ref: { kind: "pass_priority" } }));
    }
    await page.waitForFunction(() => window.__ironsmithE2E.scenarioState()?.decision?.actions?.some(action =>
      action.kind === "activate_mana_ability" && action.label.includes("Simic Growth Chamber")));
    const before = await page.evaluate(() => window.__ironsmithE2E.scenarioState());
    const land = before.players[0].battlefield.find(card => card.name === "Simic Growth Chamber");
    assert.equal(land.tapped, false);
    assert.equal(before.stack_objects.length, 1);
    assert.match(before.stack_objects[0].ability_text, /return a land/);
    assert.equal(before.players[0].hand_size, 0); // Empty fan still has its outside-click listener.
    await page.locator(`.battlefield-row-card[data-object-id="${land.id}"]`).first().click();
    const ability = page.getByRole("button", { name: /Activate Simic Growth Chamber:.*Add/ }).last();
    await ability.waitFor();
    await page.waitForFunction(() => document.querySelector('.floating-card-preview button[data-available="true"]'));
    // A native click includes pointer-down; button.click() would bypass the bug.
    await ability.click({ timeout: 5000 });
    await page.waitForFunction(() => window.__ironsmithE2E.scenarioState().players[0].mana_pool.green === 1);
    const after = await page.evaluate(() => window.__ironsmithE2E.scenarioState());
    assert.equal(after.players[0].mana_pool.blue, 1);
    assert.equal(after.players[0].battlefield.find(card => card.id === land.id).tapped, true);
    assert.deepEqual(after.stack_objects, before.stack_objects);
    assert.deepEqual(errors, []);
  } finally {
    await browser.close();
    await server.close();
  }
});
