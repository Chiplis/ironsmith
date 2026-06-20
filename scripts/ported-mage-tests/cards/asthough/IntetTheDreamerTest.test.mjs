import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/asthough/IntetTheDreamerTest.java",
  "tests": [
    {
      "name": "test_SplitCard",
      "operations": [
        {
          "op": "skipInitShuffling"
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Intet, the Dreamer",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Wax // Wane",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Intet, the Dreamer",
          "defender": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "COMBAT_DAMAGE",
          "player": null
        },
        {
          "op": "assertExileCount",
          "turn": 1,
          "phase": "COMBAT_DAMAGE",
          "player": 0,
          "name": "Wax // Wane",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "COMBAT_DAMAGE",
          "player": 0,
          "name": "Wax",
          "target": "Intet, the Dreamer"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Intet, the Dreamer",
          "power": 8,
          "toughness": 8
        }
      ]
    }
  ]
});
