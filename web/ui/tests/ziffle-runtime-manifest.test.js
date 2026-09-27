import test from "node:test";
import assert from "node:assert/strict";
import { webcrypto } from "node:crypto";
import { buildPrivateDeckManifest } from "../src/lib/multiplayer-audit.js";
import { buildZiffleRuntimeManifest } from "../src/lib/ziffle-runtime-manifest.js";

async function manifestFor(sideboard = ["Mountain", "Plains"]) {
  return buildPrivateDeckManifest({
    matchId: "sideboard-slots", owner: 0, deck: ["Island", "Forest"], sideboard,
    commanders: ["Some Commander"], saltForSlot: slot => `salt-${slot}`,
  }, webcrypto);
}

test("Ziffle replaces only library commitments and retains sideboard commitments", async () => {
  const manifest = await manifestFor();
  const original = structuredClone(manifest);
  const runtime = buildZiffleRuntimeManifest(manifest, { deckCount: 2, deckHash: "shuffled" });
  assert.deepEqual(runtime.slotCommitments, [
    { slot: 0, commitment: "ziffle:shuffled:0" },
    { slot: 1, commitment: "ziffle:shuffled:1" },
    ...manifest.slotCommitments.slice(2),
  ]);
  assert.equal(runtime.deckCount, 2);
  assert.equal(runtime.sideboardCount, 2);
  assert.equal(runtime.commanderCount, 1);
  assert.equal(runtime.slotSecrets, undefined);
  assert.equal(runtime.decklistSalt, undefined);
  assert.equal(runtime.decklistCommitment, manifest.decklistCommitment);
  assert.equal(runtime.commitmentRoot, "ziffle:shuffled");
  assert.deepEqual(manifest, original);
});

test("Ziffle keeps the existing main-deck-only shape for decks without sideboards", async () => {
  const runtime = buildZiffleRuntimeManifest(await manifestFor([]), { deckCount: 2, deckHash: "main-only" });
  assert.deepEqual(runtime.slotCommitments, [
    { slot: 0, commitment: "ziffle:main-only:0" },
    { slot: 1, commitment: "ziffle:main-only:1" },
  ]);
});

test("Ziffle rejects missing, duplicate, and out-of-range sideboard commitments", async () => {
  const manifest = await manifestFor();
  for (const slots of [
    manifest.slotCommitments.slice(0, 3),
    [...manifest.slotCommitments.slice(0, 3), manifest.slotCommitments[2]],
    [...manifest.slotCommitments.slice(0, 3), { ...manifest.slotCommitments[3], slot: 4 }],
    [...manifest.slotCommitments.slice(0, 3), { ...manifest.slotCommitments[3], commitment: "" }],
  ]) {
    assert.throws(() => buildZiffleRuntimeManifest({ ...manifest, slotCommitments: slots }, {
      deckCount: 2, deckHash: "shuffled",
    }), /every sideboard slot exactly once/);
  }
});

test("Ziffle cannot shift sideboard slots by changing the ceremony main-deck count", async () => {
  const manifest = await manifestFor();
  assert.throws(() => buildZiffleRuntimeManifest(manifest, { deckCount: 3, deckHash: "shuffled" }), /main deck/);
});
