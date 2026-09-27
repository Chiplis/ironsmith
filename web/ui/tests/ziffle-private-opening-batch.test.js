import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const source = readFileSync(new URL("../src/hooks/peer-lobby/crypto-resync.js", import.meta.url), "utf8");
const start = source.indexOf("  async function batchedOwnerPrivateZiffleOpeningsForLocalViewer(");
const end = source.indexOf("  async function privateOpeningsForLocalViewer(", start);
assert.ok(start >= 0 && end > start);
const batchSource = source.slice(start, end);
const requirement = (position, fields = {}) => ({ type: "private_open", owner: 0, viewer: 0,
  objectId: 100 + position, slot: position, commitment: `ziffle:deck:${position}`, ...fields });

function harness({ prefetch, origins = false } = {}) {
  const calls = [];
  const ceremony = { owner: 0, context: "action-shuffle", deckHash: "deck" };
  const context = {
    gameRef: { current: { ziffleRevealCards() { throw new Error("Unexpected direct batch reveal"); } } },
    resolveLocalCryptoPlayerIndex: () => 0,
    privateDeckManifestForOwner: () => ({ owner: 0 }),
    ziffleDeckHashFromCommitment: commitment => commitment.startsWith("ziffle:") ? commitment.split(":")[1] : null,
    zifflePositionFromCommitment: commitment => Number(commitment.split(":")[2]),
    zifflePositionForObjectId: () => ({ ziffleContext: ceremony.context }),
    ziffleCeremonyForOwner: (_owner, { commitment } = {}) => commitment?.startsWith("ziffle:initial:")
      ? { owner: 0, context: "initial", deckHash: "initial" } : ceremony,
    currentZiffleOriginForOpening: async opening => origins ? {
      originPosition: opening.position + 10,
      originPositionCommitment: `ziffle:initial:${opening.position + 10}`,
    } : null,
    ziffleCeremonyHasObjectOrder: () => true,
    collectZiffleRevealTokensBatch: async (actualCeremony, positions, options) => {
      calls.push({ kind: "batch", ceremony: actualCeremony, positions, options });
      if (prefetch) await prefetch();
      return [];
    },
    resolveCommittedSlotForZifflePosition: async ({ position, options }) => {
      calls.push({ kind: "resolve", position, options });
      return { resolvedRevealSlot: { slot: position } };
    },
    buildOpeningFromResolvedCommittedSlot: async ({ resolvedRevealSlot, position, positionCommitment, fallbackObjectId }) => ({
      originalSlot: resolvedRevealSlot.slot,
      openingWithPosition: { owner: 0, slot: resolvedRevealSlot.slot, objectId: fallbackObjectId, position,
        positionCommitment, card: `Card ${position}` },
    }),
    sanitizeObjectBoundOpening: async opening => opening,
    ensureZiffleOpeningProof: async (opening, options) => {
      calls.push({ kind: "proof", position: opening.position, options });
      return opening;
    },
    rememberLocalRevealedOpening: () => {},
    rememberZiffleOpeningPosition: () => {},
  };
  const batch = new Function(...Object.keys(context), `${batchSource}\nreturn batchedOwnerPrivateZiffleOpeningsForLocalViewer;`)(...Object.values(context));
  return { batch, calls, ceremony };
}

test("prefetches one authorized batch before resolving object-ordered private cards", async () => {
  const { batch, calls, ceremony } = harness();
  const requirements = [requirement(5), requirement(2), requirement(5)];
  const options = { command: { type: "select_objects", object_ids: [105] }, seq: 12, actorIndex: 0,
    actionIntent: { signature: "signed-intent" }, cryptoMaterialRequestId: "request", updateState: false };
  const result = await batch(requirements, options);
  assert.deepEqual(calls[0], { kind: "batch", ceremony, positions: [5, 2], options: { ...options, requirements } });
  assert.equal(calls.filter(call => call.kind === "batch").length, 1);
  assert.deepEqual(calls.filter(call => call.kind === "resolve").map(call => call.position), [5, 2, 5]);
  assert.deepEqual(calls.filter(call => call.kind === "proof").map(call => call.position), [5, 2, 5]);
  assert.ok(calls.filter(call => call.kind !== "batch").every(call => call.options === options));
  assert.equal(result.openings.length, 2);
  assert.equal(result.handledRequirements.size, 3);
});

test("does not request foreign, public, or invalid private positions", async () => {
  const { batch, calls } = harness();
  const valid = requirement(3);
  const invalid = [requirement(4, { owner: 1 }), requirement(5, { viewer: 1 }),
    requirement(6, { type: "public_open" }), requirement(7, { commitment: "not-ziffle" }), requirement(-1)];
  const requirements = [valid, ...invalid];
  const result = await batch(requirements, { seq: 8 });
  assert.deepEqual(calls[0].positions, [3]);
  assert.equal(calls[0].options.requirements, requirements, "carry complete authorization without widening requested positions");
  assert.deepEqual([...result.handledRequirements], [valid]);
  const noEligible = harness();
  assert.equal((await noEligible.batch(invalid)).openings.length, 0);
  assert.deepEqual(noEligible.calls, []);
});

test("prefetch failure aborts before any per-card opening or proof", async () => {
  const { batch, calls } = harness({ prefetch: async () => { throw new Error("Unauthorized reveal"); } });
  await assert.rejects(batch([requirement(2), requirement(3)]), /Unauthorized reveal/);
  assert.deepEqual(calls.map(call => call.kind), ["batch"]);
});

test("reshuffled private cards prefetch their exact immutable origins in one initial-ceremony batch", async () => {
  const { batch, calls } = harness({ origins: true });
  const requirements = [requirement(5), requirement(2), requirement(5)];
  await batch(requirements, { seq: 9 });
  const batches = calls.filter(call => call.kind === "batch");
  assert.equal(batches.length, 1);
  assert.equal(batches[0].ceremony.context, "initial");
  assert.deepEqual(batches[0].positions, [15, 12]);
  assert.equal(batches[0].options.requirements, requirements);
  assert.deepEqual(calls.filter(call => call.kind === "resolve").map(call => call.position), [5, 2, 5]);
});
