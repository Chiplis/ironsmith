import { isPrivateZiffleEpoch, ziffleInputDeckFields } from "../src/lib/ziffle-private-epochs.js";
import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import {
  buildDeckSlotOpening,
  buildPrivateDeckManifest,
  buildZiffleOpeningProof,
} from "../src/lib/multiplayer-audit.js";

const auditSource = readFileSync(new URL("../src/hooks/peer-lobby/audit-material.js", import.meta.url), "utf8");
const connectionsSource = readFileSync(new URL("../src/hooks/peer-lobby/connections.js", import.meta.url), "utf8");
const sharedSource = readFileSync(new URL("../src/hooks/peer-lobby/shared.js", import.meta.url), "utf8");
function between(source, start, end) {
  const first = source.indexOf(start);
  const last = source.indexOf(end, first + start.length);
  assert.ok(first >= 0 && last > first, `Missing production function: ${start}`);
  return source.slice(first, last);
}

async function harness({ metadata = true, ordered = true } = {}) {
  const deck = Array(61).fill("Mountain");
  deck[6] = "Bloodstained Mire";
  deck[33] = "Lightning Bolt";
  const manifest = await buildPrivateDeckManifest({ matchId: "reshuffle-regression", owner: 1, deck });
  // A later library shuffle indexes its 53 remaining objects, not the 61 original slots.
  const beforeOrder = Array.from({ length: 53 }, (_, index) => 300 + index);
  beforeOrder[33] = 157;
  const afterOrder = [...beforeOrder];
  [afterOrder[33], afterOrder[51]] = [afterOrder[51], afterOrder[33]];
  const ceremony = { owner: 1, deckCount: ordered ? 53 : 61, deckHash: "reshuffled",
    context: "match:action:42:shuffle", keyContext: "match", authenticatedOrder: true,
    ...(ordered ? { beforeOrder, afterOrder } : {}) };
  const positionCommitment = "ziffle:reshuffled:51";
  const committedOpening = await buildDeckSlotOpening({ manifest, slot: 6 });
  const opening = { ...committedOpening, objectId: 192, shuffleObjectId: 157,
    position: 51, positionCommitment, ziffleContext: ceremony.context };
  const metadataById = new Map(metadata ? [[192, { owner: 1, slot: 6,
    commitment: committedOpening.commitment, publicSlot: 51, publicCommitment: positionCommitment }]] : []);
  const nestedCeremonies = new Map();
  const calls = [];
  const context = {
    isPrivateZiffleEpoch, ziffleInputDeckFields,
    useCallback: fn => fn,
    normalizeShuffleOrder: order => Array.isArray(order) ? order.map(Number) : [],
    cloneMultiplayerPayload: structuredClone,
    ziffleRuntimeCommitment: (hash, position) => `ziffle:${hash}:${position}`,
    ziffleDeckHashFromCommitment: value => String(value || "").startsWith("ziffle:")
      ? String(value).slice(7, String(value).lastIndexOf(":")) : "",
    zifflePositionFromCommitment: value => String(value || "").startsWith("ziffle:")
      ? Number(String(value).slice(String(value).lastIndexOf(":") + 1)) : null,
    ziffleKeyContextForCeremony: value => value.keyContext || value.context,
    ziffleContextFromCeremony: value => String(value?.context || ""),
    ziffleContextFromOpening: value => String(value?.ziffleContext || ""),
    privateDeckManifestForOwner: () => manifest,
    currentHiddenCardMetadataForObject: async id => metadataById.get(Number(id)) || null,
    currentZiffleOriginForOpening: async () => null,
    ziffleOriginAnchorFromOpening: () => null,
    localRevealedOpeningForZiffleReveal: () => null,
    hiddenObjectIdForOpeningFromCheckpoint: () => null,
    hiddenCardMetadataForObjectFromCheckpoint: () => null,
    ziffleCeremonyForOwner: (_owner, { commitment } = {}) => nestedCeremonies.get(commitment) || ceremony,
    openingNeedsZiffleProof: () => true,
    verifyZiffleOpeningProofForOpening: async () => {},
    collectZiffleRevealTokens: async () => [],
    buildDeckSlotOpening,
    buildZiffleOpeningProof,
    gameRef: { current: { ziffleRevealCard: async request => {
      calls.push(request);
      return { originalSlot: ordered ? 33 : 6 };
    } } },
  };
  const functions = [
    between(sharedSource, "export function hiddenMetadataMatchesZifflePosition(", "export function hiddenObjectIdForOpeningFromCheckpoint(").replace("export ", ""),
    between(connectionsSource, "  function ziffleCeremonyHasObjectOrder(", "  function ziffleOpeningProofHasAuthenticatedObjectOrder("),
    between(auditSource, "\t  const resolveCommittedZiffleRevealSlot = useCallback(", "  async function buildOpeningFromResolvedCommittedSlot("),
    between(connectionsSource, "\t  async function ensureZiffleOpeningProof(", "  const localZiffleDiagnostics = useCallback("),
  ].join("\n");
  const helpers = new Function(...Object.keys(context), `${functions}\nreturn { resolveCommittedZiffleRevealSlot, resolveCommittedSlotForZifflePosition, ensureZiffleOpeningProof };`)(...Object.values(context));
  return { ...helpers, opening, manifest, ceremony, metadataById, nestedCeremonies, calls,
    args: { owner: 1, ceremony, shuffleOriginalSlot: ordered ? 33 : 6,
      shuffleOriginalSlotIsVerified: true, position: 51, objectId: 192, manifest } };
}

test("proof construction preserves a revealed card across an object-ordered library reshuffle", async () => {
  const h = await harness();
  const result = await h.ensureZiffleOpeningProof(h.opening);
  assert.equal(result.slot, 6);
  assert.equal(result.card, "Bloodstained Mire");
  assert.equal(result.commitment, h.opening.commitment);
  assert.equal(result.objectId, 192);
  assert.equal(result.position, 51);
  assert.equal(result.ziffleReveal.originalSlot, 6);
  assert.equal(result.ziffleReveal.shuffleOriginalSlot, 33);
});

test("verified reshuffle indices resolve through the object's committed metadata", async () => {
  const h = await harness();
  const result = await h.resolveCommittedZiffleRevealSlot({ ...h.args, card: "Bloodstained Mire" });
  assert.equal(result.slot, 6);
  assert.equal(result.card, "Bloodstained Mire");
  assert.equal(result.source, "hidden_metadata");
});

test("a verified reshuffle index alone cannot select an unrelated original deck slot", async () => {
  const h = await harness({ metadata: false });
  assert.equal(await h.resolveCommittedZiffleRevealSlot(h.args), null);
});

test("initial shuffles still resolve verified indices directly to committed slots", async () => {
  const h = await harness({ metadata: false, ordered: false });
  const result = await h.resolveCommittedZiffleRevealSlot(h.args);
  assert.equal(result.slot, 6);
  assert.equal(result.card, "Bloodstained Mire");
  const stale = { ...h.opening, ...(await buildDeckSlotOpening({ manifest: h.manifest, slot: 33 })) };
  const rebuilt = await h.ensureZiffleOpeningProof(stale);
  assert.equal(rebuilt.slot, 6);
  assert.equal(rebuilt.card, "Bloodstained Mire");
});

test("nested reshuffle metadata cannot reinterpret another shuffle's index as a manifest slot", async () => {
  const h = await harness();
  h.metadataById.set(192, { owner: 1, slot: 4, commitment: "ziffle:nested:4",
    publicSlot: 51, publicCommitment: h.opening.positionCommitment });
  const beforeOrder = Array.from({ length: 53 }, (_, index) => 500 + index);
  beforeOrder[33] = 250;
  const afterOrder = [...beforeOrder];
  [afterOrder[33], afterOrder[4]] = [afterOrder[4], afterOrder[33]];
  h.nestedCeremonies.set("ziffle:nested:4", { ...h.ceremony, deckHash: "nested", context: "prior-shuffle", beforeOrder, afterOrder });
  h.metadataById.set(250, { owner: 1, slot: 6, commitment: h.opening.commitment,
    publicSlot: 51, publicCommitment: h.opening.positionCommitment });
  const result = await h.resolveCommittedZiffleRevealSlot({ ...h.args, shuffleOriginalSlotIsVerified: false });
  assert.equal(result.slot, 6);
  assert.equal(result.card, "Bloodstained Mire");
});
