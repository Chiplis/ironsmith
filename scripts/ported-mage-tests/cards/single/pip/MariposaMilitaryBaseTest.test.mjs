import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/pip/MariposaMilitaryBaseTest.java",
  "tests": [
    {
      "name": "test_No_Radiation",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Mariposa Military Base",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 5
        },
        {
          "op": "playLand",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Mariposa Military Base"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": false
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{5}"
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
          "op": "assertTappedCount",
          "name": "Swamp",
          "tapped": true,
          "count": 5
        },
        {
          "op": "assertTappedCount",
          "name": "Mariposa Military Base",
          "tapped": true,
          "count": 1
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "count": 0,
          "name": 1
        }
      ]
    },
    {
      "name": "test_Radiation",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Mariposa Military Base",
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
          "op": "playLand",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Mariposa Military Base"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "activateAbility",
          "turn": 3,
          "phase": "UPKEEP",
          "player": 0,
          "ability": "{5}"
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "DRAW"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertTappedCount",
          "name": "Swamp",
          "tapped": true,
          "count": 3
        },
        {
          "op": "assertTappedCount",
          "name": "Mariposa Military Base",
          "tapped": true,
          "count": 1
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "count": 0,
          "name": 2
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": 0,
          "counter": "RAD",
          "count": 2
        }
      ]
    }
  ]
});
