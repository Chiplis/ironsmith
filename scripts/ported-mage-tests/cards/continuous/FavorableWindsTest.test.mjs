import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/continuous/FavorableWindsTest.java",
  "tests": [
    {
      "name": "testBoostForFlyingCreatures",
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
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Favorable Winds",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Merfolk Looter",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Sky Spirit",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Merfolk Looter",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Sky Spirit",
          "count": 2
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "BEGIN_COMBAT"
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
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "name": "Merfolk Looter",
            "player": 0
          },
          "count": 2,
          "checks": [
            {
              "property": "power",
              "expected": 1
            },
            {
              "property": "toughness",
              "expected": 1
            }
          ]
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "name": "Sky Spirit",
            "player": 0
          },
          "count": 2,
          "checks": [
            {
              "property": "power",
              "expected": 3
            },
            {
              "property": "toughness",
              "expected": 3
            }
          ]
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "name": "Merfolk Looter",
            "player": 1
          },
          "count": 2,
          "checks": [
            {
              "property": "power",
              "expected": 1
            },
            {
              "property": "toughness",
              "expected": 1
            }
          ]
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "name": "Sky Spirit",
            "player": 1
          },
          "count": 2,
          "checks": [
            {
              "property": "power",
              "expected": 2
            },
            {
              "property": "toughness",
              "expected": 2
            }
          ]
        }
      ]
    },
    {
      "name": "testMultiBoostForFlyingCreatures",
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
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Favorable Winds",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Merfolk Looter",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Sky Spirit",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Merfolk Looter",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Sky Spirit",
          "count": 2
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "BEGIN_COMBAT"
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
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "name": "Merfolk Looter",
            "player": 0
          },
          "count": 2,
          "checks": [
            {
              "property": "power",
              "expected": 1
            },
            {
              "property": "toughness",
              "expected": 1
            }
          ]
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "name": "Sky Spirit",
            "player": 0
          },
          "count": 2,
          "checks": [
            {
              "property": "power",
              "expected": 5
            },
            {
              "property": "toughness",
              "expected": 5
            }
          ]
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "name": "Merfolk Looter",
            "player": 1
          },
          "count": 2,
          "checks": [
            {
              "property": "power",
              "expected": 1
            },
            {
              "property": "toughness",
              "expected": 1
            }
          ]
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "name": "Sky Spirit",
            "player": 1
          },
          "count": 2,
          "checks": [
            {
              "property": "power",
              "expected": 2
            },
            {
              "property": "toughness",
              "expected": 2
            }
          ]
        }
      ]
    }
  ]
});
