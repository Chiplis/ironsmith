import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/plc/ChronozoaTest.java",
  "tests": [
    {
      "name": "testVanishing",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Chronozoa",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Chronozoa"
        },
        {
          "op": "setStopAt",
          "turn": 5,
          "phase": "DRAW"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Chronozoa",
          "counter": "TIME",
          "count": 1
        }
      ]
    },
    {
      "name": "testDuplicationEffect",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Chronozoa",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Chronozoa"
        },
        {
          "op": "setStopAt",
          "turn": 9,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "count": 1
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "cardType": "Creature"
          },
          "count": 2,
          "checks": [
            {
              "property": "name",
              "expected": "Chronozoa"
            },
            {
              "property": "token",
              "expected": true
            },
            {
              "property": "counterKinds",
              "expected": 1
            },
            {
              "property": "counter",
              "value": "time",
              "expected": 2
            }
          ]
        }
      ]
    }
  ]
});
