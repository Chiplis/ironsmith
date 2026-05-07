import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/triggers/PollutedBondsTest.java",
  "tests": [
    {
      "name": "PollutedBondsSimple",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Polluted Bonds",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Forest",
          "count": 1
        },
        {
          "op": "playLand",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Forest"
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 22
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 18
        }
      ]
    },
    {
      "name": "PollutedBondsOwnLand",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Polluted Bonds",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Forest",
          "count": 1
        },
        {
          "op": "playLand",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Forest"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN"
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
        }
      ]
    }
  ]
});
