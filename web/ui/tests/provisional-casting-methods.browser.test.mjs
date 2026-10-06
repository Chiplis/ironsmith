import test from "node:test";
import assert from "node:assert/strict";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";
import { createServer } from "vite";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const scenarioApi = `const e2eApi = {
  scenarioState: () => state,
  setupFreeCast: () => runWasmInteraction(async () => {
    setAutoPassEnabled(false);
    setHoldRule("always");
    await game.resetEmpty(["Alice", "Bob"], 20);
    await game.addCardToZone(0, "Omniscience", "battlefield", true);
    await game.addCardToZone(1, "Ornithopter", "battlefield", true);
    await game.addCardToZone(1, "Ornithopter", "battlefield", true);
    await game.addCardToZone(0, "Unsummon", "hand", true);
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

test("a provisional generic cast keeps Omniscience's chooser, while an explicit normal choice stays paid", { timeout: 90000 }, async () => {
  process.env.VITE_E2E_TEST = "true";
  const server = await createServer({ root, logLevel: "error",
    server: { host: "127.0.0.1", port: 0, hmr: false },
    plugins: [{ name: "provisional-casting-scenario", transform(code, id) {
      if (!id.endsWith("/context/GameContext.jsx")) return;
      return code.replace("const e2eApi = {", scenarioApi);
    } }],
  });
  await server.listen();
  const browser = await chromium.launch();
  try {
    const page = await browser.newPage({ viewport: { width: 1600, height: 1000 } });
    const errors = [];
    page.on("pageerror", error => errors.push(error.message));
    for (const explicitNormal of [false, true]) {
      await page.goto(`http://127.0.0.1:${server.httpServer.address().port}/`);
      await page.waitForFunction(() => window.__ironsmithE2E?.snapshot()?.loading === false);
      await page.waitForTimeout(1500);
      await page.evaluate(() => window.__ironsmithE2E.setupFreeCast());
      await page.waitForFunction(() => window.__ironsmithE2E.scenarioState()?.players?.[0]?.hand_cards?.some(card => card.name === "Unsummon"));
      await page.waitForTimeout(150);
      await page.evaluate(explicit => {
        const state = window.__ironsmithE2E.scenarioState();
        const card = state.players[0].hand_cards.find(card => card.name === "Unsummon");
        const actions = state.decision.actions.filter(action => action.object_id === card.id && action.kind === "cast_spell");
        const normal = actions.find(action => action.action_ref.casting_method.kind === "normal");
        if (normal.payment_proven !== false) throw new Error("Normal cast must still be unproven");
        if (actions.length !== 2) throw new Error("Both prices must be offered immediately");
        // A held gesture can predate alternative discovery. Request only its
        // original normal action to exercise the generic fallback chooser.
        window.dispatchEvent(new CustomEvent("ironsmith:hand-card-keyboard-cast", {
          detail: { objectId: card.id, cardName: card.name, card,
            actions: explicit ? actions : [normal],
            anchorRect: { left: 500, top: 500, right: 600, bottom: 600, width: 100, height: 100 },
          },
        }));
      }, explicitNormal);
      if (explicitNormal) {
        const chooser = page.getByRole("dialog", { name: "Ways to play Unsummon" });
        await chooser.waitFor();
        await chooser.locator("[data-action-row]").filter({ hasText: /^Cast Unsummon$/ }).click();
        await page.waitForFunction(() => window.__ironsmithE2E.scenarioState()?.decision?.kind === "targets");
      } else {
        await page.waitForFunction(() => window.__ironsmithE2E.scenarioState()?.decision?.kind === "select_options");
        await page.getByRole("button", { name: "Cast for free", exact: true }).click();
        await page.waitForFunction(() => window.__ironsmithE2E.scenarioState()?.decision?.kind === "targets");
      }
      await page.waitForTimeout(150);
      await page.evaluate(() => {
        const creature = window.__ironsmithE2E.scenarioState().players[1].battlefield.find(card => card.name === "Ornithopter");
        return window.__ironsmithE2E.dispatch({ type: "select_targets", targets: [{ kind: "object", object: creature.id }] });
      });
      await page.waitForFunction(paid => window.__ironsmithE2E.scenarioState()?.decision?.kind === (paid ? "mana_payment" : "priority"), explicitNormal);
      const after = await page.evaluate(() => window.__ironsmithE2E.scenarioState());
      if (explicitNormal) assert.equal(after.mana_payment.can_confirm, false);
      else assert.ok(after.stack_objects.some(card => card.name === "Unsummon"));
    }
    assert.deepEqual(errors, []);
  } finally {
    await browser.close();
    await server.close();
  }
});
