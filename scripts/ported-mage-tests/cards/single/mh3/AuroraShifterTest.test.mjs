import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/mh3/AuroraShifterTest.java",
  "tests": [
    {
      "name": "test_4_combats",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Aurora Shifter",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Grizzly Bears",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Aurora Shifter",
          "defender": 1
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "END_TURN"
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Aurora Shifter",
          "defender": 1
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "END_TURN"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Grizzly Bears"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "attack",
          "turn": 5,
          "player": 0,
          "attacker": "Grizzly Bears",
          "defender": 1
        },
        {
          "op": "setStopAt",
          "turn": 5,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 16
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": 0,
          "counter": "ENERGY",
          "count": 2
        }
      ]
    }
  ]
});
