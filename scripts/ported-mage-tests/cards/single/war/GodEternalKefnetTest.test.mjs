import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/war/GodEternalKefnetTest.java",
  "tests": [
    {
      "name": "test_Reduce_NormalSpell",
      "operations": [
        {
          "op": "clearZone",
          "player": 0,
          "zone": "hand"
        },
        {
          "op": "clearZone",
          "player": 0,
          "zone": "library"
        },
        {
          "op": "skipInitShuffling"
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "God-Eternal Kefnet",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Precision Bolt",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Grizzly Bears",
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
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": 1
        },
        {
          "op": "setStrictChooseMode",
          "value": true
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
          "op": "assertHandCount",
          "player": 0,
          "name": "Precision Bolt",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Precision Bolt",
          "count": 0
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 17
        }
      ]
    },
    {
      "name": "test_Reduce_Split",
      "operations": [
        {
          "op": "clearZone",
          "player": 0,
          "zone": "hand"
        },
        {
          "op": "clearZone",
          "player": 0,
          "zone": "library"
        },
        {
          "op": "skipInitShuffling"
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "God-Eternal Kefnet",
          "count": 1
        }
      ]
    }
  ]
});
