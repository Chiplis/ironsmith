import test from "node:test";
import assert from "node:assert/strict";
import { createWasmInteractionGate } from "../src/lib/wasmInteractionGate.js";

test("blocks overlapping interactions and enforces a 100ms cooldown", async () => {
  let now = 0;
  const gate = createWasmInteractionGate({
    debounceMs: 100,
    now: () => now,
  });

  let resolveFirst;
  const first = gate.run(() => new Promise((resolve) => {
    resolveFirst = resolve;
  }));

  assert.equal(gate.isBlocked(), true);

  let secondRan = false;
  const second = await gate.run(async () => {
    secondRan = true;
    return "second";
  });

  assert.equal(second, undefined);
  assert.equal(secondRan, false);

  resolveFirst("first");
  assert.equal(await first, "first");
  assert.equal(gate.isBlocked(), true);

  now = 99;
  assert.equal(gate.isBlocked(), true);
  assert.equal(await gate.run(async () => "cooldown"), undefined);

  now = 100;
  assert.equal(gate.isBlocked(), false);
  assert.equal(await gate.run(async () => "third"), "third");
});

test("releases the gate after errors", async () => {
  let now = 0;
  const gate = createWasmInteractionGate({
    debounceMs: 100,
    now: () => now,
  });

  await assert.rejects(
    gate.run(async () => {
      throw new Error("boom");
    }),
    /boom/
  );

  assert.equal(gate.isBlocked(), true);
  now = 100;
  assert.equal(gate.isBlocked(), false);
  assert.equal(await gate.run(async () => "ok"), "ok");
});
