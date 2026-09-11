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

test("browser facade preserves compiler failures for loading and diagnostics", async () => {
  const modules = await Promise.all(["engine", "compiler", "verifier"].map(async name => [
    name, await readFile(new URL(`../../wasm_demo/pkg/${name}_bg.wasm`, import.meta.url)),
  ]));
  await init(Object.fromEntries(modules));
  const game = new WasmGame();
  try {
    game.resetEmpty(["Alice", "Bob"], 20);
    const source = {
      canonicalName: "Unparseable Card Probe",
      group: {
        kind: "single",
        name: "Unparseable Card Probe",
        block: "Type: Creature — Demon\nPower/Toughness: 4/5\nThis is deliberately invalid oracle text.",
      },
    };
    const summary = JSON.parse(game.registerExternalCardSourcesJson(JSON.stringify(source)));
    assert.equal(summary.loaded, 0);
    assert.equal(summary.failed.length, 1);
    const error = summary.failed[0].error;
    assert.ok(error.length > 0);
    assert.doesNotMatch(error, /generated registry is not embedded/);
    assert.throws(
      () => game.addCardToZone(0, "unparseable card probe", "hand", true),
      (thrown) => String(thrown) === error,
    );
    const diagnostics = game.cardLoadDiagnostics("unparseable card probe");
    assert.equal(diagnostics.error, error);
    assert.equal(diagnostics.parseError, error);
    assert.equal(diagnostics.canonicalName, source.canonicalName);
    assert.match(diagnostics.oracleText, /deliberately invalid oracle text/);

    // A successful retry must clear the earlier failure.
    source.group.block = "Type: Creature — Demon\nPower/Toughness: 4/5\nFlying.";
    const retry = game.registerExternalCardSources(source);
    assert.deepEqual(retry.failed, []);
    assert.equal(retry.loaded, 1);
    assert.ok(game.addCardToZone(0, source.canonicalName, "hand", true));
  } finally { game.free(); }
});

test("an unknown card name reports the name, not the engine's registry internals", async () => {
  const modules = await Promise.all(["engine", "compiler", "verifier"].map(async name => [
    name, await readFile(new URL(`../../wasm_demo/pkg/${name}_bg.wasm`, import.meta.url)),
  ]));
  await init(Object.fromEntries(modules));
  const game = new WasmGame();
  try {
    game.resetEmpty(["Alice", "Bob"], 20);
    // The lean engine embeds no generated registry, so every name it has no
    // artifact for comes back through one sentinel. That sentinel is an
    // implementation detail and must never reach the add-card notice.
    const name = "Nonexistent Card Probe";
    assert.throws(
      () => game.addCardToZone(0, name, "hand", true),
      (thrown) => String(thrown.message ?? thrown) === `unknown card name: ${name}`,
    );
    const diagnostics = game.cardLoadDiagnostics(name);
    assert.equal(diagnostics.error, `unknown card name: ${name}`);
    assert.equal(diagnostics.parseError, `unknown card name: ${name}`);
  } finally { game.free(); }
});
