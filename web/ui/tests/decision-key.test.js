import test from "node:test";
import assert from "node:assert/strict";
import { decisionKey } from "../src/lib/decision-key.js";

test("select_objects decisions remount when prompt changes from discard to search", () => {
  const discardDecision = {
    kind: "select_objects",
    player: 1,
    source_id: 42,
    source_name: "Boseiju, Who Endures",
    description: "Discard a card",
    candidates: [
      { id: 42, name: "Boseiju, Who Endures", legal: true },
    ],
  };
  const searchDecision = {
    kind: "select_objects",
    player: 2,
    source_id: 99,
    source_name: "Boseiju, Who Endures",
    description: "Search your library for a land card with a basic land type",
    candidates: [
      { id: 700, name: "Forest", legal: true },
      { id: 701, name: "Plains", legal: true },
    ],
  };

  assert.notEqual(
    decisionKey(discardDecision),
    decisionKey(searchDecision),
  );
});

test("select_objects decisions with same candidates but different descriptions get different keys", () => {
  const base = {
    kind: "select_objects",
    player: 1,
    source_id: 55,
    source_name: "Choice Probe",
    candidates: [
      { id: 1, name: "Forest", legal: true },
      { id: 2, name: "Island", legal: true },
    ],
  };

  assert.notEqual(
    decisionKey({ ...base, description: "Discard a card" }),
    decisionKey({ ...base, description: "Search your library" }),
  );
});
