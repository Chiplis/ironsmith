import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/split/CastSplitCardsWithSpliceTest.java",
  "tests": [
    {
      "name": "test_ThaliaGuardianOfThraben_CostModification_Fused",
      "operations": [
        {
          "op": "clearZone",
          "player": 0,
          "zone": "hand"
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Thalia, Guardian of Thraben",
          "count": 1
        }
      ]
    },
    {
      "name": "test_ThaliaGuardianOfThraben_CostModification_FusedWithSplice",
      "operations": [
        {
          "op": "clearZone",
          "player": 0,
          "zone": "hand"
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Thalia, Guardian of Thraben",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Everdream",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 3
        }
      ]
    }
  ]
});
