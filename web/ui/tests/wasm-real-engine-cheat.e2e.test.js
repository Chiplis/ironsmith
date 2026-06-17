import test from "node:test";
import assert from "node:assert/strict";
import net from "node:net";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";
import { createServer as createViteServer } from "vite";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const UI_ROOT = path.resolve(__dirname, "..");
const WASM_MODULE_URL = `/@fs/${path.resolve(UI_ROOT, "../wasm_demo/pkg/ironsmith.js")}`;

function hiddenManifest(owner, count) {
  return {
    owner,
    deckCount: count,
    commitmentRoot: `root-${owner}`,
    decklistHash: `deck-${owner}`,
    slotCommitments: Array.from({ length: count }, (_, slot) => ({
      slot,
      commitment: `commitment-${owner}-${slot}`,
    })),
  };
}

async function freePort() {
  return new Promise((resolve, reject) => {
    const server = net.createServer();
    server.once("error", reject);
    server.listen(0, "127.0.0.1", () => {
      const address = server.address();
      const port = typeof address === "object" && address ? address.port : 0;
      server.close(() => resolve(port));
    });
  });
}

async function startWasmServer() {
  const vitePort = await freePort();
  const vite = await createViteServer({
    root: UI_ROOT,
    configFile: path.join(UI_ROOT, "vite.config.js"),
    clearScreen: false,
    logLevel: "silent",
    server: {
      host: "127.0.0.1",
      port: vitePort,
      strictPort: true,
      hmr: false,
      watch: null,
    },
  });
  await vite.listen();
  return {
    vite,
    baseUrl: `http://127.0.0.1:${vitePort}`,
  };
}

test("real WASM engine rejects a forged cast for a card outside the actor's hand", { timeout: 30000 }, async () => {
  const { vite, baseUrl } = await startWasmServer();
  let browser = null;

  try {
    browser = await chromium.launch();
    const page = await browser.newPage();
    const pageErrors = [];
    page.on("pageerror", (error) => pageErrors.push(String(error?.stack || error)));
    page.on("console", (message) => {
      if (message.type() === "error") pageErrors.push(message.text());
    });

    await page.goto(baseUrl);
    const result = await page.evaluate(async ({ wasmModuleUrl }) => {
      const mod = await import(wasmModuleUrl);
      await mod.default();
      const game = new mod.WasmGame();
      let state = game.startMatch({
        playerNames: ["Alice", "Bob"],
        startingLife: 20,
        seed: 1,
        format: "normal",
        openingHandSize: 7,
        decks: [
          Array(60).fill("Ornithopter"),
          Array(60).fill("Ornithopter"),
        ],
      });

      for (let step = 0; step < 12; step += 1) {
        if (state.decision?.actions?.some((action) => action.action_ref?.kind === "cast_spell")) {
          break;
        }
        const action = state.decision?.actions?.find((candidate) => (
          candidate.action_ref?.kind === "keep_opening_hand"
          || candidate.action_ref?.kind === "continue_pregame"
          || candidate.action_ref?.kind === "begin_game"
          || candidate.action_ref?.kind === "pass_priority"
        ));
        if (!action) {
          throw new Error(`could not advance to a cast decision: ${JSON.stringify(state.decision)}`);
        }
        state = game.dispatch({ type: "priority_action", action_ref: action.action_ref });
      }

      const decision = state.decision;
      if (!decision?.actions?.some((action) => action.action_ref?.kind === "cast_spell")) {
        throw new Error(`real engine did not expose a castable hand spell: ${JSON.stringify(decision)}`);
      }

      const checkpointBefore = game.exportSyncCheckpoint();
      const actor = Number(decision.player);
      const libraryCardId = checkpointBefore.players[actor].library[0];
      const libraryObject = checkpointBefore.objects.find((object) => object.id === libraryCardId);
      const legalHandSpellIds = decision.actions
        .filter((action) => action.action_ref?.kind === "cast_spell")
        .map((action) => action.action_ref.spell_id);
      const forgedCommand = {
        type: "priority_action",
        action_ref: {
          kind: "cast_spell",
          spell_id: libraryCardId,
          from_zone: "hand",
          casting_method: { kind: "normal" },
        },
      };

      let rejectedError = null;
      try {
        game.dispatch(forgedCommand);
      } catch (error) {
        rejectedError = String(error?.message || error);
      }

      return {
        actor,
        rejectedError,
        forgedCommand,
        libraryObject: {
          id: libraryObject?.id,
          name: libraryObject?.name,
          zone: libraryObject?.zone,
        },
        legalHandSpellIds,
        stateUnchanged: JSON.stringify(game.exportSyncCheckpoint()) === JSON.stringify(checkpointBefore),
      };
    }, { wasmModuleUrl: WASM_MODULE_URL });

    assert.equal(result.actor, 0);
    assert.equal(result.libraryObject.name, "Ornithopter");
    assert.equal(result.libraryObject.zone, "library");
    assert.equal(result.legalHandSpellIds.includes(result.forgedCommand.action_ref.spell_id), false);
    assert.match(result.rejectedError, /invalid priority action ref/);
    assert.equal(result.stateUnchanged, true);
    assert.deepEqual(pageErrors, []);
  } finally {
    await browser?.close();
    await vite.close();
  }
});

test("real WASM engine emits crypto requirements when a hidden committed card is played", { timeout: 30000 }, async () => {
  const { vite, baseUrl } = await startWasmServer();
  let browser = null;

  try {
    browser = await chromium.launch();
    const page = await browser.newPage();
    const pageErrors = [];
    page.on("pageerror", (error) => pageErrors.push(String(error?.stack || error)));
    page.on("console", (message) => {
      if (message.type() === "error") pageErrors.push(message.text());
    });

    await page.goto(baseUrl);
    const result = await page.evaluate(async ({ wasmModuleUrl, manifests }) => {
      const mod = await import(wasmModuleUrl);
      await mod.default();
      const game = new mod.WasmGame();
      let state = game.startMatch({
        playerNames: ["Alice", "Bob"],
        startingLife: 20,
        seed: 1,
        format: "normal",
        openingHandSize: 7,
        decks: [
          Array(60).fill("Ornithopter"),
          Array(60).fill("Ornithopter"),
        ],
        hiddenDeckManifests: manifests,
      });

      for (let step = 0; step < 12; step += 1) {
        if (state.decision?.actions?.some((action) => action.action_ref?.kind === "cast_spell")) {
          break;
        }
        const action = state.decision?.actions?.find((candidate) => (
          candidate.action_ref?.kind === "keep_opening_hand"
          || candidate.action_ref?.kind === "continue_pregame"
          || candidate.action_ref?.kind === "begin_game"
          || candidate.action_ref?.kind === "pass_priority"
        ));
        if (!action) {
          throw new Error(`could not advance to a cast decision: ${JSON.stringify(state.decision)}`);
        }
        state = game.dispatch({ type: "priority_action", action_ref: action.action_ref });
      }

      const castAction = state.decision?.actions?.find(
        (action) => action.action_ref?.kind === "cast_spell"
      );
      if (!castAction) {
        throw new Error(`real engine did not expose a castable hand spell: ${JSON.stringify(state.decision)}`);
      }
      state = game.dispatch({
        type: "priority_action",
        action_ref: castAction.action_ref,
      });

      return {
        requirements: state.crypto_requirements || state.cryptoRequirements || [],
      };
    }, {
      wasmModuleUrl: WASM_MODULE_URL,
      manifests: [hiddenManifest(0, 60), hiddenManifest(1, 60)],
    });

    assert.equal(
      result.requirements.some((requirement) => requirement.type === "public_open"),
      true,
      JSON.stringify(result.requirements)
    );
    assert.deepEqual(pageErrors, []);
  } finally {
    await browser?.close();
    await vite.close();
  }
});

test("real WASM engine keeps Tainted Pact prompt after post-resolution hidden opening", { timeout: 30000 }, async () => {
  const { vite, baseUrl } = await startWasmServer();
  let browser = null;

  try {
    browser = await chromium.launch();
    const page = await browser.newPage();
    const pageErrors = [];
    page.on("pageerror", (error) => pageErrors.push(String(error?.stack || error)));
    page.on("console", (message) => {
      if (message.type() === "error") pageErrors.push(message.text());
    });

    await page.goto(baseUrl);
    const result = await page.evaluate(async ({ wasmModuleUrl, manifests }) => {
      const mod = await import(wasmModuleUrl);
      await mod.default();
      const game = new mod.WasmGame();
      let state = game.startMatch({
        playerNames: ["Alice", "Bob"],
        startingLife: 20,
        seed: 1,
        format: "normal",
        openingHandSize: 7,
        decks: [
          Array(60).fill("Island"),
          Array(60).fill("Mountain"),
        ],
        hiddenDeckManifests: manifests,
      });
      const taintedPactId = game.addCardToZone(0, "Tainted Pact", "hand", true);
      game.addCardToZone(0, "Black Lotus", "battlefield", true);
      state = game.uiState();

      function firstAction(predicate) {
        return (state.decision?.actions || []).find(predicate) || null;
      }

      function passAction() {
        return firstAction((action) => action.action_ref?.kind === "pass_priority");
      }

      function setupAction() {
        return firstAction((action) => [
          "keep_opening_hand",
          "continue_pregame",
          "begin_game",
          "pass_priority",
        ].includes(action.action_ref?.kind));
      }

      function dispatchAction(action) {
        state = game.dispatch({ type: "priority_action", action_ref: action.action_ref });
      }

      for (let step = 0; step < 40; step += 1) {
        const castTainted = firstAction((action) =>
          action.action_ref?.kind === "cast_spell"
          && Number(action.action_ref?.spell_id) === Number(taintedPactId)
        );
        if (castTainted) break;
        const action = setupAction();
        if (!action) {
          throw new Error(`could not advance to Tainted Pact cast action: ${JSON.stringify(state.decision)}`);
        }
        dispatchAction(action);
      }

      const castTainted = firstAction((action) =>
        action.action_ref?.kind === "cast_spell"
        && Number(action.action_ref?.spell_id) === Number(taintedPactId)
      );
      if (!castTainted) {
        throw new Error(`Tainted Pact was not castable: ${JSON.stringify(state.decision)}`);
      }
      dispatchAction(castTainted);

      for (let step = 0; step < 20; step += 1) {
        if (state.decision?.kind === "priority" && /Tainted Pact/i.test(JSON.stringify(state.stack_preview || []))) {
          break;
        }
        if (state.decision?.kind !== "select_options") {
          throw new Error(`unexpected payment decision: ${JSON.stringify(state.decision)}`);
        }
        const legal = (state.decision.options || []).filter((option) => option.legal !== false);
        const option =
          legal.find((candidate) => /Black Lotus/i.test(candidate.description || ""))
          || legal.find((candidate) => /^black$/i.test(candidate.description || ""))
          || legal.find((candidate) => /black|\{B\}|from mana pool|pay/i.test(candidate.description || ""))
          || legal[0];
        if (!option) {
          throw new Error(`no legal payment option: ${JSON.stringify(state.decision)}`);
        }
        state = game.dispatch({ type: "select_options", option_indices: [option.index] });
      }

      if (!(state.decision?.kind === "priority" && /Tainted Pact/i.test(JSON.stringify(state.stack_preview || [])))) {
        throw new Error(`Tainted Pact did not reach the stack: ${JSON.stringify(state)}`);
      }

      for (let step = 0; step < 4; step += 1) {
        const pass = passAction();
        if (!pass) {
          throw new Error(`expected pass priority while resolving Tainted Pact: ${JSON.stringify(state.decision)}`);
        }
        dispatchAction(pass);
        if (
          state.decision?.kind === "select_options"
          && /put .* into your hand/i.test(state.decision.description || "")
        ) {
          break;
        }
      }

      if (
        state.decision?.kind !== "select_options"
        || !/put .* into your hand/i.test(state.decision.description || "")
      ) {
        throw new Error(`Tainted Pact did not pause for the exiled card: ${JSON.stringify(state.decision)}`);
      }

      const publicOpen = (state.crypto_requirements || []).find((requirement) =>
        requirement.type === "public_open" && requirement.owner === 0 && requirement.slot != null
      );
      if (!publicOpen) {
        throw new Error(`expected public opening for Tainted Pact exiled card: ${JSON.stringify(state.crypto_requirements)}`);
      }

      state = game.revealHiddenSlot({
        owner: publicOpen.owner,
        slot: publicOpen.slot,
        cardName: publicOpen.card,
        commitment: publicOpen.commitment,
      });

      const decisionAfterReveal = state.decision || null;
      const yesOption = (decisionAfterReveal?.options || []).find((option) =>
        option.index === 1 && option.legal !== false
      );
      if (
        decisionAfterReveal?.kind !== "select_options"
        || !/put .* into your hand/i.test(decisionAfterReveal.description || "")
        || !yesOption
      ) {
        throw new Error(`hidden opening cleared Tainted Pact prompt: ${JSON.stringify(decisionAfterReveal)}`);
      }

      state = game.dispatch({ type: "select_options", option_indices: [1] });
      return {
        promptAfterReveal: decisionAfterReveal.description,
        handContainsOpenedCard: (state.players?.[0]?.hand_cards || []).some((card) =>
          card.name === publicOpen.card
        ),
        graveyardContainsSpell: (state.players?.[0]?.graveyard_cards || []).some((card) =>
          card.name === "Tainted Pact"
        ),
      };
    }, {
      wasmModuleUrl: WASM_MODULE_URL,
      manifests: [hiddenManifest(0, 60), hiddenManifest(1, 60)],
    });

    assert.match(result.promptAfterReveal, /put .* into your hand/i);
    assert.equal(result.handContainsOpenedCard, true);
    assert.equal(result.graveyardContainsSpell, true);
    assert.deepEqual(pageErrors, []);
  } finally {
    await browser?.close();
    await vite.close();
  }
});

test("real WASM engine opens Tainted Pact duplicate-stop exile card", { timeout: 30000 }, async () => {
  const { vite, baseUrl } = await startWasmServer();
  let browser = null;

  try {
    browser = await chromium.launch();
    const page = await browser.newPage();
    const pageErrors = [];
    page.on("pageerror", (error) => pageErrors.push(String(error?.stack || error)));
    page.on("console", (message) => {
      if (message.type() === "error") pageErrors.push(message.text());
    });

    await page.goto(baseUrl);
    const result = await page.evaluate(async ({ wasmModuleUrl, manifests }) => {
      const mod = await import(wasmModuleUrl);
      await mod.default();
      const game = new mod.WasmGame();
      let state = game.startMatch({
        playerNames: ["Alice", "Bob"],
        startingLife: 20,
        seed: 1,
        format: "normal",
        openingHandSize: 7,
        decks: [
          Array(60).fill("Island"),
          Array(60).fill("Mountain"),
        ],
        hiddenDeckManifests: manifests,
      });
      const taintedPactId = game.addCardToZone(0, "Tainted Pact", "hand", true);
      game.addCardToZone(0, "Black Lotus", "battlefield", true);
      state = game.uiState();

      function firstAction(predicate) {
        return (state.decision?.actions || []).find(predicate) || null;
      }

      function passAction() {
        return firstAction((action) => action.action_ref?.kind === "pass_priority");
      }

      function dispatchAction(action) {
        state = game.dispatch({ type: "priority_action", action_ref: action.action_ref });
      }

      for (let step = 0; step < 40; step += 1) {
        const castTainted = firstAction((action) =>
          action.action_ref?.kind === "cast_spell"
          && Number(action.action_ref?.spell_id) === Number(taintedPactId)
        );
        if (castTainted) break;
        const action = firstAction((candidate) => [
          "keep_opening_hand",
          "continue_pregame",
          "begin_game",
          "pass_priority",
        ].includes(candidate.action_ref?.kind));
        if (!action) {
          throw new Error(`could not advance to Tainted Pact cast action: ${JSON.stringify(state.decision)}`);
        }
        dispatchAction(action);
      }

      const castTainted = firstAction((action) =>
        action.action_ref?.kind === "cast_spell"
        && Number(action.action_ref?.spell_id) === Number(taintedPactId)
      );
      if (!castTainted) {
        throw new Error(`Tainted Pact was not castable: ${JSON.stringify(state.decision)}`);
      }
      dispatchAction(castTainted);

      for (let step = 0; step < 20; step += 1) {
        if (state.decision?.kind === "priority" && /Tainted Pact/i.test(JSON.stringify(state.stack_preview || []))) {
          break;
        }
        if (state.decision?.kind !== "select_options") {
          throw new Error(`unexpected payment decision: ${JSON.stringify(state.decision)}`);
        }
        const legal = (state.decision.options || []).filter((option) => option.legal !== false);
        const option =
          legal.find((candidate) => /Black Lotus/i.test(candidate.description || ""))
          || legal.find((candidate) => /^black$/i.test(candidate.description || ""))
          || legal.find((candidate) => /black|\{B\}|from mana pool|pay/i.test(candidate.description || ""))
          || legal[0];
        if (!option) {
          throw new Error(`no legal payment option: ${JSON.stringify(state.decision)}`);
        }
        state = game.dispatch({ type: "select_options", option_indices: [option.index] });
      }

      if (!(state.decision?.kind === "priority" && /Tainted Pact/i.test(JSON.stringify(state.stack_preview || [])))) {
        throw new Error(`Tainted Pact did not reach the stack: ${JSON.stringify(state)}`);
      }
      const stackTransitionLabels = (state.zone_transitions || []).map((transition) =>
        `${transition.from_zone || transition.fromZone}->${transition.to_zone || transition.toZone}:${transition.card?.name || ""}`
      );

      for (let step = 0; step < 4; step += 1) {
        const pass = passAction();
        if (!pass) {
          throw new Error(`expected pass priority while resolving Tainted Pact: ${JSON.stringify(state.decision)}`);
        }
        dispatchAction(pass);
        if (
          state.decision?.kind === "select_options"
          && /put .* into your hand/i.test(state.decision.description || "")
        ) {
          break;
        }
      }

      const firstOpen = (state.crypto_requirements || []).find((requirement) =>
        requirement.type === "public_open" && requirement.owner === 0 && requirement.slot != null
      );
      if (!firstOpen?.card) {
        throw new Error(`expected first public opening for Tainted Pact: ${JSON.stringify(state.crypto_requirements)}`);
      }
      state = game.revealHiddenSlot({
        owner: firstOpen.owner,
        slot: firstOpen.slot,
        cardName: firstOpen.card,
        commitment: firstOpen.commitment,
      });
      state = game.dispatch({ type: "select_options", option_indices: [0] });

      if (
        state.decision?.kind === "select_options"
        && /put .* into your hand/i.test(state.decision.description || "")
      ) {
        throw new Error(`duplicate Tainted Pact cards should stop without another put prompt: ${JSON.stringify(state.decision)}`);
      }

      const duplicateOpen = (state.crypto_requirements || []).find((requirement) =>
        requirement.type === "public_open"
        && requirement.owner === 0
        && requirement.slot != null
        && Number(requirement.objectId) !== Number(firstOpen.objectId)
      );
      if (!duplicateOpen?.card) {
        throw new Error(`expected duplicate-stop public opening: ${JSON.stringify(state.crypto_requirements)}`);
      }

      state = game.revealHiddenSlot({
        owner: duplicateOpen.owner,
        slot: duplicateOpen.slot,
        cardName: duplicateOpen.card,
        commitment: duplicateOpen.commitment,
      });

      return {
        duplicateCard: duplicateOpen.card,
        stackTransitionLabels,
        finalTransitionLabels: (state.zone_transitions || []).map((transition) =>
          `${transition.from_zone || transition.fromZone}->${transition.to_zone || transition.toZone}:${transition.card?.name || ""}`
        ),
        viewedCardNames: (state.viewed_cards?.cards || []).map((card) => card.name),
        viewedCardLabels: (state.viewed_cards?.cards || []).map((card) => card.name || `Card #${card.id}`),
        exileNames: (state.players?.[0]?.exile_cards || []).map((card) => card.name),
        graveyardContainsSpell: (state.players?.[0]?.graveyard_cards || []).some((card) =>
          card.name === "Tainted Pact"
        ),
      };
    }, {
      wasmModuleUrl: WASM_MODULE_URL,
      manifests: [hiddenManifest(0, 60), hiddenManifest(1, 60)],
    });

    assert.equal(result.duplicateCard, "Island");
    assert.ok(
      result.stackTransitionLabels.some((label) => /^hand->stack:Tainted Pact$/i.test(label)),
      `expected hand-to-stack transition, got ${JSON.stringify(result.stackTransitionLabels)}`,
    );
    assert.ok(
      result.finalTransitionLabels.some((label) => /^library->exile:Island$/i.test(label)),
      `expected library-to-exile transition, got ${JSON.stringify(result.finalTransitionLabels)}`,
    );
    assert.ok(
      result.finalTransitionLabels.some((label) => /^stack->graveyard:Tainted Pact$/i.test(label)),
      `expected stack-to-graveyard transition, got ${JSON.stringify(result.finalTransitionLabels)}`,
    );
    assert.equal(result.viewedCardNames.filter((name) => name === "Island").length, 2);
    assert.deepEqual(result.viewedCardLabels.filter((name) => /^Card #/i.test(name)), []);
    assert.equal(result.exileNames.filter((name) => name === "Island").length, 2);
    assert.equal(result.graveyardContainsSpell, true);
    assert.deepEqual(pageErrors, []);
  } finally {
    await browser?.close();
    await vite.close();
  }
});

test("real WASM engine redacts committed hand cards after private reveal", { timeout: 30000 }, async () => {
  const { vite, baseUrl } = await startWasmServer();
  let browser = null;

  try {
    browser = await chromium.launch();
    const page = await browser.newPage();
    const pageErrors = [];
    page.on("pageerror", (error) => pageErrors.push(String(error?.stack || error)));
    page.on("console", (message) => {
      if (message.type() === "error") pageErrors.push(message.text());
    });

    await page.goto(baseUrl);
    const result = await page.evaluate(async ({ wasmModuleUrl, manifests }) => {
      const mod = await import(wasmModuleUrl);
      await mod.default();
      const game = new mod.WasmGame();
      game.startMatch({
        playerNames: ["Alice", "Bob"],
        startingLife: 20,
        seed: 1,
        format: "normal",
        openingHandSize: 7,
        decks: [
          Array(60).fill("Island"),
          Array(60).fill("Mountain"),
        ],
        hiddenDeckManifests: manifests,
      });

      const before = game.exportSyncCheckpoint();
      const bobHandObjectId = before.players[1].hand[0];
      const opening = game.exportHiddenCardOpening(BigInt(bobHandObjectId));
      game.revealHiddenSlot({
        owner: opening.owner,
        slot: opening.slot,
        cardName: opening.card,
        commitment: opening.commitment,
      });

      const redactedForAlice = game.exportRedactedSyncCheckpoint(0);
      const redactedObject = redactedForAlice.objects.find((object) => object.id === bobHandObjectId);
      return {
        opening,
        redactedObject,
      };
    }, {
      wasmModuleUrl: WASM_MODULE_URL,
      manifests: [hiddenManifest(0, 60), hiddenManifest(1, 60)],
    });

    assert.equal(result.opening.owner, 1);
    assert.equal(result.opening.card, "Mountain");
    assert.equal(result.redactedObject.name, "Hidden Card");
    assert.equal(result.redactedObject.hiddenCard.commitment, result.opening.commitment);
    assert.deepEqual(pageErrors, []);
  } finally {
    await browser?.close();
    await vite.close();
  }
});

test("real WASM engine emits private openings for committed scry and surveil inspections", { timeout: 30000 }, async () => {
  const { vite, baseUrl } = await startWasmServer();
  let browser = null;

  try {
    browser = await chromium.launch();
    const page = await browser.newPage();
    const pageErrors = [];
    page.on("pageerror", (error) => pageErrors.push(String(error?.stack || error)));
    page.on("console", (message) => {
      if (message.type() === "error") pageErrors.push(message.text());
    });

    await page.goto(baseUrl);
    const result = await page.evaluate(async ({ wasmModuleUrl, manifests }) => {
      const mod = await import(wasmModuleUrl);
      await mod.default();

      function dispatchPriority(game, action) {
        return game.dispatch({
          type: "priority_action",
          action_ref: action.action_ref,
        });
      }

      function advanceToInspectionPrompt({ spellName, landName, fillerName }) {
        const game = new mod.WasmGame();
        let state = game.startMatch({
          playerNames: ["Alice", "Bob"],
          startingLife: 20,
          seed: 1,
          format: "normal",
          openingHandSize: 7,
          decks: [
            [landName, spellName, landName, landName, landName, landName, landName, landName, landName, landName],
            Array(10).fill(fillerName),
          ],
          hiddenDeckManifests: manifests,
        });

        for (let step = 0; step < 20; step += 1) {
          const actions = state.decision?.actions || [];
          let action = actions.find((candidate) => (
            candidate.action_ref?.kind === "keep_opening_hand"
            || candidate.action_ref?.kind === "continue_pregame"
            || candidate.action_ref?.kind === "begin_game"
          ));
          if (!action) action = actions.find((candidate) => candidate.action_ref?.kind === "play_land");
          if (!action && actions.some((candidate) => candidate.label?.includes(spellName))) break;
          if (!action) action = actions.find((candidate) => candidate.action_ref?.kind === "pass_priority");
          if (!action) {
            throw new Error(`could not advance to ${spellName}: ${JSON.stringify(state.decision)}`);
          }
          state = dispatchPriority(game, action);
        }

        const castAction = state.decision?.actions?.find((action) => action.label?.includes(spellName));
        if (!castAction) {
          throw new Error(`could not cast ${spellName}: ${JSON.stringify(state.decision)}`);
        }
        state = dispatchPriority(game, castAction);

        if (state.decision?.kind !== "select_options") {
          throw new Error(`${spellName} did not ask for mana payment: ${JSON.stringify(state.decision)}`);
        }
        state = game.dispatch({ type: "select_options", option_indices: [0] });

        for (let step = 0; step < 10; step += 1) {
          if (state.decision?.kind === "select_objects") break;
          const action = (state.decision?.actions || []).find(
            (candidate) => candidate.action_ref?.kind === "pass_priority"
          ) || state.decision?.actions?.[0];
          if (!action) {
            throw new Error(`could not reach ${spellName} inspection prompt: ${JSON.stringify(state.decision)}`);
          }
          state = dispatchPriority(game, action);
        }

        return {
          decision: state.decision,
          requirements: state.crypto_requirements || state.cryptoRequirements || [],
          viewedCards: state.viewed_cards || state.viewedCards || null,
        };
      }

      return {
        scry: advanceToInspectionPrompt({
          spellName: "Preordain",
          landName: "Island",
          fillerName: "Mountain",
        }),
        surveil: advanceToInspectionPrompt({
          spellName: "Barrier of Bones",
          landName: "Swamp",
          fillerName: "Mountain",
        }),
      };
    }, {
      wasmModuleUrl: WASM_MODULE_URL,
      manifests: [hiddenManifest(0, 10), hiddenManifest(1, 10)],
    });

    assert.equal(result.scry.decision.kind, "select_objects");
    assert.match(result.scry.decision.description, /Scry 2/);
    assert.equal(
      result.scry.requirements.some((requirement) => (
        requirement.type === "private_view_window"
        && requirement.viewer === 0
        && requirement.owner === 0
        && requirement.zone === "library"
        && requirement.count === 2
      )),
      true,
      JSON.stringify(result.scry.requirements)
    );
    assert.equal(
      result.scry.requirements.filter((requirement) => requirement.type === "private_open").length,
      2,
      JSON.stringify(result.scry.requirements)
    );
    assert.equal(result.scry.viewedCards?.visibility, "private");
    assert.equal(result.scry.viewedCards?.cards?.length, 2);

    assert.equal(result.surveil.decision.kind, "select_objects");
    assert.match(result.surveil.decision.description, /Surveil 1/);
    assert.equal(
      result.surveil.requirements.some((requirement) => (
        requirement.type === "private_view_window"
        && requirement.viewer === 0
        && requirement.owner === 0
        && requirement.zone === "library"
        && requirement.count === 1
      )),
      true,
      JSON.stringify(result.surveil.requirements)
    );
    assert.equal(
      result.surveil.requirements.filter((requirement) => requirement.type === "private_open").length,
      1,
      JSON.stringify(result.surveil.requirements)
    );
    assert.equal(result.surveil.viewedCards?.visibility, "private");
    assert.equal(result.surveil.viewedCards?.cards?.length, 1);
    assert.deepEqual(pageErrors, []);
  } finally {
    await browser?.close();
    await vite.close();
  }
});

test("real WASM engine ziffle position reveal ignores opened commitment metadata", { timeout: 30000 }, async () => {
  const { vite, baseUrl } = await startWasmServer();
  let browser = null;

  try {
    browser = await chromium.launch();
    const page = await browser.newPage();
    const pageErrors = [];
    page.on("pageerror", (error) => pageErrors.push(String(error?.stack || error)));
    page.on("console", (message) => {
      if (message.type() === "error") pageErrors.push(message.text());
    });

    await page.goto(baseUrl);
    const bobManifest = hiddenManifest(1, 0);
    const result = await page.evaluate(async ({ wasmModuleUrl, bobManifest }) => {
      const mod = await import(wasmModuleUrl);
      await mod.default();
      const game = new mod.WasmGame();
      game.startMatch({
        playerNames: ["Alice", "Bob"],
        startingLife: 20,
        seed: 1,
        format: "normal",
        openingHandSize: 2,
        decks: [[], []],
        hiddenDeckManifests: [
          {
            owner: 0,
            deckCount: 2,
            commitmentRoot: "ziffle:test-deck",
            decklistHash: "alice-deck",
            slotCommitments: [
              { slot: 0, commitment: "ziffle:test-deck:0" },
              { slot: 1, commitment: "ziffle:test-deck:1" },
            ],
          },
          bobManifest,
        ],
      });

      game.revealHiddenPosition({
        owner: 0,
        position: 1,
        originalSlot: 0,
        cardName: "Island",
        positionCommitment: "ziffle:test-deck:1",
        commitment: "original-slot-0",
      });
      game.revealHiddenPosition({
        owner: 0,
        position: 0,
        originalSlot: 1,
        cardName: "Mountain",
        positionCommitment: "ziffle:test-deck:0",
        commitment: "original-slot-1",
      });

      const checkpoint = game.exportSyncCheckpoint();
      const redactedForBob = game.exportRedactedSyncCheckpoint(1);
      const handObjects = checkpoint.players[0].hand.map((id) =>
        checkpoint.objects.find((object) => object.id === id)
      );
      const redactedHandObjects = checkpoint.players[0].hand.map((id) =>
        redactedForBob.objects.find((object) => object.id === id)
      );
      return {
        names: handObjects.map((object) => object?.name),
        redactedNames: redactedHandObjects.map((object) => object?.name),
        redactedCommitments: redactedHandObjects.map((object) => object?.hiddenCard?.commitment),
      };
    }, { wasmModuleUrl: WASM_MODULE_URL, bobManifest });

    assert.deepEqual(result.names.sort(), ["Island", "Mountain"]);
    assert.deepEqual(result.redactedNames.sort(), ["Hidden Card", "Hidden Card"]);
    assert.deepEqual(result.redactedCommitments.sort(), ["original-slot-0", "original-slot-1"]);
    assert.deepEqual(pageErrors, []);
  } finally {
    await browser?.close();
    await vite.close();
  }
});
