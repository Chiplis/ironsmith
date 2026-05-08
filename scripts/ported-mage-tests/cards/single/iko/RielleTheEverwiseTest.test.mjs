import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/iko/RielleTheEverwiseTest.java",
  "tests": [
    {
      "name": "test_CleanupDiscard_Opp_Response",
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
          "name": "Rielle, the Everwise",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Island",
          "count": 10
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Island",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Lightning Bolt",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Shock",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Mountain",
          "count": 2
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Island"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "CLEANUP",
          "player": 1,
          "name": "Lightning Bolt",
          "target": 0
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "CLEANUP",
          "player": 1
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "CLEANUP",
          "player": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "CLEANUP",
          "player": 1,
          "name": "Shock",
          "target": 0
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "CLEANUP",
          "player": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Island"
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "UPKEEP"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 15
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "count": 7
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Island",
          "count": 6
        }
      ]
    }
  ]
});
