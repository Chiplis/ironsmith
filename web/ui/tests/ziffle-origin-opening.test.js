import { isPrivateZiffleEpoch, ziffleInputDeckFields } from "../src/lib/ziffle-private-epochs.js";
import { hiddenCardMetadataForObjectFromCheckpoint, hiddenCardMetadataAtPositionFromCheckpoint } from "../src/lib/hidden-card-metadata.js";
import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { buildDeckSlotOpening, buildPrivateDeckManifest, buildZiffleOpeningProof,
  ziffleOriginAnchorFromOpening, ziffleOriginAnchorFromMetadata, assertZiffleOpeningOriginMatchesMetadata,
} from "../src/lib/multiplayer-audit.js";

const source = name => readFileSync(new URL(`../src/hooks/peer-lobby/${name}.js`, import.meta.url), "utf8");
const shared = source("shared");
const connections = source("connections");
const audit = source("audit-material");
function between(text, start, end) {
  const first = text.indexOf(start), last = text.indexOf(end, first + start.length);
  assert.ok(first >= 0 && last > first, `Missing production function ${start}`);
  return text.slice(first, last).replaceAll("export function ", "function ");
}

async function harness({ workerMetadata = false } = {}) {
  const deck = Array(61).fill("Mountain");
  deck[2] = deck[4] = "Barbarian Ring";
  const manifest = await buildPrivateDeckManifest({ matchId: "origin-match", owner: 1, deck });
  const committed = await buildDeckSlotOpening({ manifest, slot: 4 });
  const originCommitment = "ziffle:initial:23", currentCommitment = "ziffle:later:51";
  // The shuffle's object 85 was retired by a draw, then object 211 by play_land.
  const checkpoint = { objects: [{ id: 212, stableId: 85, zone: "Battlefield", hiddenCard: {
    owner: 1, slot: 4, commitment: committed.commitment,
    publicSlot: 51, publicCommitment: currentCommitment,
    originSlot: 23, originCommitment,
  } }] };
  const initial = { owner: 1, context: "origin-match", keyContext: "origin-match", deckHash: "initial", deckCount: 61 };
  const later = { owner: 1, context: "origin-match:action:42:shuffle", keyContext: "origin-match", deckHash: "later", deckCount: 53,
    beforeOrder: Array.from({ length: 53 }, (_, i) => 70 + i), afterOrder: Array.from({ length: 53 }, (_, i) => 122 - i), authenticatedOrder: true };
  const calls = [];
  let checkpointReads = 0;
  const ctx = {
    isPrivateZiffleEpoch, ziffleInputDeckFields,
    hiddenCardMetadataForObjectFromCheckpoint, hiddenCardMetadataAtPositionFromCheckpoint,
    useCallback: fn => fn,
    gameRef: { current: {
      getHiddenCardState: async () => { checkpointReads++; return checkpoint; },
      ...(workerMetadata ? {
        getHiddenCardMetadata: async id => hiddenCardMetadataForObjectFromCheckpoint(checkpoint, id),
        getHiddenCardMetadataAtPosition: async (...args) => hiddenCardMetadataAtPositionFromCheckpoint(checkpoint, ...args),
      } : {}),
      ziffleRevealCard: async input => { calls.push(input); return { originalSlot: input.context === initial.context ? 4 : 2 }; },
    } },
    currentAuditMatchId: () => "origin-match",
    normalizeShuffleOrder: value => Array.isArray(value) ? value.map(Number) : [],
    cloneMultiplayerPayload: structuredClone,
    ziffleCeremonyForOwner: (_owner, { commitment } = {}) => commitment?.includes(":initial:") ? initial : later,
    hydrateZiffleCeremonyForLookup: value => value,
    ziffleCeremonyForOpeningProof: (_proof, fallback) => fallback,
    ziffleKeyContextForCeremony: value => value.keyContext || value.context,
    matchStartPayloadRef: { current: {} },
    collectZiffleRevealTokens: async (_ceremony, position) => [{ player: 0, cardPosition: position, tokenHex: "token", proofHex: "proof", publicKeyHex: "key" }],
    privateDeckManifestForOwner: () => manifest,
    currentHiddenCardMetadataForObject: async () => null,
    localRevealedOpeningForZiffleReveal: () => null,
    localRevealedOpeningForRequirement: () => null,
    auditEncryptionPublicKeyForPlayer: () => "",
    openingMatchesRequirement: () => true,
    buildDeckSlotOpening, buildZiffleOpeningProof,
    ziffleOriginAnchorFromOpening, ziffleOriginAnchorFromMetadata, assertZiffleOpeningOriginMatchesMetadata,
  };
  const functions = [
    between(shared, "export function ziffleRuntimeCommitment(", "export function ziffleContextForCommitment("),
    between(audit, "  const currentZiffleOriginForOpening = useCallback(", "\t\t  const sanitizeObjectBoundOpening = useCallback("),
    between(audit, "\t  const resolveCommittedZiffleRevealSlot = useCallback(", "  async function buildOpeningFromResolvedCommittedSlot("),
    between(audit, "  const verifyAuditSatisfiesCryptoRequirements = useCallback(", "  function previewAuditOpeningInInspector("),
    between(connections, "  function ziffleTokensForPosition(", "  function ziffleRevealTokenCacheKey("),
    between(connections, "\t  function openingNeedsZiffleProof(", "  const localZiffleDiagnostics = useCallback("),
  ].join("\n");
  const helpers = new Function(...Object.keys(ctx), `${functions}\nreturn {ensureZiffleOpeningProof, verifyZiffleOpeningProofForOpening, verifyZiffleOpeningCryptographicProof, currentZiffleOriginForOpening, resolveCommittedSlotForZifflePosition, verifyAuditSatisfiesCryptoRequirements};`)(...Object.values(ctx));
  return { ...helpers, checkpoint, manifest, calls, later, checkpointReads: () => checkpointReads, opening: { ...committed, objectId: 211, position: 51, positionCommitment: currentCommitment, ziffleContext: later.context }, originCommitment };
}

test("worker metadata projection preserves fresh origin binding and ambiguous-position rejection", async () => {
  const h = await harness({ workerMetadata: true });
  const opening = await h.ensureZiffleOpeningProof(h.opening);
  await h.verifyZiffleOpeningProofForOpening(opening);
  assert.equal(h.checkpointReads(), 0, "Full checkpoints do not cross the worker boundary");
  h.checkpoint.objects.push({ ...h.checkpoint.objects[0], id: 213 });
  await assert.rejects(h.ensureZiffleOpeningProof(opening), /ambiguous immutable origin/);
  h.checkpoint.objects.pop();
  h.checkpoint.objects[0].hiddenCard.originSlot = 24;
  h.checkpoint.objects[0].hiddenCard.originCommitment = "ziffle:initial:24";
  await assert.rejects(h.ensureZiffleOpeningProof(opening), /trusted identity/);
});

test("proof reuse binds once to current metadata and still verifies the cryptographic slot", async () => {
  const h = await harness();
  const opening = await h.ensureZiffleOpeningProof(h.opening);
  const reads = h.checkpointReads(), calls = h.calls.length;
  await h.ensureZiffleOpeningProof(opening);
  assert.equal(h.checkpointReads() - reads, 1);
  assert.ok(h.calls.length > calls, "Reusing a proof still verifies its ciphertext slot");
  const duplicate = await buildDeckSlotOpening({ manifest: h.manifest, slot: 2 });
  await assert.rejects(h.ensureZiffleOpeningProof({ ...opening, ...duplicate }),
    /Ziffle card opening proof slot mismatch/);
});

test("a shuffled card's original proof survives both draw and public zone-change IDs", async () => {
  const h = await harness();
  const opening = await h.ensureZiffleOpeningProof(h.opening, { forceZiffleOpeningProof: true });
  assert.equal(opening.objectId, 212);
  assert.equal(opening.position, 51);
  assert.equal(opening.positionCommitment, "ziffle:later:51");
  assert.equal(opening.originPosition, 23);
  assert.equal(opening.originPositionCommitment, h.originCommitment);
  assert.equal(opening.ziffleReveal.position, 23);
  assert.equal(opening.ziffleReveal.positionCommitment, h.originCommitment);
  assert.equal(opening.ziffleReveal.originalSlot, 4);
  await h.verifyZiffleOpeningProofForOpening(opening);
  assert.ok(h.calls.every(call => call.context === "origin-match" && call.cardPosition === 23));
});

test("a salted opening for a different copy of the same card cannot replace the origin slot", async () => {
  const h = await harness();
  const duplicate = await buildDeckSlotOpening({ manifest: h.manifest, slot: 2 });
  await assert.rejects(h.ensureZiffleOpeningProof({ ...h.opening, ...duplicate }, { forceZiffleOpeningProof: true }), /different committed slot/);
  const valid = await h.ensureZiffleOpeningProof(h.opening);
  const forged = { ...valid, ...duplicate, ziffleReveal: { ...valid.ziffleReveal, originalSlot: 2 } };
  await assert.rejects(h.verifyZiffleOpeningProofForOpening(forged), /different shuffle slot|different committed slot/);
});

test("the origin anchor is checked against trusted current position metadata before proof reuse", async () => {
  const h = await harness();
  const valid = await h.ensureZiffleOpeningProof(h.opening);
  await assert.rejects(h.verifyZiffleOpeningProofForOpening({ ...valid, originPosition: 24, originPositionCommitment: "ziffle:initial:24" }), /trusted identity/);
  await assert.rejects(h.verifyZiffleOpeningProofForOpening({ ...valid, position: 50, positionCommitment: "ziffle:later:50" }), /trusted identity/);
  await assert.rejects(h.verifyZiffleOpeningProofForOpening({ ...valid, owner: 0 }), /trusted identity/);
  const withoutOrigin = { ...valid };
  delete withoutOrigin.originPosition;
  delete withoutOrigin.originPositionCommitment;
  await assert.rejects(h.verifyZiffleOpeningProofForOpening(withoutOrigin), /missing its immutable origin/);
  h.checkpoint.objects[0].hiddenCard.originSlot = 24;
  h.checkpoint.objects[0].hiddenCard.originCommitment = "ziffle:initial:24";
  await assert.rejects(h.ensureZiffleOpeningProof(valid, { skipFreshZiffleOpeningProofVerification: true }), /trusted identity/);
});

test("private identity resolution uses the immutable origin before independent later-shuffle indices", async () => {
  const h = await harness();
  const result = await h.resolveCommittedSlotForZifflePosition({ owner: 1, ceremony: h.later,
    position: 51, objectId: 211, card: "Barbarian Ring", manifest: h.manifest });
  assert.equal(result.resolvedRevealSlot.slot, 4);
  assert.equal(result.resolvedRevealSlot.objectId, 212);
  assert.equal(result.resolvedRevealSlot.originPosition, 23);
  assert.equal(result.resolvedRevealSlot.source, "immutable_origin");
  assert.equal(result.shuffleOriginalSlot, null);
  assert.deepEqual(h.calls.map(call => [call.context, call.cardPosition]), [["origin-match", 23]]);
});

test("envelope cryptography can precede object creation but hydration verification still requires state binding", async () => {
  const h = await harness();
  const opening = await h.ensureZiffleOpeningProof(h.opening);
  h.checkpoint.objects.length = 0;
  await h.verifyZiffleOpeningCryptographicProof(opening);
  await assert.rejects(h.verifyZiffleOpeningProofForOpening(opening), /trusted identity/);
});

test("trusted public-open requirements reject omitted origins and relocated current positions", async () => {
  const h = await harness();
  const opening = await h.ensureZiffleOpeningProof(h.opening);
  const requirements = [{ type: "public_open", ...h.checkpoint.objects[0].hiddenCard }];
  const verify = candidate => h.verifyAuditSatisfiesCryptoRequirements({ requirements, audit: { openings: [candidate] } });
  await verify(opening);
  const downgraded = { ...opening };
  delete downgraded.originPosition;
  delete downgraded.originPositionCommitment;
  delete downgraded.position;
  delete downgraded.positionCommitment;
  await assert.rejects(verify(downgraded), /trusted identity/);
  await assert.rejects(verify({ ...opening, position: 50, positionCommitment: "ziffle:later:50" }), /required current public position/);
});
