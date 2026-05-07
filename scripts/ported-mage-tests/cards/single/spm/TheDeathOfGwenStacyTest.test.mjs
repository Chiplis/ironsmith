import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/spm/TheDeathOfGwenStacyTest.java",
  "tests": [
    {
      "name": "testTheDeathOfGwenStacy",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "The Death of Gwen Stacy",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": "playerC",
          "name": "Mountain",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Mountain",
          "count": 1
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Mountain"
        },
        {
          "op": "addTarget",
          "player": "playerD",
          "target": "TestPlayer.TARGET_SKIP"
        },
        {
          "op": "addTarget",
          "player": "playerC",
          "target": "Mountain"
        },
        {
          "op": "addTarget",
          "player": 1,
          "target": "Mountain"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
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
          "op": "assertLife",
          "player": "playerC",
          "life": 20
        },
        {
          "op": "assertLife",
          "player": "playerD",
          "life": 17
        }
      ]
    }
  ]
});
