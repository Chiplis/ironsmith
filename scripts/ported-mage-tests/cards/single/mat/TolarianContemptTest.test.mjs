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
          "op": "addTarget",
          "player": 0,
          "target": "Raging Goblin"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Memnite"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "TestPlayer.TARGET_SKIP"
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
          "name": "playerD",
          "count": 0
        },
        {
          "op": "assertLibraryCount",
          "name": "Raging Goblin",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "name": "playerC",
          "count": 1
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Memnite",
          "counter": "REJECTION",
          "count": 1
        },
        {
          "op": "assertLibraryCount",
          "name": "Memnite",
          "count": 1
        }
      ]
    }
  ]
});
