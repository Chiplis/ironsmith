#!/usr/bin/env node

import {
  addCustomCardWithAbility,
  assert,
  captureCheckpoint,
  concede,
  createNewGameAndPlayers,
  getAbilities,
  getBattlefield,
  getCheckpoint,
  getGraveyard,
  getHand,
  getId,
  getLibrary,
  getManaPool,
  getPermanent,
  getPlayer,
  initWasmGame,
  names,
  restoreCheckpoint,
  runCode,
  setSideboard,
} from "./wasm-test-harness.mjs";

async function main() {
  const { game } = await initWasmGame();
  createNewGameAndPlayers(game, {
    playerNames: ["Alice", "Bob", "Cara"],
    openingHandSize: 0,
  });

  game.addCardToZone(0, "Llanowar Elves", "Battlefield", true);
  game.addCardToHand(0, "Lightning Bolt");
  game.addCardToZone(0, "Forest", "Library", true);
  game.addCardToZone(0, "Mountain", "Graveyard", true);
  setSideboard(game, 0, ["Plains"]);

  runCode(game, (mutableCheckpoint) => {
    mutableCheckpoint.players[0].life = 13;
  });
  let checkpoint = getCheckpoint(game);
  assert(getPlayer(checkpoint, 0).life === 13, "runCode should import checkpoint mutations");

  const saved = captureCheckpoint(game);
  game.addCardToHand(0, "Mountain");
  restoreCheckpoint(game, saved);
  assert(names(getHand(game, 0)).filter((name) => name === "Mountain").length === 0, "restoreCheckpoint should roll back later changes");

  concede(game, 1);
  assert(getCheckpoint(game).players[1].hasLost, "concede helper should mark the player as lost");

  const customId = addCustomCardWithAbility(game, {
    name: "Ironsmith Harness Bear",
    typeLine: "Creature - Bear",
    oracleText: "Vigilance",
    power: "2",
    toughness: "2",
  });

  checkpoint = getCheckpoint(game);
  assert(names(getBattlefield(checkpoint, 0)).includes("Llanowar Elves"), "battlefield query should find permanents");
  assert(getPermanent(checkpoint, 0, "Llanowar Elves").name === "Llanowar Elves", "getPermanent should return a match");
  assert(getId(checkpoint, 0, "Llanowar Elves") > 0, "getId should return a runtime object id");
  assert(names(getHand(checkpoint, 0)).includes("Lightning Bolt"), "hand query should include visible hand card");
  assert(names(getLibrary(checkpoint, 0)).includes("Forest"), "library query should read the full checkpoint library");
  assert(names(getGraveyard(checkpoint, 0)).includes("Mountain"), "graveyard query should read the checkpoint graveyard");
  assert(getManaPool(checkpoint, 0).green === 0, "mana pool query should expose color fields");
  assert(getAbilities(game, customId).some((line) => line.includes("Vigilance")), "custom card helper should compile oracle text");

  console.log("wasm harness smoke test passed");
}

main().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
