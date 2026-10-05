// UNVALIDATED: authored during the source-only card failure campaign.
import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { openingPreparationProgress } from "../src/lib/opening-preparation-progress.js";

test("precommit opening progress carries counts but no selected hand identity", () => {
  const localPreview = {
    progressCurrent: 1,
    progressTotal: 3,
    cardName: "Private discarded card",
    zone: "hand",
    openingPreview: { owner: 0, card: "Private discarded card", slot: 8 },
    opening: { card: "Private discarded card", salt: "private-opening-material" },
    detail: "Private discarded card",
  };
  assert.deepEqual(openingPreparationProgress(localPreview), {
    operation: "Preparing public openings",
    progressCurrent: 1,
    progressTotal: 3,
  });
  assert.equal(localPreview.cardName, "Private discarded card", "local preview is unchanged");
});

test("precommit progress cannot copy identity strings through counter fields", () => {
  assert.deepEqual(openingPreparationProgress({
    progressCurrent: "Private discarded card",
    progressTotal: { card: "Private discarded card" },
  }), { operation: "Preparing public openings" });
});

test("local opening previews use the bounded progress payload before publication", () => {
  const source = readFileSync(new URL("../src/hooks/usePeerLobby.js", import.meta.url), "utf8");
  const handler = source.slice(source.indexOf("const previewBuiltLocalOpening ="),
    source.indexOf("const submitPerf =", source.indexOf("const previewBuiltLocalOpening =")));
  assert.match(handler, /previewAuditOpeningInInspector\(preview/);
  assert.match(handler, /const progressPayload = openingPreparationProgress\(progress\)/);
  assert.match(handler, /progressPayload\s*\)/);
  // Preserve both normal public proof construction and the accepted action's
  // actual openings. Redacting proof requirements is not a confidentiality fix.
  assert.match(source, /stagePreparedLocalAction\?\.\(\{[\s\S]*?openings, rngReveals, shuffleProofs/);
  assert.match(source, /await appendAppliedSequencedAction\(message\);[\s\S]*?relaySequencedAction\(message\)/);
});
