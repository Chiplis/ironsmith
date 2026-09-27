import { publicDeckManifest } from "./multiplayer-audit.js";

export function buildZiffleRuntimeManifest(manifest, ceremony) {
  const baseManifest = publicDeckManifest(manifest);
  const deckCount = Number(baseManifest?.deckCount || 0);
  const sideboardCount = Number(baseManifest?.sideboardCount || 0);
  if (!Number.isSafeInteger(deckCount) || deckCount < 0
    || Number(ceremony?.deckCount) !== deckCount) {
    throw new Error("Ziffle ceremony deck count does not match the committed main deck");
  }
  if (!Number.isSafeInteger(sideboardCount) || sideboardCount < 0) {
    throw new Error("Invalid committed sideboard count");
  }
  const sideboardSlots = (baseManifest?.slotCommitments || [])
    .filter(({ slot }) => Number(slot) >= deckCount)
    .sort((left, right) => Number(left.slot) - Number(right.slot));
  if (sideboardSlots.length !== sideboardCount
    || sideboardSlots.some(({ slot, commitment }, index) => (
      Number(slot) !== deckCount + index || !String(commitment || "")
    ))) {
    throw new Error("Hidden manifest must commit every sideboard slot exactly once");
  }
  const deckHash = String(ceremony?.deckHash || "");
  return {
    ...baseManifest,
    deckCount,
    commitmentRoot: `ziffle:${deckHash}`,
    slotCommitments: [
      ...Array.from({ length: deckCount }, (_, position) => ({
        slot: position,
        commitment: `ziffle:${deckHash}:${position}`,
      })),
      // Only the main deck participates in the shuffle ceremony.
      ...sideboardSlots,
    ],
  };
}
