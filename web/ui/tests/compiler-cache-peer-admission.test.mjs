import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { CURRENT_AUDIT_PROTOCOL_VERSION } from "../src/lib/multiplayer-audit.js";

// Source-authored, UNRUN. Invoke the real callbacks directly, without relying
// on transport admission, React, signed genesis, or an installed engine.
function productionCallback(file, name, endMarker, context) {
  const source = readFileSync(new URL(`../src/hooks/peer-lobby/${file}.js`, import.meta.url), "utf8");
  const start = source.indexOf(`const ${name} = useCallback(`);
  const body = source.indexOf("async (", start);
  const end = source.indexOf(endMarker, body);
  assert.ok(start >= 0 && body > start && end > body, `${name} production callback exists`);
  return new Function(...Object.keys(context), `return (${source.slice(body, end)}\n});`)(...Object.values(context));
}

const PROTOCOL_VERSION = CURRENT_AUDIT_PROTOCOL_VERSION;
const invalidVersions = [25, 26, 27, 28, 29, 30, undefined, null, "26", String(PROTOCOL_VERSION), PROTOCOL_VERSION + 1];
const modes = ["trusted", "verified"];

function harness(kind) {
  const calls = [];
  const stop = label => { calls.push(label); throw new Error(`reached ${label}`); };
  const guardedRef = label => ({ get current() { return stop(label); } });
  const awaiting = { current: false };
  const resync = { current: false };
  const context = {
    PROTOCOL_VERSION,
    assertRuntimeVersion: () => stop("runtime hint"),
    gameRef: guardedRef("engine access"),
    servicesRef: guardedRef("reset access"),
    multiplayerRef: guardedRef("session access"),
    awaitingStateResyncRef: awaiting,
    resyncInProgressRef: resync,
    updateMultiplayer: () => {},
    scheduleResyncRetry: () => false,
    setStatus: () => {},
  };
  const invoke = kind === "start"
    ? productionCallback("validation", "applyMatchStart", "\n\t    },\n    [", context)
    : productionCallback("messaging", "applyStateResync", "\n    },\n    [", context);
  return { calls, invoke, awaiting, resync };
}

test("direct match starts reject historical, missing, and nonnumeric versions before runtime or engine access", async () => {
  assert.equal(PROTOCOL_VERSION, 31);
  for (const securityMode of modes) {
    for (const options of [{}, { skipGenesisVerification: true }, { verifiedResyncReplay: true },
      { skipGenesisVerification: true, verifiedResyncReplay: true }]) {
      for (const protocolVersion of invalidVersions) {
        const h = harness("start");
        await assert.rejects(h.invoke({ protocolVersion, securityMode }, options), /Match start requires audit protocol 31/);
        assert.deepEqual(h.calls, []);
      }
    }
  }
  for (const payload of [undefined, null, {}]) {
    const h = harness("start");
    await assert.rejects(h.invoke(payload), /Match start requires audit protocol 31/);
    assert.deepEqual(h.calls, []);
  }
});

test("direct match start rechecks a mutated accepted payload even when genesis checks are skipped", async () => {
  const h = harness("start");
  const payload = { protocolVersion: PROTOCOL_VERSION, securityMode: "trusted" };
  await assert.rejects(h.invoke(payload, { skipGenesisVerification: true }), /reached runtime hint/);
  assert.deepEqual(h.calls, ["runtime hint"]);
  for (const version of invalidVersions) {
    h.calls.length = 0;
    payload.protocolVersion = version;
    await assert.rejects(h.invoke(payload, { skipGenesisVerification: true }), /Match start requires audit protocol 31/);
    assert.deepEqual(h.calls, []);
  }
});

test("Trusted and Verified nested resync carriers fail before flags, resets, or engine reads", async () => {
  for (const securityMode of modes) {
    const invalid = [undefined, null, {}, { protocolVersion: PROTOCOL_VERSION },
      { protocolVersion: PROTOCOL_VERSION, match: null }];
    for (const version of invalidVersions) {
      invalid.push({ protocolVersion: version, match: { protocolVersion: PROTOCOL_VERSION, securityMode } });
      invalid.push({ protocolVersion: PROTOCOL_VERSION, match: { protocolVersion: version, securityMode } });
      invalid.push({ protocolVersion: version, match: { protocolVersion: version, securityMode } });
    }
    for (const message of invalid) {
      const h = harness("resync");
      await assert.rejects(h.invoke(message), /State resync requires audit protocol 31 on both message and match/);
      assert.deepEqual(h.calls, []);
      assert.equal(h.awaiting.current, false);
      assert.equal(h.resync.current, false);
    }
  }
});

test("current nested resync proceeds past admission for Trusted and Verified recovery", async () => {
  for (const securityMode of modes) {
    const h = harness("resync");
    await assert.rejects(h.invoke({ protocolVersion: PROTOCOL_VERSION,
      match: { protocolVersion: PROTOCOL_VERSION, securityMode } }), /reached session access/);
    assert.deepEqual(h.calls, ["session access"]);
  }
});

test("resync rechecks both protocol owners after a carrier is mutated between calls", async () => {
  for (const owner of ["outer", "inner"]) {
    const h = harness("resync");
    const message = { protocolVersion: PROTOCOL_VERSION, match: { protocolVersion: PROTOCOL_VERSION, securityMode: "trusted" } };
    for (const version of invalidVersions) {
      (owner === "outer" ? message : message.match).protocolVersion = version;
      await assert.rejects(h.invoke(message), /State resync requires audit protocol 31/);
      assert.deepEqual(h.calls, []);
      assert.equal(h.awaiting.current, false);
      assert.equal(h.resync.current, false);
    }
  }
});
