import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/continuous/LandTypeChangingEffectsTest.java",
  "tests": [
    {
      "name": "testMagusOfTheMoonAndChromaticLantern",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Magus of the Moon",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Canopy Vista",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Plains",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Chromatic Lantern",
          "count": 1
        },
        {
          "op": "castSpell",
          "name": "Chromatic Lantern",
          "player": 1,
          "turn": 2,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertStackObject",
          "source": "Chromatic Lantern",
          "count": 1,
          "turn": 2,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "waitStackResolved",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Chromatic Lantern",
          "count": 1
        },
        {
          "op": "waitStackResolved",
          "turn": 4,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 1,
            "name": "Canopy Vista"
          },
          "count": 1,
          "checks": [
            {
              "property": "cardType",
              "expected": true,
              "value": "Land"
            },
            {
              "property": "subtype",
              "expected": true,
              "value": "Mountain"
            },
            {
              "property": "subtype",
              "expected": false,
              "value": "Forest"
            },
            {
              "property": "subtype",
              "expected": false,
              "value": "Plains"
            }
          ]
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": "Blue"
        },
        {
          "op": "activateManaAbility",
          "source": "Canopy Vista",
          "ability": "Add one mana of any color",
          "player": 1,
          "count": 1,
          "turn": 4,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertManaPool",
          "player": 1,
          "colors": "U",
          "amount": 1
        },
        {
          "op": "assertTappedCount",
          "player": 1,
          "name": "Canopy Vista",
          "count": 1,
          "tapped": true
        }
      ]
    },
    {
      "name": "testChromaticLanternBeforeMagusOfTheMoon",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Magus of the Moon",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Canopy Vista",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Plains",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Chromatic Lantern",
          "count": 1
        },
        {
          "op": "castSpell",
          "name": "Chromatic Lantern",
          "player": 1,
          "turn": 2,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertStackObject",
          "source": "Chromatic Lantern",
          "count": 1,
          "turn": 2,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "waitStackResolved",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Chromatic Lantern",
          "count": 1
        },
        {
          "op": "castSpell",
          "name": "Magus of the Moon",
          "player": 0,
          "turn": 3,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertStackObject",
          "source": "Magus of the Moon",
          "count": 1,
          "turn": 3,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "waitStackResolved",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Magus of the Moon",
          "count": 1
        },
        {
          "op": "waitStackResolved",
          "turn": 4,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 1,
            "name": "Canopy Vista"
          },
          "count": 1,
          "checks": [
            {
              "property": "cardType",
              "expected": true,
              "value": "Land"
            },
            {
              "property": "subtype",
              "expected": true,
              "value": "Mountain"
            },
            {
              "property": "subtype",
              "expected": false,
              "value": "Forest"
            },
            {
              "property": "subtype",
              "expected": false,
              "value": "Plains"
            }
          ]
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": "Blue"
        },
        {
          "op": "activateManaAbility",
          "source": "Canopy Vista",
          "ability": "Add one mana of any color",
          "player": 1,
          "count": 1,
          "turn": 4,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertManaPool",
          "player": 1,
          "colors": "U",
          "amount": 1
        },
        {
          "op": "assertTappedCount",
          "player": 1,
          "name": "Canopy Vista",
          "count": 1,
          "tapped": true
        }
      ]
    },
    {
      "name": "testLandDoesNotLooseOtherAbilities",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Aquitect's Will",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Forbidding Watchtower",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Plains",
          "count": 2
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Aquitect's Will",
          "target": "Forbidding Watchtower"
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Aquitect's Will",
          "count": 1,
          "objectTarget": "Forbidding Watchtower"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "resolutions": 1
        },
        {
          "op": "assertCounterCount",
          "name": "Forbidding Watchtower",
          "count": 1,
          "player": 1,
          "counter": "FLOOD",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "activateAbility",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "ability": "{1}{W}:",
          "source": "Forbidding Watchtower"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Aquitect's Will",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Forbidding Watchtower",
          "count": 1
        },
        {
          "op": "assertCounterCount",
          "player": 1,
          "name": "Forbidding Watchtower",
          "counter": "FLOOD",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "assertType(\"Forbidding Watchtower\", CardType.LAND, SubType.ISLAND)"
        },
        {
          "op": "assertPowerToughness",
          "player": 1,
          "name": "Forbidding Watchtower",
          "power": 1,
          "toughness": 5
        }
      ]
    },
    {
      "name": "testBloodMoonBeforeUrborg",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Blood Moon",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Urborg, Tomb of Yawgmoth",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Canopy Vista",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Blood Moon"
        },
        {
          "op": "playLand",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Urborg, Tomb of Yawgmoth"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Blood Moon",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Urborg, Tomb of Yawgmoth",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "assertType(canopyvista, CardType.LAND, SubType.MOUNTAIN)"
        },
        {
          "op": "unsupported",
          "source": "assertNotSubtype(canopyvista, SubType.ISLAND)"
        },
        {
          "op": "unsupported",
          "source": "assertNotSubtype(canopyvista, SubType.SWAMP)"
        },
        {
          "op": "unsupported",
          "source": "assertType(urborgtoy, CardType.LAND, SubType.MOUNTAIN)"
        },
        {
          "op": "unsupported",
          "source": "assertNotSubtype(urborgtoy, SubType.SWAMP)"
        },
        {
          "op": "assertManaOptions",
          "player": 1,
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "options": [
            "{R}"
          ],
          "exact": true
        }
      ]
    },
    {
      "name": "testBloodMoonAfterUrborg",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Blood Moon",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Urborg, Tomb of Yawgmoth",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Canopy Vista",
          "count": 1
        },
        {
          "op": "playLand",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Urborg, Tomb of Yawgmoth"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Blood Moon"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Blood Moon",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Urborg, Tomb of Yawgmoth",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "assertType(canopyvista, CardType.LAND, SubType.MOUNTAIN)"
        },
        {
          "op": "unsupported",
          "source": "assertNotSubtype(canopyvista, SubType.ISLAND)"
        },
        {
          "op": "unsupported",
          "source": "assertNotSubtype(canopyvista, SubType.SWAMP)"
        },
        {
          "op": "unsupported",
          "source": "assertType(urborgtoy, CardType.LAND, SubType.MOUNTAIN)"
        },
        {
          "op": "unsupported",
          "source": "assertNotSubtype(urborgtoy, SubType.SWAMP)"
        },
        {
          "op": "assertManaOptions",
          "player": 1,
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "options": [
            "{R}"
          ],
          "exact": true
        }
      ]
    },
    {
      "name": "testCormusBellAfterUrborg",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Urborg, Tomb of Yawgmoth",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Kormus Bell",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain@flooded",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain@untouched1",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain@untouched2",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain@untouched3",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Quicksilver Fountain",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Forest",
          "count": 3
        },
        {
          "op": "playLand",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Urborg, Tomb of Yawgmoth"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Kormus Bell"
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Quicksilver Fountain"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": {
            "groups": [
              [
                "@flooded"
              ]
            ]
          }
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Urborg, Tomb of Yawgmoth",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Kormus Bell",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Quicksilver Fountain",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Mountain",
          "count": 0
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 0,
            "alias": "@flooded"
          },
          "count": 1,
          "checks": [
            {
              "property": "cardType",
              "value": "Land",
              "expected": true
            },
            {
              "property": "cardType",
              "value": "Creature",
              "expected": false
            },
            {
              "property": "subtype",
              "value": "Island",
              "expected": true
            },
            {
              "property": "subtype",
              "value": "Mountain",
              "expected": false
            },
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
            "player": 0,
            "alias": "@untouched1"
          },
          "count": 1,
          "checks": [
            {
              "property": "cardType",
              "value": "Land",
              "expected": true
            },
            {
              "property": "cardType",
              "value": "Creature",
              "expected": true
            },
            {
              "property": "subtype",
              "value": "Island",
              "expected": false
            },
            {
              "property": "subtype",
              "value": "Mountain",
              "expected": true
            },
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
            "player": 0,
            "alias": "@untouched2"
          },
          "count": 1,
          "checks": [
            {
              "property": "cardType",
              "value": "Land",
              "expected": true
            },
            {
              "property": "cardType",
              "value": "Creature",
              "expected": true
            },
            {
              "property": "subtype",
              "value": "Island",
              "expected": false
            },
            {
              "property": "subtype",
              "value": "Mountain",
              "expected": true
            },
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
            "player": 0,
            "alias": "@untouched3"
          },
          "count": 1,
          "checks": [
            {
              "property": "cardType",
              "value": "Land",
              "expected": true
            },
            {
              "property": "cardType",
              "value": "Creature",
              "expected": true
            },
            {
              "property": "subtype",
              "value": "Island",
              "expected": false
            },
            {
              "property": "subtype",
              "value": "Mountain",
              "expected": true
            },
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
            "player": 0,
            "name": "Urborg, Tomb of Yawgmoth"
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
              "value": "Swamp",
              "expected": true
            },
            {
              "property": "subtype",
              "value": "Island",
              "expected": false
            },
            {
              "property": "subtype",
              "value": "Mountain",
              "expected": false
            }
          ]
        }
      ]
    },
    {
      "name": "testBloodSunWithUrborgtoyAndStormtideLeviathanMan",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Urborg, Tomb of Yawgmoth",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Blood Sun",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Stormtide Leviathan",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Darksteel Citadel",
          "count": 1
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 0,
            "name": "Urborg, Tomb of Yawgmoth"
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
              "value": "Island",
              "expected": true
            },
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
            "player": 0,
            "name": "Mountain"
          },
          "count": 4,
          "checks": [
            {
              "property": "cardType",
              "value": "Land",
              "expected": true
            },
            {
              "property": "subtype",
              "value": "Island",
              "expected": true
            },
            {
              "property": "subtype",
              "value": "Swamp",
              "expected": true
            }
          ]
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Darksteel Citadel",
          "count": 1
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Darksteel Citadel",
          "ability": "Indestructible",
          "expected": false
        }
      ]
    },
    {
      "name": "testOrcishFarmer",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Orcish Farmer",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Reliquary Tower",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{T}: Target",
          "target": "Plains"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{T}: Target",
          "target": "Reliquary Tower"
        },
        {
          "op": "assertSubtype",
          "turn": 1,
          "phase": "END_TURN",
          "player": 0,
          "name": "Plains",
          "subtype": "SWAMP",
          "extra": "true",
          "seatIndependent": true
        },
        {
          "op": "assertSubtype",
          "turn": 1,
          "phase": "END_TURN",
          "player": 0,
          "name": "Plains",
          "subtype": "PLAINS",
          "extra": "false",
          "seatIndependent": true
        },
        {
          "op": "assertSubtype",
          "turn": 1,
          "phase": "END_TURN",
          "player": 1,
          "name": "Reliquary Tower",
          "subtype": "SWAMP",
          "extra": "true",
          "seatIndependent": true
        },
        {
          "op": "assertSubtype",
          "turn": 2,
          "phase": "UPKEEP",
          "player": 0,
          "name": "Plains",
          "subtype": "SWAMP",
          "extra": "true",
          "seatIndependent": true
        },
        {
          "op": "assertSubtype",
          "turn": 2,
          "phase": "UPKEEP",
          "player": 0,
          "name": "Plains",
          "subtype": "PLAINS",
          "extra": "false",
          "seatIndependent": true
        },
        {
          "op": "assertSubtype",
          "turn": 2,
          "phase": "UPKEEP",
          "player": 1,
          "name": "Reliquary Tower",
          "subtype": "SWAMP",
          "extra": "false",
          "seatIndependent": true
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "UPKEEP"
        },
        {
          "op": "execute"
        },
        {
          "op": "unsupported",
          "source": "assertNotSubtype(\"Plains\", SubType.SWAMP)"
        },
        {
          "op": "unsupported",
          "source": "assertSubtype(\"Plains\", SubType.PLAINS)"
        }
      ]
    }
  ]
});
