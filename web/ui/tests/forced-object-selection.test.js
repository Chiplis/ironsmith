import test from "node:test";
import assert from "node:assert/strict";
import {
  forcedObjectSelectionCommand,
  localForcedObjectSelectionCommand,
} from "../src/lib/forced-object-selection.js";
import { selectObjectSyncMetadataForCommand } from "../src/lib/sync-commands.js";

const chosenCard = {
  id: 47,
  name: "Lightning Bolt",
  legal: true,
  selection_identity: "hidden_reference",
  hidden_ref: { owner: 1, slot: 47, commitment: "ziffle:deck:47" },
  reveal_policy: "public",
};
const discard = {
  kind: "select_objects",
  player: 1,
  description: "Choose 1 card to discard",
  min: 1,
  max: 1,
  allow_partial_completion: false,
  candidates: [chosenCard],
};

test("Thoughtseize's forced discard becomes a normal answer with its reveal metadata", () => {
  const state = { perspective: 1, decision: discard };
  const command = localForcedObjectSelectionCommand(state);
  assert.deepEqual(command, { type: "select_objects", object_ids: [47] });
  assert.deepEqual(selectObjectSyncMetadataForCommand(command, state), {
    stableIds: [null],
    hiddenRefs: [chosenCard.hidden_ref],
  });
});

test("only the deciding frontend answers when peers have different hidden candidates", () => {
  const ownerState = { perspective: 1, decision: discard };
  const peerState = {
    perspective: 0,
    decision: { ...discard, candidates: [
      { id: 47, name: "Hidden Card", legal: true },
      { id: 48, name: "Hidden Card", legal: true },
    ] },
  };
  assert.ok(localForcedObjectSelectionCommand(ownerState));
  assert.equal(localForcedObjectSelectionCommand(peerState), null);
  // Even a peer that knows the chosen card must wait for the actor's answer.
  assert.equal(localForcedObjectSelectionCommand({ perspective: 0, decision: discard }), null);
});

test("optional choices and searches that allow failure still ask the player", () => {
  for (const decision of [
    { ...discard, min: 0 },
    { ...discard, allow_partial_completion: true },
    { ...discard, max: 2 },
    { ...discard, candidates: [chosenCard, { id: 48, legal: true }] },
    { ...discard, candidates: [{ ...chosenCard, legal: false }] },
    { ...discard, candidates: [{ id: 47 }] },
    { ...discard, candidates: [] },
  ]) {
    assert.equal(forcedObjectSelectionCommand(decision), null);
  }
});

test("automation requires a live decision and a known local actor", () => {
  for (const state of [
    null,
    { decision: discard },
    { perspective: null, decision: discard },
    { perspective: 1, decision: { ...discard, player: null } },
    { perspective: 1, decision: discard, game_over: { winner: 0 } },
  ]) {
    assert.equal(localForcedObjectSelectionCommand(state), null);
  }
  assert.equal(forcedObjectSelectionCommand({ ...discard, kind: "targets" }), null);
});
