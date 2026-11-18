import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/ice/ZursWeirdingTest.java",
  "tests": [
    {
      "name": "testYes",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Zur's Weirding",
          "count": 1
        },
        {
          "op": "clearZone",
          "zone": "library",
          "player": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 1,
          "name": "Forest",
          "count": 1
        },
        {
          "op": "assertLibraryCount",
          "player": 1,
          "name": "Forest",
          "count": 1
        },
        {
          "op": "setOptionChoices",
          "player": 0,
          "kind": "boolean",
          "description": "Pay 2 life",
          "choices": [
            "Yes"
          ]
        },
        {
          "op": "assertActivePlayer",
          "player": 1,
          "turn": 2,
          "phase": "DRAW"
        },
        {
          "op": "waitStackResolved",
          "turn": 2,
          "phase": "DRAW"
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 18
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 20
        },
        {
          "op": "assertLibraryCount",
          "player": 1,
          "name": "Forest",
          "count": 0
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "graveyard",
            "player": 1,
            "name": "Forest"
          },
          "count": 1,
          "checks": []
        },
        {
          "op": "assertHandCount",
          "player": 1,
          "name": "Forest",
          "count": 0
        }
      ]
    },
    {
      "name": "testNo",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Zur's Weirding",
          "count": 1
        },
        {
          "op": "clearZone",
          "zone": "library",
          "player": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 1,
          "name": "Forest",
          "count": 1
        },
        {
          "op": "assertLibraryCount",
          "player": 1,
          "name": "Forest",
          "count": 1
        },
        {
          "op": "setOptionChoices",
          "player": 0,
          "kind": "boolean",
          "description": "Pay 2 life",
          "choices": [
            "No"
          ]
        },
        {
          "op": "assertActivePlayer",
          "player": 1,
          "turn": 2,
          "phase": "DRAW"
        },
        {
          "op": "waitStackResolved",
          "turn": 2,
          "phase": "DRAW"
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
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Forest",
          "count": 0
        },
        {
          "op": "assertHandCount",
          "player": 1,
          "name": "Forest",
          "count": 1
        }
      ]
    }
  ]
});
