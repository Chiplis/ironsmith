import test from "node:test";
import assert from "node:assert/strict";
import { createWasmInteractionGate } from "../src/lib/wasmInteractionGate.js";

test("queued planning and payment survive an in-flight action and its cooldown", async () => {
  let now = 0;
  const gate = createWasmInteractionGate({ now: () => now });
  let release;
  const first = gate.run(() => new Promise(resolve => { release = resolve; }));
  const actions = [];
  const planning = gate.runWhenReady(() => { actions.push("replan"); return "planned"; });
  assert.deepEqual(actions, []);
  release();
  await first;
  now = 100;
  assert.equal(await planning, "planned");
  const payment = gate.runWhenReady(() => actions.push("confirm"));
  assert.deepEqual(actions, ["replan"]);
  now = 200;
  await payment;
  assert.deepEqual(actions, ["replan", "confirm"]);
});

test("queued refinement is discarded when its payment is no longer current", async () => {
  const gate = createWasmInteractionGate();
  let release;
  const first = gate.run(() => new Promise(resolve => { release = resolve; }));
  let current = true;
  let ran = false;
  const queued = gate.runWhenReady(() => { ran = true; }, () => current);
  current = false;
  release();
  await first;
  await queued;
  assert.equal(ran, false);
});

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

test("phase automation resumes during click cooldown without extending it", async () => {
  let now = 0;
  const gate = createWasmInteractionGate({ now: () => now });
  await gate.run(() => "pass");
  now = 10;
  assert.equal(gate.isBlocked(), true);
  assert.equal(await gate.runAutomatic(() => "next phase"), "next phase");
  assert.equal(await gate.run(() => "duplicate click"), undefined);
  now = 100;
  assert.equal(gate.isBlocked(), false);
  await gate.runAutomatic(() => "analysis complete");
  assert.equal(gate.isBlocked(), false, "automation does not create another click cooldown");
});

test("phase automation cannot overlap a user action or another continuation", async () => {
  const gate = createWasmInteractionGate();
  let release;
  const userAction = gate.run(() => new Promise(resolve => { release = resolve; }));
  assert.equal(gate.isInFlight(), true);
  assert.equal(await gate.runAutomatic(() => assert.fail("overlapping automation")), undefined);
  release();
  await userAction;
  const continuation = gate.runAutomatic(() => new Promise(resolve => { release = resolve; }));
  assert.equal(await gate.runAutomatic(() => assert.fail("overlapping continuation")), undefined);
  assert.equal(await gate.run(() => assert.fail("overlapping click")), undefined);
  release();
  await continuation;
  assert.equal(gate.isInFlight(), false);
  await assert.rejects(gate.runAutomatic(() => { throw new Error("automation failed"); }), /automation failed/);
  assert.equal(gate.isInFlight(), false);
});
