import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/abilities/keywords/BattleCryTest.java",
  "tests": [
    {
      "name": "testBoostDurationUntilEndTurn",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Signal Pest",
          "count": 3
        },
        {
          "op": "attack",
          "turn": 2,
          "player": 1,
          "attacker": "Signal Pest",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 2,
          "player": 1,
          "attacker": "Signal Pest",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 2,
          "player": 1,
          "attacker": "Signal Pest",
          "defender": 1
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": "Battle cry"
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Signal Pest",
          "count": 3
        },
        {
          "op": "assertPowerToughness",
          "player": 1,
          "name": "Signal Pest",
          "power": 2,
          "toughness": 1
        }
      ]
    },
    {
      "name": "testBoostDurationNotNextTurn",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Signal Pest",
          "count": 3
        },
        {
          "op": "attack",
          "turn": 2,
          "player": 1,
          "attacker": "Signal Pest",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 2,
          "player": 1,
          "attacker": "Signal Pest",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 2,
          "player": 1,
          "attacker": "Signal Pest",
          "defender": 1
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": "Battle cry"
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "UPKEEP"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Signal Pest",
          "count": 3
        },
        {
          "op": "assertPowerToughness",
          "player": 1,
          "name": "Signal Pest",
          "power": 0,
          "toughness": 1
        }
      ]
    }
  ]
});
