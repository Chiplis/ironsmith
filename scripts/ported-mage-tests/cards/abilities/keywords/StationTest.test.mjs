import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/abilities/keywords/StationTest.java",
  "tests": [
    {
      "name": "testNoStation",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Galvanizing Sawship",
          "count": 1
        },
        {
          "op": "setStrictChooseMode",
          "value": true
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
          "op": "assertCounterCount",
          "player": 0,
          "name": "Galvanizing Sawship",
          "counter": "CHARGE",
          "count": 0
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 0,
            "name": "Galvanizing Sawship"
          },
          "checks": [
            {
              "property": "cardType",
              "expected": true,
              "value": "Artifact"
            },
            {
              "property": "subtype",
              "expected": true,
              "value": "Spacecraft"
            },
            {
              "property": "cardType",
              "expected": false,
              "value": "Creature"
            },
            {
              "property": "keyword",
              "value": "Haste",
              "expected": false
            },
            {
              "property": "keyword",
              "value": "Flying",
              "expected": false
            }
          ],
          "count": 1
        }
      ]
    },
    {
      "name": "testStationInsufficient",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Galvanizing Sawship",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Riot Devils",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "Station"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Riot Devils"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
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
          "op": "assertCounterCount",
          "player": 0,
          "name": "Galvanizing Sawship",
          "counter": "CHARGE",
          "count": 2
        },
        {
          "op": "assertTapped",
          "player": 0,
          "name": "Riot Devils",
          "tapped": true
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 0,
            "name": "Galvanizing Sawship"
          },
          "checks": [
            {
              "property": "cardType",
              "expected": true,
              "value": "Artifact"
            },
            {
              "property": "subtype",
              "expected": true,
              "value": "Spacecraft"
            },
            {
              "property": "cardType",
              "expected": false,
              "value": "Creature"
            },
            {
              "property": "keyword",
              "value": "Haste",
              "expected": false
            },
            {
              "property": "keyword",
              "value": "Flying",
              "expected": false
            }
          ],
          "count": 1
        }
      ]
    },
    {
      "name": "testStationSufficient",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Galvanizing Sawship",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Hill Giant",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "Station"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Hill Giant"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
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
          "op": "assertCounterCount",
          "player": 0,
          "name": "Galvanizing Sawship",
          "counter": "CHARGE",
          "count": 3
        },
        {
          "op": "assertTapped",
          "player": 0,
          "name": "Hill Giant",
          "tapped": true
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 0,
            "name": "Galvanizing Sawship"
          },
          "checks": [
            {
              "property": "cardType",
              "expected": true,
              "value": "Artifact"
            },
            {
              "property": "subtype",
              "expected": true,
              "value": "Spacecraft"
            },
            {
              "property": "cardType",
              "expected": true,
              "value": "Creature"
            },
            {
              "property": "power",
              "expected": 6
            },
            {
              "property": "toughness",
              "expected": 5
            },
            {
              "property": "keyword",
              "value": "Haste",
              "expected": true
            },
            {
              "property": "keyword",
              "value": "Flying",
              "expected": true
            }
          ],
          "count": 1
        }
      ]
    },
    {
      "name": "testTapestryWarden",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Galvanizing Sawship",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Tapestry Warden",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Riot Devils",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "Station"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Riot Devils"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
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
          "op": "assertCounterCount",
          "player": 0,
          "name": "Galvanizing Sawship",
          "counter": "CHARGE",
          "count": 3
        },
        {
          "op": "assertTapped",
          "player": 0,
          "name": "Riot Devils",
          "tapped": true
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 0,
            "name": "Galvanizing Sawship"
          },
          "checks": [
            {
              "property": "cardType",
              "expected": true,
              "value": "Artifact"
            },
            {
              "property": "subtype",
              "expected": true,
              "value": "Spacecraft"
            },
            {
              "property": "cardType",
              "expected": true,
              "value": "Creature"
            },
            {
              "property": "power",
              "expected": 6
            },
            {
              "property": "toughness",
              "expected": 5
            },
            {
              "property": "keyword",
              "value": "Haste",
              "expected": true
            },
            {
              "property": "keyword",
              "value": "Flying",
              "expected": true
            }
          ],
          "count": 1
        }
      ]
    },
    {
      "name": "testEntropicBattlecruiser",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Entropic Battlecruiser",
          "count": 1
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
          "zone": "HAND",
          "player": 0,
          "name": "Specter's Wail",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Wastes",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "Station"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Balduvian Bears"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Specter's Wail"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": 1
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
          "op": "assertLife",
          "player": 1,
          "life": 17
        }
      ]
    }
  ]
});
