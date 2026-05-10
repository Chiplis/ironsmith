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
