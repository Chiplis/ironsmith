import test from "node:test";
import assert from "node:assert/strict";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";
import { createServer } from "vite";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const scenarioApi = `const e2eApi = {
  scenarioState: () => state,
  setupUnfundedPayment: () => runWasmInteraction(async () => {
    setAutoPassEnabled(false);
    setHoldRule("always");
    await game.resetEmpty(["Alice", "Bob"], 20);
    for (const name of ["Plains", "Island", "Swamp"]) {
      await game.addCardToZone(0, name, "battlefield", true);
    }
    await game.addCardToZone(0, "Tenured Inkcaster", "hand", true);
    await game.finishPuzzleSetup();
    let next = await game.uiState();
    for (let i = 0; i < 30; i++) {
      if (next.phase === "first main phase" && next.active_player === 0) break;
      const action = next.decision?.actions?.find(action =>
        ["keep_opening_hand", "begin_game", "continue_pregame", "pass_priority"].includes(action.kind));
      if (!action) throw new Error("Cannot advance scenario: " + JSON.stringify(next.decision));
      next = await game.dispatch({ type: "priority_action", action_ref: action.action_ref });
    }
    game.getPaymentActivationOptions = async () => { throw new Error("Payment options unavailable in regression"); };
    game.analyzePayment = async () => { throw new Error("Payment ranking unavailable in regression"); };
    await finalizeState(game, next);
  }),`;

test("real payment editor activates clicked lands when background options and ranking fail, and cancel restores them", { timeout: 90000 }, async () => {
  process.env.VITE_E2E_TEST = "true";
  const server = await createServer({
    root,
    logLevel: "error",
    server: { host: "127.0.0.1", port: 0, hmr: false },
    plugins: [{
      name: "mana-payment-land-scenario",
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
    await page.waitForTimeout(1500);
    await page.evaluate(() => window.__ironsmithE2E.setupUnfundedPayment());
    await page.waitForTimeout(150);
    const before = await page.evaluate(() => window.__ironsmithE2E.scenarioState());
    await page.evaluate(() => {
      const state = window.__ironsmithE2E.scenarioState();
      const spell = state.players[0].hand_cards.find(card => card.name === "Tenured Inkcaster");
      const action = state.decision.actions.find(action => action.kind === "cast_spell" && action.object_id === spell.id);
      if (!action) throw new Error("Missing timing-legal spell: " + JSON.stringify(state.decision));
      return window.__ironsmithE2E.dispatch({
        type: "priority_action", action_ref: action.action_ref,
      });
    });
    await page.waitForFunction(() => ["mana_payment", "select_options"].includes(window.__ironsmithE2E.scenarioState()?.decision?.kind));
    if (await page.evaluate(() => window.__ironsmithE2E.scenarioState().decision.kind === "select_options")) {
      await page.waitForTimeout(150);
      await page.evaluate(() => {
        const options = window.__ironsmithE2E.scenarioState().decision.options;
        const normal = options.find(option => /normal cast/i.test(option.description));
        if (!normal) throw new Error("Missing normal casting method: " + JSON.stringify(options));
        return window.__ironsmithE2E.dispatch({ type: "select_options", option_indices: [normal.index] });
      });
    }
    await page.waitForFunction(() => window.__ironsmithE2E.scenarioState()?.decision?.kind === "mana_payment");
    await page.waitForFunction(() => {
      const payment = window.__ironsmithE2E.scenarioState().mana_payment;
      return payment.mana_abilities.length === 3 && payment.activation_options_error === true;
    });
    for (const [name, color] of [["Island", "blue"], ["Plains", "white"], ["Swamp", "black"]]) {
      const land = before.players[0].battlefield.find(card => card.name === name);
      await page.locator(`.battlefield-row-card[data-object-id="${land.id}"]`).first().click();
      await page.waitForFunction(({ id, color }) => {
        const state = window.__ironsmithE2E.scenarioState();
        return state.players[0].mana_pool[color] === 1
          && state.players[0].battlefield.find(card => card.id === id)?.tapped === true;
      }, { id: land.id, color }, { timeout: 5000 });
      assert.equal(await page.getByRole("dialog", { name: "Activate mana ability" }).count(), 0);
    }
    const paid = await page.evaluate(() => window.__ironsmithE2E.scenarioState());
    assert.equal(paid.mana_payment.can_confirm, false, "three lands cannot fund the five-mana spell");
    await page.getByRole("button", { name: "Cancel", exact: true }).click();
    await page.waitForFunction(() => window.__ironsmithE2E.scenarioState()?.decision?.kind === "priority");
    const after = await page.evaluate(() => window.__ironsmithE2E.scenarioState());
    assert.ok(after.players[0].hand_cards.some(card => card.name === "Tenured Inkcaster"));
    assert.ok(after.players[0].battlefield.every(card => !card.tapped));
    for (const color of ["white", "blue", "black"]) assert.equal(after.players[0].mana_pool[color], 0);
    assert.deepEqual(errors, []);
  } finally {
    await browser.close();
    await server.close();
  }
});
