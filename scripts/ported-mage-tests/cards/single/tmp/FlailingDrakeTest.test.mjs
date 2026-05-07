import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/tmp/FlailingDrakeTest.java",
  "tests": [
    {
      "name": "testIncreaseBlocker",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Flailing Drake",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Island",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Snapping Drake",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Flailing Drake",
          "defender": 1
        },
        {
          "op": "block",
          "turn": 3,
          "player": 1,
          "blocker": "Snapping Drake",
          "attacker": "Flailing Drake"
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Flailing Drake",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 1,
          "name": "Snapping Drake",
          "power": 4,
          "toughness": 3
        }
      ]
    },
    {
      "name": "testIncreaseBlocked",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Flailing Drake",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Island",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Snapping Drake",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 4,
          "player": 1,
          "attacker": "Snapping Drake",
          "defender": 1
        },
        {
          "op": "block",
          "turn": 4,
          "player": 0,
          "blocker": "Flailing Drake",
          "attacker": "Snapping Drake"
        },
        {
          "op": "setStopAt",
          "turn": 4,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Flailing Drake",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 1,
          "name": "Snapping Drake",
          "power": 4,
          "toughness": 3
        }
      ]
    }
  ]
});
