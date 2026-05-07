import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/akh/DuskDawnTest.java",
  "tests": [
    {
      "name": "testCastDusk",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Watchwolf",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 5
        }
      ]
    },
    {
      "name": "testCastDuskFromGraveyardFail",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Watchwolf",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 4
        }
      ]
    },
    {
      "name": "testCastDawnFromGraveyard",
      "operations": []
    },
    {
      "name": "testCastDawnFail",
      "operations": []
    }
  ]
});
