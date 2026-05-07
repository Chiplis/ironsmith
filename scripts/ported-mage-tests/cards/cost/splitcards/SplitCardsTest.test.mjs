import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/cost/splitcards/SplitCardsTest.java",
  "tests": [
    {
      "name": "testReturnCardFromSoulfireGrandMaster",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Volcanic Island",
          "count": 6
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Soulfire Grand Master",
          "count": 1
        }
      ]
    }
  ]
});
