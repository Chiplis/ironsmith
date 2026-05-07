import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/cost/sacrifice/SacrificeLandTest.java",
  "tests": [
    {
      "name": "testRollback",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Soldevi Excavations",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 1
        },
        {
          "op": "playLand",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Soldevi Excavations"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "RandomUtil.nextBoolean()"
        },
        {
          "op": "unsupported",
          "source": "rollbackAfterActionsStart()"
        },
        {
          "op": "playLand",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Soldevi Excavations"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "RandomUtil.nextBoolean()"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
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
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Soldevi Excavations",
          "count": "sacSecondLand ? 1 : 0"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Island",
          "count": "sacSecondLand ? 1 : 0"
        }
      ]
    },
    {
      "name": "testMultiple",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Soldevi Excavations",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Heart of Yavimaya",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Lake of the Dead",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 1
        },
        {
          "op": "playLand",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Soldevi Excavations"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Yes"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Island"
        },
        {
          "op": "playLand",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Heart of Yavimaya"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Yes"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Forest"
        },
        {
          "op": "playLand",
          "turn": 5,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Lake of the Dead"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Yes"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Swamp"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 5,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Soldevi Excavations",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Island",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Heart of Yavimaya",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Forest",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Lake of the Dead",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Swamp",
          "count": 1
        }
      ]
    }
  ]
});
