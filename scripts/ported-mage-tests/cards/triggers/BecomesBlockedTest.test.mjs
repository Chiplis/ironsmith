import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/triggers/BecomesBlockedTest.java",
  "tests": [
    {
      "name": "testRabidElephant",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Rabid Elephant",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Savannah Lions",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Hill Giant",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Rabid Elephant",
          "defender": 1
        },
        {
          "op": "block",
          "turn": 1,
          "player": 1,
          "blocker": "Savannah Lions",
          "attacker": "Rabid Elephant"
        },
        {
          "op": "block",
          "turn": 1,
          "player": 1,
          "blocker": "Hill Giant",
          "attacker": "Rabid Elephant"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "COMBAT_DAMAGE"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Rabid Elephant",
          "power": 7,
          "toughness": 8
        }
      ]
    }
  ]
});
