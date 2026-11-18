import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/spm/CheeringCrowdTest.java",
  "tests": [
    {
      "name": "testCheeringCrowd",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Cheering Crowd",
          "count": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": true
        },
        {
          "op": "setChoice",
          "player": "playerC",
          "value": true
        },
        {
          "op": "setChoice",
          "player": "playerD",
          "value": true
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "resolutions": 1
        },
        {
          "op": "assertManaPool",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "colors": "C",
          "amount": "1"
        },
        {
          "op": "waitStackResolved",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": "playerD",
          "resolutions": 1
        },
        {
          "op": "assertManaPool",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 3,
          "colors": "C",
          "amount": "2"
        },
        {
          "op": "waitStackResolved",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": "playerC",
          "resolutions": 1
        },
        {
          "op": "assertManaPool",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 2,
          "colors": "C",
          "amount": "3"
        },
        {
          "op": "waitStackResolved",
          "turn": 4,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "resolutions": 1
        },
        {
          "op": "assertManaPool",
          "turn": 4,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "colors": "C",
          "amount": "4"
        },
        {
          "op": "setStopAt",
          "turn": 4,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Cheering Crowd",
          "counter": "P1P1",
          "count": 4
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
