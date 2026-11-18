import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/one/EncroachingMycosynthTest.java",
  "tests": [
    {
      "name": "testEncroachingMycosynth",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Balduvian Bears",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Balduvian Bears",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Balduvian Bears",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Encroaching Mycosynth",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "EXILED",
          "player": 0,
          "name": "Balduvian Bears",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Balduvian Bears",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "COMMAND",
          "player": 0,
          "name": "Balduvian Bears",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Balduvian Bears"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 0,
            "name": "Balduvian Bears"
          },
          "checks": [
            {
              "property": "cardType",
              "expected": true,
              "value": "Artifact"
            }
          ],
          "count": 2
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 0,
            "excludeCardType": "Land"
          },
          "checks": [
            {
              "property": "cardType",
              "expected": true,
              "value": "Artifact"
            }
          ]
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "hand",
            "player": 0,
            "name": "Balduvian Bears"
          },
          "checks": [
            {
              "property": "cardType",
              "expected": true,
              "value": "Artifact"
            }
          ],
          "count": 1
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "hand",
            "player": 0,
            "excludeCardType": "Land"
          },
          "checks": [
            {
              "property": "cardType",
              "expected": true,
              "value": "Artifact"
            }
          ]
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "library",
            "player": 0,
            "name": "Balduvian Bears"
          },
          "checks": [
            {
              "property": "cardType",
              "expected": true,
              "value": "Artifact"
            }
          ],
          "count": 1
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "library",
            "player": 0,
            "excludeCardType": "Land"
          },
          "checks": [
            {
              "property": "cardType",
              "expected": true,
              "value": "Artifact"
            }
          ]
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "command",
            "player": 0,
            "name": "Balduvian Bears"
          },
          "checks": [
            {
              "property": "cardType",
              "expected": true,
              "value": "Artifact"
            }
          ],
          "count": 1
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "command",
            "player": 0,
            "excludeCardType": "Land"
          },
          "checks": [
            {
              "property": "cardType",
              "expected": true,
              "value": "Artifact"
            }
          ]
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "exile",
            "player": 0,
            "name": "Balduvian Bears"
          },
          "checks": [
            {
              "property": "cardType",
              "expected": true,
              "value": "Artifact"
            }
          ],
          "count": 1
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "exile",
            "player": 0,
            "excludeCardType": "Land"
          },
          "checks": [
            {
              "property": "cardType",
              "expected": true,
              "value": "Artifact"
            }
          ]
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "graveyard",
            "player": 0,
            "name": "Balduvian Bears"
          },
          "checks": [
            {
              "property": "cardType",
              "expected": true,
              "value": "Artifact"
            }
          ],
          "count": 1
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "graveyard",
            "player": 0,
            "excludeCardType": "Land"
          },
          "checks": [
            {
              "property": "cardType",
              "expected": true,
              "value": "Artifact"
            }
          ]
        }
      ]
    }
  ]
});
