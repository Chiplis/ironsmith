import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/continuous/LayerTests.java",
  "tests": [
    {
      "name": "testMultipleLayeredDependency",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Conspiracy",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Opalescence",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Enchanted Evening",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 5
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 5
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 5
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Glorious Anthem",
          "count": 1
        },
        {
          "op": "activateManaAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "source": "Swamp",
          "ability": "{T}: Add {B}",
          "count": 5
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Conspiracy"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Advisor"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Conspiracy",
          "count": 1
        },
        {
          "op": "activateManaAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "source": "Plains",
          "ability": "{T}: Add {W}",
          "count": 4
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Opalescence"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Opalescence",
          "count": 1
        },
        {
          "op": "activateManaAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "source": "Island",
          "ability": "{T}: Add {U}",
          "count": 5
        },
        {
          "op": "setOptionChoices",
          "player": 0,
          "kind": "hybrid_payment",
          "description": "Choose how to pay pip 2 of Enchanted Evening",
          "choices": [
            "{U} (Blue mana)"
          ]
        },
        {
          "op": "setOptionChoices",
          "player": 0,
          "kind": "hybrid_payment",
          "description": "Choose how to pay pip 3 of Enchanted Evening",
          "choices": [
            "{U} (Blue mana)"
          ]
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Enchanted Evening"
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Enchanted Evening",
          "count": 1
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Enchanted Evening",
          "count": 1
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "name": "Swamp",
            "player": 0
          },
          "count": 5,
          "checks": [
            {
              "property": "cardType",
              "value": "Land",
              "expected": true
            },
            {
              "property": "subtype",
              "value": "Advisor",
              "expected": true
            }
          ]
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Swamp",
          "power": 1,
          "toughness": 1
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "name": "Enchanted Evening",
            "player": 0
          },
          "count": 1,
          "checks": [
            {
              "property": "cardType",
              "value": "Enchantment",
              "expected": true
            },
            {
              "property": "subtype",
              "value": "Advisor",
              "expected": true
            }
          ]
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Enchanted Evening",
          "power": 6,
          "toughness": 6
        }
      ]
    },
    {
      "name": "testMycosynthLatticeAndMarchOfTheMachinesAndHumility",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Mycosynth Lattice",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "March of the Machines",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Humility",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 10
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 10
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 10
        },
        {
          "op": "castSpell",
          "name": "Humility",
          "player": 0,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertStackObject",
          "source": "Humility",
          "count": 1,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "isSpell": true
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Humility",
          "count": 1
        },
        {
          "op": "castSpell",
          "name": "March of the Machines",
          "player": 0,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertStackObject",
          "source": "March of the Machines",
          "count": 1,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "isSpell": true
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "March of the Machines",
          "count": 1
        },
        {
          "op": "castSpell",
          "name": "Mycosynth Lattice",
          "player": 0,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertStackObject",
          "source": "Mycosynth Lattice",
          "count": 1,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "isSpell": true
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Mycosynth Lattice",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Island",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Swamp",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Plains",
          "count": 0
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 0,
            "name": "Humility"
          },
          "count": 1,
          "checks": [
            {
              "property": "cardType",
              "value": "creature",
              "expected": true
            },
            {
              "property": "cardType",
              "value": "artifact",
              "expected": true
            }
          ]
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Humility",
          "power": 4,
          "toughness": 4
        },
        {
          "op": "assertAbilityKindCount",
          "player": 0,
          "name": "Humility",
          "kind": "static",
          "count": 0
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 0,
            "name": "March of the Machines"
          },
          "count": 1,
          "checks": [
            {
              "property": "cardType",
              "value": "creature",
              "expected": true
            },
            {
              "property": "cardType",
              "value": "artifact",
              "expected": true
            }
          ]
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "March of the Machines",
          "power": 4,
          "toughness": 4
        },
        {
          "op": "assertAbilityKindCount",
          "player": 0,
          "name": "March of the Machines",
          "kind": "static",
          "count": 0
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 0,
            "name": "Mycosynth Lattice"
          },
          "count": 1,
          "checks": [
            {
              "property": "cardType",
              "value": "creature",
              "expected": true
            },
            {
              "property": "cardType",
              "value": "artifact",
              "expected": true
            }
          ]
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Mycosynth Lattice",
          "power": 6,
          "toughness": 6
        },
        {
          "op": "assertAbilityKindCount",
          "player": 0,
          "name": "Mycosynth Lattice",
          "kind": "static",
          "count": 0
        }
      ]
    },
    {
      "name": "testBloodMoon_UrborgTombOfYawgmothInteraction",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Blood Moon",
          "count": 1
        },
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
          "name": "Plains",
          "count": 1
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "unsupported",
          "source": "assertType(\"Urborg, Tomb of Yawgmoth\", CardType.LAND, SubType.MOUNTAIN)"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Swamp",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Plains",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "assertType(\"Plains\", CardType.LAND, SubType.PLAINS)"
        }
      ]
    },
    {
      "name": "complexExampleFromLayersArticle",
      "operations": [
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
          "name": "Giant Growth",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Bog Wraith",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Lignify",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Figure of Destiny",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Mirrorweave",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 20
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 20
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 20
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Giant Growth",
          "target": "Grizzly Bears"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Lignify",
          "target": "Bog Wrath"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{R/W}:"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{R/W}{R/W}{R/W}:"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{R/W}{R/W}{R/W}{R/W}{R/W}{R/W}:"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Mirrorweave",
          "target": "Figure of Destiny"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Figure of Destiny",
          "count": 3
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Figure of Destiny",
          "power": 4,
          "toughness": 4
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Figure of Destiny",
          "power": 8,
          "toughness": 8
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Figure of Destiny",
          "power": 0,
          "toughness": 4
        }
      ],
      "skip": "upstream @Ignore"
    },
    {
      "name": "testUrborgWithAnimateLandAndOvinize",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Animate Land",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Ovinize",
          "count": 1
        },
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
          "name": "Forest",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 2
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Animate Land",
          "target": "Urborg, Tomb of Yawgmoth"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Ovinize",
          "target": "Urborg, Tomb of Yawgmoth"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "unsupported",
          "source": "assertType(\"Urborg, Tomb of Yawgmoth\", CardType.CREATURE, SubType.SWAMP)"
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Urborg, Tomb of Yawgmoth",
          "power": 0,
          "toughness": 1
        }
      ]
    },
    {
      "name": "testFromAnArticle",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Scourge of the Nobilis",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Inside Out",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Battlegate Mimic",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 8
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 8
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Wilderness Hypnotist",
          "count": 2
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Scourge of the Nobilis",
          "target": "Battlegate Mimic"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "ability": "{T}:",
          "target": "Battlegate Mimic"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Inside Out",
          "target": "Battlegate Mimic"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "ability": "{T}:",
          "target": "Battlegate Mimic"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Battlegate Mimic",
          "power": 4,
          "toughness": 2
        }
      ],
      "skip": "upstream @Ignore"
    },
    {
      "name": "testExampleFromReddit2021",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Life and Limb",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Humility",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Yavimaya, Cradle of Growth",
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
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Island",
          "count": 1
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "name": "Plains",
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
              "value": "Forest",
              "expected": true
            }
          ]
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "name": "Swamp",
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
              "value": "Forest",
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
              "property": "cardType",
              "value": "Creature",
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
            "name": "Yavimaya, Cradle of Growth",
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
              "value": "Forest",
              "expected": true
            }
          ]
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Plains",
          "power": 1,
          "toughness": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Swamp",
          "power": 1,
          "toughness": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 1,
          "name": "Island",
          "power": 1,
          "toughness": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Yavimaya, Cradle of Growth",
          "power": 1,
          "toughness": 1
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "name": "Plains",
            "player": 0
          },
          "count": 1,
          "checks": [
            {
              "property": "color",
              "value": "Green",
              "expected": true
            }
          ]
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "name": "Swamp",
            "player": 0
          },
          "count": 1,
          "checks": [
            {
              "property": "color",
              "value": "Green",
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
              "property": "color",
              "value": "Green",
              "expected": true
            }
          ]
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "name": "Yavimaya, Cradle of Growth",
            "player": 0
          },
          "count": 1,
          "checks": [
            {
              "property": "color",
              "value": "Green",
              "expected": true
            }
          ]
        }
      ]
    }
  ]
});
