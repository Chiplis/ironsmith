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
          "name": "Aurora Shifter@shifter",
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
          "op": "setOptionChoices",
          "player": 0,
          "kind": "boolean",
          "description": "Pay {E}{E}",
          "choices": [
            "No"
          ]
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Aurora Shifter@shifter",
          "defender": 1
        },
        {
          "op": "assertCounterCount",
          "name": 0,
          "count": 1,
          "player": 0,
          "counter": "ENERGY",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "setOptionChoices",
          "player": 0,
          "kind": "boolean",
          "description": "Pay {E}{E}",
          "choices": [
            "No"
          ]
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Aurora Shifter@shifter",
          "defender": 1
        },
        {
          "op": "assertCounterCount",
          "name": 0,
          "count": 2,
          "player": 0,
          "counter": "ENERGY",
          "turn": 3,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Grizzly Bears"
        },
        {
          "op": "setOptionChoices",
          "player": 0,
          "kind": "boolean",
          "description": "Pay {E}{E}",
          "choices": [
            "Yes"
          ]
        },
        {
          "op": "assertStackObject",
          "turn": 5,
          "phase": "BEGIN_COMBAT",
          "source": "Aurora Shifter@shifter",
          "count": 1,
          "abilityKind": "Triggered"
        },
        {
          "op": "waitStackResolved",
          "turn": 5,
          "phase": "BEGIN_COMBAT",
          "resolutions": 1
        },
        {
          "op": "assertStackObject",
          "turn": 5,
          "phase": "BEGIN_COMBAT",
          "source": "Aurora Shifter@shifter",
          "count": 1,
          "abilityKind": "Triggered",
          "objectTarget": "Grizzly Bears"
        },
        {
          "op": "waitStackResolved",
          "turn": 5,
          "phase": "BEGIN_COMBAT",
          "resolutions": 1
        },
        {
          "op": "assertCounterCount",
          "name": 0,
          "count": 0,
          "player": 0,
          "counter": "ENERGY",
          "turn": 5,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "attack",
          "turn": 5,
          "player": 0,
          "attacker": "Aurora Shifter@shifter",
          "defender": 1
        },
        {
          "op": "assertCounterCount",
          "name": 0,
          "count": 2,
          "player": 0,
          "counter": "ENERGY",
          "turn": 5,
          "phase": "POSTCOMBAT_MAIN"
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
