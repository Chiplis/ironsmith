import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/conditional/SejiriMerfolkTest.java",
  "tests": [
    {
      "name": "testWithoutPlains",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Sejiri Merfolk",
          "count": 1
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "DRAW"
        },
        {
          "op": "execute"
        }
      ]
    },
    {
      "name": "testWithPlains",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Sejiri Merfolk",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 1
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "DRAW"
        },
        {
          "op": "execute"
        }
      ]
    }
  ]
});
