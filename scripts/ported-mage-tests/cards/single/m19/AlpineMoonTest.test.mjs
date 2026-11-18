import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/m19/AlpineMoonTest.java",
  "tests": [
    {
      "name": "testAlpineMoonAfterUrborg",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "name": "Alpine Moon",
          "player": 1,
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "name": "Mountain",
          "player": 1,
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "name": "Island",
          "player": 0,
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "name": "Urborg, Tomb of Yawgmoth",
          "player": 0,
          "count": 1
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "name": "Urborg, Tomb of Yawgmoth",
            "player": 0
          },
          "count": 1,
          "checks": [
            {
              "property": "subtype",
              "value": "Swamp",
              "expected": true
            }
          ]
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": "Urborg, Tomb of Yawgmoth"
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Alpine Moon"
        },
        {
          "op": "waitStackResolved",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Alpine Moon",
          "count": 1
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "name": "Urborg, Tomb of Yawgmoth",
            "player": 0
          },
          "count": 1,
          "checks": [
            {
              "property": "subtype",
              "value": "Swamp",
              "expected": false
            }
          ]
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "name": "Mountain",
            "player": 1
          },
          "count": 1,
          "checks": [
            {
              "property": "subtype",
              "value": "Swamp",
              "expected": true
            }
          ]
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "name": "Island",
            "player": 0
          },
          "count": 1,
          "checks": [
            {
              "property": "subtype",
              "value": "Swamp",
              "expected": true
            }
          ]
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Green"
        },
        {
          "op": "activateManaAbility",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "source": "Urborg, Tomb of Yawgmoth",
          "ability": "Add one mana of any color",
          "count": 1
        },
        {
          "op": "assertManaPool",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "colors": "G",
          "amount": 1
        }
      ]
    },
    {
      "name": "testAlpineMoonBeforeUrborg",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "name": "Alpine Moon",
          "player": 0,
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "name": "Mountain",
          "player": 0,
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "name": "Island",
          "player": 1,
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "name": "Urborg, Tomb of Yawgmoth",
          "player": 1,
          "count": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Urborg, Tomb of Yawgmoth"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Alpine Moon"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Alpine Moon",
          "count": 1
        },
        {
          "op": "playLand",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Urborg, Tomb of Yawgmoth"
        },
        {
          "op": "assertPermanentCount",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Urborg, Tomb of Yawgmoth",
          "count": 1
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "name": "Urborg, Tomb of Yawgmoth",
            "player": 1
          },
          "count": 1,
          "checks": [
            {
              "property": "subtype",
              "value": "Swamp",
              "expected": true
            }
          ]
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "name": "Mountain",
            "player": 0
          },
          "count": 1,
          "checks": [
            {
              "property": "subtype",
              "value": "Swamp",
              "expected": true
            }
          ]
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "name": "Island",
            "player": 1
          },
          "count": 1,
          "checks": [
            {
              "property": "subtype",
              "value": "Swamp",
              "expected": true
            }
          ]
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": "Green"
        },
        {
          "op": "activateManaAbility",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "source": "Urborg, Tomb of Yawgmoth",
          "ability": "Add one mana of any color",
          "count": 1
        },
        {
          "op": "assertManaPool",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "colors": "G",
          "amount": 1
        }
      ]
    }
  ]
});
