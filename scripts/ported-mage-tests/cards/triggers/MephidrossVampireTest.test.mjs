import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/triggers/MephidrossVampireTest.java",
  "tests": [
    {
      "name": "testMultiTriggers",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Mephidross Vampire",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Ornithopter",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 2,
          "player": 1,
          "attacker": "Mephidross Vampire",
          "defender": 1
        },
        {
          "op": "block",
          "turn": 2,
          "player": 0,
          "blocker": "Ornithopter",
          "attacker": "Mephidross Vampire"
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 20
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 20
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Mephidross Vampire",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 1,
          "name": "Mephidross Vampire",
          "power": 4,
          "toughness": 5
        }
      ]
    }
  ]
});
