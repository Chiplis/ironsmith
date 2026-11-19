import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/spm/ParkerLuckTest.java",
  "tests": [
    {
      "name": "testParkerLuck",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "skipInitShuffling"
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Parker Luck",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": "playerD",
          "name": "Bear Cub",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": "playerC",
          "name": "Fugitive Wizard",
          "count": 1
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": {
            "groups": [
              [
                2,
                3
              ]
            ]
          }
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "END_TURN",
          "source": "Parker Luck",
          "abilityKind": "Triggered",
          "count": 1,
          "targetCount": 2,
          "completeTargets": true
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "END_TURN"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "CLEANUP"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertLife",
          "player": "playerC",
          "life": 18
        },
        {
          "op": "assertLife",
          "player": "playerD",
          "life": 19
        },
        {
          "op": "assertHandCount",
          "player": "playerC",
          "count": 1
        },
        {
          "op": "assertHandCount",
          "player": "playerD",
          "count": 1
        }
      ],
      "turnOrder": [
        0,
        3,
        2,
        1
      ]
    },
    {
      "name": "testParkerLuckOneLibraryEmpty",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "skipInitShuffling"
        },
        {
          "op": "clearZone",
          "player": "playerC",
          "zone": "library"
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Parker Luck",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": "playerD",
          "name": "Bear Cub",
          "count": 1
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": {
            "groups": [
              [
                2,
                3
              ]
            ]
          }
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "END_TURN",
          "source": "Parker Luck",
          "abilityKind": "Triggered",
          "count": 1,
          "targetCount": 2,
          "completeTargets": true
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "END_TURN"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "CLEANUP"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertLife",
          "player": "playerC",
          "life": 18
        },
        {
          "op": "assertHandCount",
          "player": "playerD",
          "count": 1
        }
      ],
      "turnOrder": [
        0,
        3,
        2,
        1
      ]
    }
  ]
});
