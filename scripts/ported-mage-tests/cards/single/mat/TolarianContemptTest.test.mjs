import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/mat/TolarianContemptTest.java",
  "tests": [
    {
      "name": "testEachOpponent",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Tolarian Contempt",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 5
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": "playerD",
          "name": "Raging Goblin",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": "playerC",
          "name": "Memnite",
          "count": 2
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Tolarian Contempt"
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Tolarian Contempt",
          "count": 1,
          "onTop": true
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "resolutions": 2
        },
        {
          "op": "assertCounterCount",
          "player": 3,
          "name": "Raging Goblin",
          "count": 1,
          "counter": "REJECTION"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": {
            "groups": [
              [],
              [
                "Memnite"
              ],
              [
                "Raging Goblin"
              ]
            ]
          }
        },
        {
          "op": "setChoice",
          "player": "playerD",
          "value": true
        },
        {
          "op": "setChoice",
          "player": "playerC",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "UNTAP"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": "playerD",
          "count": 0
        },
        {
          "op": "assertLibraryCount",
          "name": "Raging Goblin",
          "count": 1,
          "player": 3
        },
        {
          "op": "assertPermanentCount",
          "player": "playerC",
          "count": 1
        },
        {
          "op": "assertCounterCount",
          "player": 2,
          "name": "Memnite",
          "counter": "REJECTION",
          "count": 1
        },
        {
          "op": "assertLibraryCount",
          "name": "Memnite",
          "count": 1,
          "player": 2
        }
      ]
    }
  ]
});
