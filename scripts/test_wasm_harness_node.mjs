#!/usr/bin/env node

import {
  addCustomCardWithAbility,
  assert,
  captureRuntimeState,
  concede,
  createNewGameAndPlayers,
  getBattlefield,
  getInspectionState,
  getGraveyard,
  getHand,
  getId,
  getLibrary,
  getManaPool,
  getPermanent,
  getPlayer,
  initWasmGame,
  names,
  restoreRuntimeState,
  releaseRuntimeState,
  setSideboard,
} from "./wasm-test-harness.mjs";

async function main() {
  const { game } = await initWasmGame({ pkg: "demo" });
  game.resetEmpty(["Alice", "Bob", "Cara"], 20);

  game.addCardToZone(0, "Llanowar Elves", "Battlefield", true);
  game.addCardToHand(0, "Lightning Bolt");
  game.addCardToZone(0, "Forest", "Library", true);
  game.addCardToZone(0, "Island", "Library", true);
  game.addCardToZone(0, "Mountain", "Graveyard", true);
  game.addCardToZone(2, "Plains", "Graveyard", true);
  setSideboard(game, 0, ["Plains"]);
  game.finishPuzzleSetup();

  game.setLife(0, 13);
  let checkpoint = getInspectionState(game);
  assert(getPlayer(checkpoint, 0).life === 13, 'native life setup should be visible to inspection');

  const saved = captureRuntimeState(game);
  game.addCardToHand(0, "Mountain");
  restoreRuntimeState(game, saved);
  assert(names(getHand(game, 0)).filter((name) => name === "Mountain").length === 0, "native restore should roll back later changes");

  releaseRuntimeState(saved);

  concede(game, 1);
  assert(getInspectionState(game).players[1].hasLost, "concede helper should mark the player as lost");

  const customId = addCustomCardWithAbility(game, {
    name: "Ironsmith Harness Bear",
    typeLine: "Creature - Bear",
    oracleText: "Vigilance",
    power: "2",
    toughness: "2",
  });

  checkpoint = getInspectionState(game);
  assert(names(getBattlefield(checkpoint, 0)).includes("Llanowar Elves"), "battlefield query should find permanents");
  assert(getPermanent(checkpoint, 0, "Llanowar Elves").name === "Llanowar Elves", "getPermanent should return a match");
  assert(getId(checkpoint, 0, "Llanowar Elves") > 0, "getId should return a runtime object id");
  assert(names(getHand(checkpoint, 0)).includes("Lightning Bolt"), "hand query should include visible hand card");
  assert(JSON.stringify(names(getLibrary(checkpoint, 0))) === JSON.stringify(["Island", "Forest"]), "library query must preserve the native top-first order");
  assert(names(getGraveyard(checkpoint, 0)).includes("Mountain"), "graveyard query should read the native graveyard");
  assert(getManaPool(checkpoint, 0).green === 0, "mana pool query should expose color fields");
  assert(game.objectDetails(BigInt(customId)).compiled_text.some((line) => line.includes("Vigilance")), "custom card helper should compile oracle text");
  game.addObjectCountersForSetup(BigInt(customId), "+1/+1", 3);
  assert(game.objectDetails(BigInt(customId)).power === 5, "native counter setup must affect calculated characteristics");
  game.clearPlayerZoneForSetup(0, "graveyard");
  assert(getGraveyard(game, 0).length === 0, "native zone clearing must remove the requested owner's cards");
  assert(names(getGraveyard(game, 2)).includes("Plains"), "native zone clearing must preserve the other owner's cards");
  assert(game.objectDetails(BigInt(customId)).compiled_text.includes("Vigilance"), "fixture edits must preserve executable abilities");

  console.log("wasm harness smoke test passed");
}

main().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
