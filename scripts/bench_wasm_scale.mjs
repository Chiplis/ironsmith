#!/usr/bin/env node
import { performance } from "node:perf_hooks";
import { mkdir, writeFile } from "node:fs/promises";
import { initWasmGame, createNewGameAndPlayers } from "./wasm-test-harness.mjs";

async function main() {
  const { game } = await initWasmGame();
  createNewGameAndPlayers(game, {
    playerNames: ["Alice", "Bob"],
    startingLife: 20,
    seed: 0x5eed,
    format: "normal",
    openingHandSize: 0,
    decks: [[], []],
  });

  const n = Number(process.env.WASM_SCALE_OBJECTS || 100);
  for (let i = 0; i < n; i += 1) {
    game.addCardToZone(i % 2, i % 4 === 0 ? "Grizzly Bears" : "Ornithopter", "battlefield", true);
  }

  const started = performance.now();
  game.uiState();
  const elapsedMs = performance.now() - started;
  const report = {
    objects: n,
    uiStateMs: elapsedMs,
    lastSnapshotPerf: game.lastSnapshotPerf(),
    lastWorkCounters: game.lastWorkCounters(),
  };

  await mkdir("reports/bench", { recursive: true });
  const path = `reports/bench/wasm-scale-${Date.now()}.json`;
  await writeFile(path, `${JSON.stringify(report, null, 2)}\n`);
  console.log(`wrote ${path}`);
}

main().catch((error) => {
  console.error(error?.stack || error);
  process.exit(1);
});
