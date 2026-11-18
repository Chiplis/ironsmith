import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/usg/SporogenesisTest.java",
  "tests": [
    {
      "name": "sporogenesisTest",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Sporogenesis",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Skophos Warleader",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Barony Vampire",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Warpath Ghoul",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Douser of Lights",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Skophos Warleader"
        },
        {
          "op": "assertCounterCount",
          "turn": 1,
          "phase": "END_TURN",
          "player": 0,
          "name": "Skophos Warleader",
          "counter": "FUNGUS",
          "count": 1,
          "seatIndependent": true
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Skophos Warleader"
        },
        {
          "op": "assertCounterCount",
          "turn": 3,
          "phase": "END_TURN",
          "player": 0,
          "name": "Skophos Warleader",
          "counter": "FUNGUS",
          "count": 2,
          "seatIndependent": true
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Barony Vampire"
        },
        {
          "op": "assertCounterCount",
          "turn": 5,
          "phase": "END_TURN",
          "player": 0,
          "name": "Barony Vampire",
          "counter": "FUNGUS",
          "count": 1,
          "seatIndependent": true
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Warpath Ghoul"
        },
        {
          "op": "assertCounterCount",
          "turn": 7,
          "phase": "END_TURN",
          "player": 0,
          "name": "Warpath Ghoul",
          "counter": "FUNGUS",
          "count": 1,
          "seatIndependent": true
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Warpath Ghoul"
        },
        {
          "op": "assertCounterCount",
          "turn": 9,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Warpath Ghoul",
          "counter": "FUNGUS",
          "count": 2,
          "seatIndependent": true
        },
        {
          "op": "activateAbility",
          "turn": 9,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "ability": "{R}, Sacrifice"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Warpath Ghoul"
        },
        {
          "op": "assertPowerToughness",
          "turn": 9,
          "phase": "END_TURN",
          "player": 0,
          "name": "Skophos Warleader",
          "power": 5,
          "toughness": 5
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Douser of Lights"
        },
        {
          "op": "assertCounterCount",
          "turn": 11,
          "phase": "END_TURN",
          "player": 0,
          "name": "Douser of Lights",
          "counter": "FUNGUS",
          "count": 1,
          "seatIndependent": true
        },
        {
          "op": "activateAbility",
          "turn": 12,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "ability": "{R}, Sacrifice"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Sporogenesis"
        },
        {
          "op": "assertCounterCount",
          "turn": 12,
          "phase": "END_TURN",
          "player": 0,
          "name": "Skophos Warleader",
          "counter": "FUNGUS",
          "count": 0,
          "seatIndependent": true
        },
        {
          "op": "assertCounterCount",
          "turn": 12,
          "phase": "END_TURN",
          "player": 0,
          "name": "Barony Vampire",
          "counter": "FUNGUS",
          "count": 0,
          "seatIndependent": true
        },
        {
          "op": "assertCounterCount",
          "turn": 12,
          "phase": "END_TURN",
          "player": 0,
          "name": "Douser of Lights",
          "counter": "FUNGUS",
          "count": 0,
          "seatIndependent": true
        },
        {
          "op": "assertPowerToughness",
          "turn": 12,
          "phase": "END_TURN",
          "player": 0,
          "name": "Skophos Warleader",
          "power": 5,
          "toughness": 5
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 13,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Warpath Ghoul",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Sporogenesis",
          "count": 1
        }
      ]
    }
  ]
});
