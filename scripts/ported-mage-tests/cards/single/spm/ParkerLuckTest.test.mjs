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
          "target": "playerC"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "playerD"
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
          "player": 0,
          "name": "playerC",
          "count": 1
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "playerD",
          "count": 1
        }
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
          "target": "playerC"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "playerD"
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
          "player": 0,
          "name": "playerD",
          "count": 1
        }
      ]
    }
  ]
});
