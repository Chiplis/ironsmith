import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/zen/CobraTrapTest.java",
  "tests": [
    {
      "name": "testCard",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Cobra Trap",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Mountain",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Stone Rain",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Stone Rain",
          "target": "Forest"
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Cobra Trap"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Cast with alternative cost: {G}"
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
          "player": 0,
          "name": "Forest",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Snake Token",
          "count": 4
        }
      ]
    },
    {
      "name": "testCardNegative",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Cobra Trap",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Mountain",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Stone Rain",
          "count": 1
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "label": "Cast Cobra",
          "expected": false
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
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Forest",
          "count": 2
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Snake Token",
          "count": 0
        }
      ]
    }
  ]
});
