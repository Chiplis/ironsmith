import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/continuous/SubTypeChangingEffectsTest.java",
  "tests": [
    {
      "name": "testConspiracyGiveType",
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
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 5
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 1,
          "name": "Silvercoat Lion",
          "count": 1
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
          "value": "Orc"
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
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Conspiracy",
          "count": 1
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "library",
            "player": 0,
            "cardType": "Creature"
          },
          "checks": [
            {
              "property": "subtype",
              "value": "ORC",
              "expected": true
            },
            {
              "property": "subtype",
              "value": "CAT",
              "expected": false
            }
          ]
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "library",
            "player": 1,
            "cardType": "Creature"
          },
          "checks": [
            {
              "property": "subtype",
              "value": "ORC",
              "expected": false
            },
            {
              "property": "subtype",
              "value": "CAT",
              "expected": true
            }
          ]
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "hand",
            "player": 0,
            "cardType": "Creature"
          },
          "checks": [
            {
              "property": "subtype",
              "value": "ORC",
              "expected": true
            }
          ]
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "hand",
            "player": 1,
            "cardType": "Creature"
          },
          "checks": [
            {
              "property": "subtype",
              "value": "ORC",
              "expected": false
            }
          ]
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "graveyard",
            "player": 0,
            "cardType": "Creature"
          },
          "checks": [
            {
              "property": "subtype",
              "value": "ORC",
              "expected": true
            }
          ]
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "graveyard",
            "player": 1,
            "cardType": "Creature"
          },
          "checks": [
            {
              "property": "subtype",
              "value": "ORC",
              "expected": false
            }
          ]
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 0,
            "name": "Silvercoat Lion"
          },
          "count": 1,
          "checks": [
            {
              "property": "subtype",
              "value": "Cat",
              "expected": false
            },
            {
              "property": "subtype",
              "value": "Orc",
              "expected": true
            }
          ]
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 1,
            "name": "Silvercoat Lion"
          },
          "count": 1,
          "checks": [
            {
              "property": "subtype",
              "value": "Cat",
              "expected": true
            },
            {
              "property": "subtype",
              "value": "Orc",
              "expected": false
            }
          ]
        }
      ]
    },
    {
      "name": "testConspiracyIsRestCorrectly",
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
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 5
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Disenchant",
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
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 1,
          "name": "Silvercoat Lion",
          "count": 1
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
          "value": "Orc"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 1,
          "name": "Disenchant",
          "target": "Conspiracy"
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
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Conspiracy",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Disenchant",
          "count": 1
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "library",
            "player": 0,
            "cardType": "Creature"
          },
          "checks": [
            {
              "property": "subtype",
              "value": "ORC",
              "expected": false
            },
            {
              "property": "subtype",
              "value": "CAT",
              "expected": true
            }
          ]
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "hand",
            "player": 0,
            "cardType": "Creature"
          },
          "checks": [
            {
              "property": "subtype",
              "value": "ORC",
              "expected": false
            },
            {
              "property": "subtype",
              "value": "CAT",
              "expected": true
            }
          ]
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "graveyard",
            "player": 0,
            "cardType": "Creature"
          },
          "checks": [
            {
              "property": "subtype",
              "value": "ORC",
              "expected": false
            },
            {
              "property": "subtype",
              "value": "CAT",
              "expected": true
            }
          ]
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 0,
            "name": "Silvercoat Lion"
          },
          "count": 1,
          "checks": [
            {
              "property": "subtype",
              "value": "Cat",
              "expected": true
            },
            {
              "property": "subtype",
              "value": "Orc",
              "expected": false
            }
          ]
        }
      ]
    },
    {
      "name": "testArcaneAdaptationGiveType",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Arcane Adaptation",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 1,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Arcane Adaptation"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Orc"
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
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Arcane Adaptation",
          "count": 1
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "library",
            "player": 0,
            "cardType": "Creature"
          },
          "checks": [
            {
              "property": "subtype",
              "value": "ORC",
              "expected": true
            },
            {
              "property": "subtype",
              "value": "CAT",
              "expected": true
            }
          ]
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "library",
            "player": 1,
            "cardType": "Creature"
          },
          "checks": [
            {
              "property": "subtype",
              "value": "ORC",
              "expected": false
            },
            {
              "property": "subtype",
              "value": "CAT",
              "expected": true
            }
          ]
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "hand",
            "player": 0,
            "cardType": "Creature"
          },
          "checks": [
            {
              "property": "subtype",
              "value": "ORC",
              "expected": true
            },
            {
              "property": "subtype",
              "value": "CAT",
              "expected": true
            }
          ]
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "hand",
            "player": 1,
            "cardType": "Creature"
          },
          "checks": [
            {
              "property": "subtype",
              "value": "ORC",
              "expected": false
            },
            {
              "property": "subtype",
              "value": "CAT",
              "expected": true
            }
          ]
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "graveyard",
            "player": 0,
            "cardType": "Creature"
          },
          "checks": [
            {
              "property": "subtype",
              "value": "ORC",
              "expected": true
            },
            {
              "property": "subtype",
              "value": "CAT",
              "expected": true
            }
          ]
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "graveyard",
            "player": 1,
            "cardType": "Creature"
          },
          "checks": [
            {
              "property": "subtype",
              "value": "ORC",
              "expected": false
            },
            {
              "property": "subtype",
              "value": "CAT",
              "expected": true
            }
          ]
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 0,
            "name": "Silvercoat Lion"
          },
          "count": 1,
          "checks": [
            {
              "property": "subtype",
              "value": "Cat",
              "expected": true
            },
            {
              "property": "subtype",
              "value": "Orc",
              "expected": true
            }
          ]
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 1,
            "name": "Silvercoat Lion"
          },
          "count": 1,
          "checks": [
            {
              "property": "subtype",
              "value": "Cat",
              "expected": true
            },
            {
              "property": "subtype",
              "value": "Orc",
              "expected": false
            }
          ]
        }
      ]
    },
    {
      "name": "testArcaneAdaptationIsRestCorrectly",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Arcane Adaptation",
          "count": 1
        },
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
          "name": "Forest",
          "count": 8
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
          "zone": "HAND",
          "player": 0,
          "name": "Advent of the Wurm",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Beast Attack",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Disenchant",
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
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 1,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "UPKEEP",
          "player": 0,
          "name": "Advent of the Wurm"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Arcane Adaptation"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Orc"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "BEGIN_COMBAT",
          "player": 0,
          "name": "Beast Attack"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 1,
          "name": "Disenchant",
          "target": "Arcane Adaptation"
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
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Arcane Adaptation",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Advent of the Wurm",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Beast Attack",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Disenchant",
          "count": 1
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "library",
            "player": 0,
            "cardType": "Creature"
          },
          "checks": [
            {
              "property": "subtype",
              "value": "ORC",
              "expected": false
            },
            {
              "property": "subtype",
              "value": "CAT",
              "expected": true
            }
          ]
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "hand",
            "player": 0,
            "cardType": "Creature"
          },
          "checks": [
            {
              "property": "subtype",
              "value": "ORC",
              "expected": false
            },
            {
              "property": "subtype",
              "value": "CAT",
              "expected": true
            }
          ]
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "graveyard",
            "player": 0,
            "cardType": "Creature"
          },
          "checks": [
            {
              "property": "subtype",
              "value": "ORC",
              "expected": false
            },
            {
              "property": "subtype",
              "value": "CAT",
              "expected": true
            }
          ]
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 0,
            "name": "Silvercoat Lion"
          },
          "count": 1,
          "checks": [
            {
              "property": "subtype",
              "value": "Cat",
              "expected": true
            },
            {
              "property": "subtype",
              "value": "Orc",
              "expected": false
            }
          ]
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 0,
            "name": "Beast Token"
          },
          "count": 1,
          "checks": [
            {
              "property": "subtype",
              "value": "Beast",
              "expected": true
            },
            {
              "property": "subtype",
              "value": "Orc",
              "expected": false
            }
          ]
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 0,
            "name": "Wurm Token"
          },
          "count": 1,
          "checks": [
            {
              "property": "subtype",
              "value": "Wurm",
              "expected": true
            },
            {
              "property": "subtype",
              "value": "Orc",
              "expected": false
            }
          ]
        }
      ]
    },
    {
      "name": "testKeepOtherTypes",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Volcanic Island",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Gingerbrute",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Dragonshift",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Dragonshift",
          "target": "Gingerbrute"
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
          "op": "unsupported",
          "source": "assertType(\"Gingerbrute\", CardType.ARTIFACT, SubType.FOOD)"
        },
        {
          "op": "unsupported",
          "source": "assertType(\"Gingerbrute\", CardType.CREATURE, SubType.DRAGON)"
        },
        {
          "op": "unsupported",
          "source": "assertNotSubtype(\"Gingerbrute\", SubType.GOLEM)"
        }
      ]
    },
    {
      "name": "testKeepOtherTypes2",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Volcanic Island",
          "count": 7
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Gingerbrute",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Dragonshift",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Dragonshift with overload"
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
          "op": "unsupported",
          "source": "assertType(\"Gingerbrute\", CardType.ARTIFACT, SubType.FOOD)"
        },
        {
          "op": "unsupported",
          "source": "assertType(\"Gingerbrute\", CardType.CREATURE, SubType.DRAGON)"
        },
        {
          "op": "unsupported",
          "source": "assertNotSubtype(\"Gingerbrute\", SubType.GOLEM)"
        }
      ]
    },
    {
      "name": "testMaskwoodNexus",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Maskwood Nexus",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 1,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Maskwood Nexus"
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
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Maskwood Nexus",
          "count": 1
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "library",
            "player": 0,
            "cardType": "Creature"
          },
          "checks": [
            {
              "property": "subtype",
              "value": "ORC",
              "expected": true
            },
            {
              "property": "subtype",
              "value": "CAT",
              "expected": true
            }
          ]
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "library",
            "player": 1,
            "cardType": "Creature"
          },
          "checks": [
            {
              "property": "subtype",
              "value": "ORC",
              "expected": false
            },
            {
              "property": "subtype",
              "value": "CAT",
              "expected": true
            }
          ]
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "hand",
            "player": 0,
            "cardType": "Creature"
          },
          "checks": [
            {
              "property": "subtype",
              "value": "ORC",
              "expected": true
            }
          ]
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "hand",
            "player": 1,
            "cardType": "Creature"
          },
          "checks": [
            {
              "property": "subtype",
              "value": "ORC",
              "expected": false
            }
          ]
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "graveyard",
            "player": 0,
            "cardType": "Creature"
          },
          "checks": [
            {
              "property": "subtype",
              "value": "ORC",
              "expected": true
            }
          ]
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "graveyard",
            "player": 1,
            "cardType": "Creature"
          },
          "checks": [
            {
              "property": "subtype",
              "value": "ORC",
              "expected": false
            }
          ]
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 0,
            "name": "Silvercoat Lion"
          },
          "count": 1,
          "checks": [
            {
              "property": "subtype",
              "value": "Cat",
              "expected": true
            },
            {
              "property": "subtype",
              "value": "Orc",
              "expected": true
            }
          ]
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 1,
            "name": "Silvercoat Lion"
          },
          "count": 1,
          "checks": [
            {
              "property": "subtype",
              "value": "Cat",
              "expected": true
            },
            {
              "property": "subtype",
              "value": "Orc",
              "expected": false
            }
          ]
        }
      ]
    },
    {
      "name": "testMaskwoodNexus2",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Maskwood Nexus",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Shatter",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 6
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 1,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Maskwood Nexus"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "BEGIN_COMBAT",
          "player": 0,
          "name": "Shatter",
          "target": "Maskwood Nexus"
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
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Maskwood Nexus",
          "count": 1
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "library",
            "player": 0,
            "cardType": "Creature"
          },
          "checks": [
            {
              "property": "subtype",
              "value": "ORC",
              "expected": false
            },
            {
              "property": "subtype",
              "value": "CAT",
              "expected": true
            }
          ]
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "library",
            "player": 1,
            "cardType": "Creature"
          },
          "checks": [
            {
              "property": "subtype",
              "value": "ORC",
              "expected": false
            },
            {
              "property": "subtype",
              "value": "CAT",
              "expected": true
            }
          ]
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "hand",
            "player": 0,
            "cardType": "Creature"
          },
          "checks": [
            {
              "property": "subtype",
              "value": "ORC",
              "expected": false
            }
          ]
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "hand",
            "player": 1,
            "cardType": "Creature"
          },
          "checks": [
            {
              "property": "subtype",
              "value": "ORC",
              "expected": false
            }
          ]
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "graveyard",
            "player": 0,
            "cardType": "Creature"
          },
          "checks": [
            {
              "property": "subtype",
              "value": "ORC",
              "expected": false
            }
          ]
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "graveyard",
            "player": 1,
            "cardType": "Creature"
          },
          "checks": [
            {
              "property": "subtype",
              "value": "ORC",
              "expected": false
            }
          ]
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 0,
            "name": "Silvercoat Lion"
          },
          "count": 1,
          "checks": [
            {
              "property": "subtype",
              "value": "Cat",
              "expected": true
            },
            {
              "property": "subtype",
              "value": "Orc",
              "expected": false
            }
          ]
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 1,
            "name": "Silvercoat Lion"
          },
          "count": 1,
          "checks": [
            {
              "property": "subtype",
              "value": "Cat",
              "expected": true
            },
            {
              "property": "subtype",
              "value": "Orc",
              "expected": false
            }
          ]
        }
      ]
    },
    {
      "name": "testMaskwoodNexus3",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Sarkhan the Masterless",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Silvercoat Lion",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Maskwood Nexus",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Bonebreaker Giant",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Bonebreaker Giant",
          "defender": 1
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
          "op": "unsupported",
          "source": "assertDamageReceived(playerA, \"Bonebreaker Giant\", 3)"
        }
      ]
    }
  ]
});
