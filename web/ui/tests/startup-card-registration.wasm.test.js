import test from "node:test";
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import init, { WasmGame } from "../../wasm_demo/pkg/ironsmith.js";

test("browser facade loads startup cards when baked artifacts are stale", async () => {
  const modules = await Promise.all(["engine", "compiler", "verifier"].map(async name => [
    name, await readFile(new URL(`../../wasm_demo/pkg/${name}_bg.wasm`, import.meta.url)),
  ]));
  await init(Object.fromEntries(modules));
  const game = new WasmGame();
  try {
    game.resetEmpty(["Alice", "Bob"], 20);
    for (const route of ["omniscience", "yawgmoth-thran-physician"]) {
      const source = JSON.parse(await readFile(new URL(`../public/cards/${route}.json`, import.meta.url)));
      source.artifacts[0].payloadChecksum = "outdated-artifact";
      const summary = JSON.parse(game.registerExternalCardSourcesJson(JSON.stringify(source)));
      assert.deepEqual(summary.failed, []);
      assert.equal(summary.loaded, 1);
      for (const player of [0, 1]) {
        const id = game.addCardToZone(player, source.canonicalName, "battlefield", true);
        const details = game.objectDetails(id);
        assert.equal(details.name, source.canonicalName);
        assert.equal(details.zone.toLowerCase(), "battlefield");
      }
    }
  } finally { game.free(); }
});
