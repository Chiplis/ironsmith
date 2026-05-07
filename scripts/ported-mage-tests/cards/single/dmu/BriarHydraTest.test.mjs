import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/dmu/BriarHydraTest.java",
  "tests": [
    {
      "name": "test_NoDomain",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Briar Hydra",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Briar Hydra",
          "defender": 1
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Briar Hydra"
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
          "name": "Briar Hydra",
          "power": 6,
          "toughness": 6
        }
      ]
    },
    {
      "name": "test_Domain_2",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Briar Hydra",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Bayou",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Briar Hydra",
          "defender": 1
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Briar Hydra"
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
          "name": "Briar Hydra",
          "power": 8,
          "toughness": 8
        }
      ]
    }
  ]
});
