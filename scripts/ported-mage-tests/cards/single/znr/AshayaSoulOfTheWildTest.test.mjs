import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/znr/AshayaSoulOfTheWildTest.java",
  "tests": [
    {
      "name": "testAshaya",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Ashaya, Soul of the Wild",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 5
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Grizzly Bears",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Ashaya, Soul of the Wild"
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
            "name": "Ashaya, Soul of the Wild"
          },
          "count": 1,
          "checks": [
            {
              "property": "cardType",
              "value": "LAND",
              "expected": true
            },
            {
              "property": "subtype",
              "value": "FOREST",
              "expected": true
            }
          ]
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 0,
            "name": "Ashaya, Soul of the Wild"
          },
          "count": 1,
          "checks": [
            {
              "property": "cardType",
              "value": "CREATURE",
              "expected": true
            },
            {
              "property": "subtype",
              "value": "ELEMENTAL",
              "expected": true
            }
          ]
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Ashaya, Soul of the Wild",
          "ability": "Add {G}",
          "expected": true
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Ashaya, Soul of the Wild",
          "power": 7,
          "toughness": 7
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 0,
            "name": "Grizzly Bears"
          },
          "count": 1,
          "checks": [
            {
              "property": "cardType",
              "value": "LAND",
              "expected": true
            },
            {
              "property": "subtype",
              "value": "FOREST",
              "expected": true
            }
          ]
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 0,
            "name": "Grizzly Bears"
          },
          "count": 1,
          "checks": [
            {
              "property": "cardType",
              "value": "CREATURE",
              "expected": true
            },
            {
              "property": "subtype",
              "value": "BEAR",
              "expected": true
            }
          ]
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Grizzly Bears",
          "ability": "Add {G}",
          "expected": true
        }
      ]
    },
    {
      "name": "testAshayaNoAbilities",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Ashaya, Soul of the Wild",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Kenrith's Transformation",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 7
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Grizzly Bears",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Ashaya, Soul of the Wild"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Kenrith's Transformation",
          "target": "Ashaya, Soul of the Wild"
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
            "name": "Ashaya, Soul of the Wild"
          },
          "count": 1,
          "checks": [
            {
              "property": "cardType",
              "value": "LAND",
              "expected": false
            }
          ]
        },
        {
          "op": "unsupported",
          "source": "assertNotSubtype(ashaya, SubType.ELEMENTAL)"
        },
        {
          "op": "unsupported",
          "source": "assertNotSubtype(ashaya, SubType.FOREST)"
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 0,
            "name": "Ashaya, Soul of the Wild"
          },
          "count": 1,
          "checks": [
            {
              "property": "cardType",
              "value": "CREATURE",
              "expected": true
            },
            {
              "property": "subtype",
              "value": "ELK",
              "expected": true
            }
          ]
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Ashaya, Soul of the Wild",
          "ability": "Add {G}",
          "expected": false
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Ashaya, Soul of the Wild",
          "power": 3,
          "toughness": 3
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 0,
            "name": "Grizzly Bears"
          },
          "count": 1,
          "checks": [
            {
              "property": "cardType",
              "value": "LAND",
              "expected": true
            },
            {
              "property": "subtype",
              "value": "FOREST",
              "expected": true
            }
          ]
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 0,
            "name": "Grizzly Bears"
          },
          "count": 1,
          "checks": [
            {
              "property": "cardType",
              "value": "CREATURE",
              "expected": true
            },
            {
              "property": "subtype",
              "value": "BEAR",
              "expected": true
            }
          ]
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Grizzly Bears",
          "ability": "Add {G}",
          "expected": true
        }
      ]
    },
    {
      "name": "testAshayaVolrathsShapeshifter",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Grizzly Bears",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Volrath's Shapeshifter@ashaya.copy",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Ashaya, Soul of the Wild",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Ashaya, Soul of the Wild",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Volrath's Shapeshifter",
          "objectAlias": "@ashaya.copy"
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Volrath's Shapeshifter",
          "count": 1
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
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
            "alias": "@ashaya.copy"
          },
          "count": 1,
          "checks": [
            {
              "property": "calculatedName",
              "expected": "Ashaya, Soul of the Wild"
            },
            {
              "property": "cardType",
              "value": "Land",
              "expected": true
            },
            {
              "property": "subtype",
              "value": "Forest",
              "expected": true
            }
          ]
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 0,
            "alias": "@ashaya.copy"
          },
          "count": 1,
          "checks": [
            {
              "property": "calculatedName",
              "expected": "Ashaya, Soul of the Wild"
            },
            {
              "property": "cardType",
              "value": "Creature",
              "expected": true
            },
            {
              "property": "subtype",
              "value": "Elemental",
              "expected": true
            }
          ]
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Ashaya, Soul of the Wild",
          "ability": "Add {G}",
          "expected": true
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Ashaya, Soul of the Wild",
          "power": 5,
          "toughness": 5
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "name": "Grizzly Bears",
            "player": 0
          },
          "count": 1,
          "checks": [
            {
              "property": "cardType",
              "value": "Land",
              "expected": true
            },
            {
              "property": "subtype",
              "value": "Forest",
              "expected": true
            }
          ]
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "name": "Grizzly Bears",
            "player": 0
          },
          "count": 1,
          "checks": [
            {
              "property": "cardType",
              "value": "Creature",
              "expected": true
            },
            {
              "property": "subtype",
              "value": "Bear",
              "expected": true
            }
          ]
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Grizzly Bears",
          "ability": "Add {G}",
          "expected": true
        }
      ]
    }
  ]
});
