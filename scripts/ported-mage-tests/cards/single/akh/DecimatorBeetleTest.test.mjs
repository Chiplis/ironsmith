import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/akh/DecimatorBeetleTest.java",
  "tests": [
    {
      "name": "targetOpponentCreatureWithDecimator",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Decimator Beetle",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Grizzly Bears",
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
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Decimator Beetle"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Grizzly Bears"
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Decimator Beetle",
          "defender": 1
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Grizzly Bears"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Hill Giant"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "END_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Grizzly Bears",
          "power": 2,
          "toughness": 2
        },
        {
          "op": "assertPowerToughness",
          "player": 1,
          "name": "Hill Giant",
          "power": 2,
          "toughness": 2
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Grizzly Bears",
          "counter": "M1M1",
          "count": 0
        },
        {
          "op": "assertCounterCount",
          "player": 1,
          "name": "Hill Giant",
          "counter": "M1M1",
          "count": 1
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 16
        }
      ]
    }
  ]
});
