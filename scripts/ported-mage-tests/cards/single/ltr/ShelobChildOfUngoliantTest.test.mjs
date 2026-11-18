import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/ltr/ShelobChildOfUngoliantTest.java",
  "tests": [
    {
      "name": "test_Shelob_NoSpiderDamage",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Shelob, Child of Ungoliant",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Day of Judgment",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Grizzly Bears",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Day of Judgment"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Grizzly Bears",
          "count": 0
        }
      ]
    },
    {
      "name": "test_Shelob_DieTogether",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Shelob, Child of Ungoliant",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Prey Upon",
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
          "player": 1,
          "name": "Feral Abomination",
          "count": 1
        },
        {
          "op": "castSpell",
          "name": "Prey Upon",
          "player": 0,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "target": {
            "groups": [
              [
                "Shelob, Child of Ungoliant"
              ],
              [
                "Feral Abomination"
              ]
            ]
          }
        },
        {
          "op": "assertStackObject",
          "source": "Prey Upon",
          "count": 1,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "targetCount": 2,
          "objectTarget": "Feral Abomination"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "resolutions": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Shelob, Child of Ungoliant",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Feral Abomination",
          "count": 1
        },
        {
          "op": "assertStackObject",
          "source": "Shelob, Child of Ungoliant",
          "count": 1,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "abilityKind": "Triggered"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Feral Abomination",
          "count": 1
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 0,
            "name": "Feral Abomination",
            "token": true
          },
          "count": 1,
          "checks": []
        },
        {
          "op": "assertType",
          "name": "Feral Abomination",
          "player": 0,
          "cardType": "ARTIFACT"
        },
        {
          "op": "assertType",
          "name": "Feral Abomination",
          "player": 0,
          "cardType": "CREATURE",
          "extra": "false"
        },
        {
          "op": "assertSubtype",
          "name": "Feral Abomination",
          "player": 0,
          "subtype": "Food",
          "extra": "true"
        },
        {
          "op": "assertSubtype",
          "name": "Feral Abomination",
          "player": 0,
          "subtype": "Thrull",
          "extra": "false"
        }
      ]
    },
    {
      "name": "test_Shelob_TokenKeepsArtifactSubtypes",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Shelob, Child of Ungoliant",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Prey Upon",
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
          "player": 1,
          "name": "Lizard Blades",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Prey Upon",
          "target": {
            "groups": [
              [
                "Shelob, Child of Ungoliant"
              ],
              [
                "Lizard Blades"
              ]
            ]
          }
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "execute"
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Lizard Blades",
          "count": 1
        },
        {
          "op": "assertTokenCount",
          "player": 0,
          "name": "Lizard Blades",
          "count": 1
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "name": "Lizard Blades",
            "player": 0
          },
          "count": 1,
          "checks": [
            {
              "property": "cardType",
              "value": "Artifact",
              "expected": true
            },
            {
              "property": "cardType",
              "value": "Creature",
              "expected": false
            },
            {
              "property": "subtype",
              "value": "Food",
              "expected": true
            },
            {
              "property": "subtype",
              "value": "Lizard",
              "expected": false
            },
            {
              "property": "subtype",
              "value": "Equipment",
              "expected": false
            }
          ]
        }
      ]
    },
    {
      "name": "test_Shelob_TokenIsCopySoHasText",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Shelob, Child of Ungoliant",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Prey Upon",
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
          "player": 1,
          "name": "Angel of Mercy",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Prey Upon",
          "target": "Shelob, Child of Ungoliant^Angel of Mercy"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Angel of Mercy",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "assertTokenCount(playerA, \"Angel of Mercy\", 1)"
        },
        {
          "op": "unsupported",
          "source": "assertType(\"Angel of Mercy\", CardType.ARTIFACT, true)"
        },
        {
          "op": "unsupported",
          "source": "assertSubtype(\"Angel of Mercy\", SubType.FOOD)"
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 23
        }
      ]
    },
    {
      "name": "test_Shelob_GristDoesNotDieAsCreature",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Shelob, Child of Ungoliant",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Bite Down",
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
          "player": 1,
          "name": "Grist, the Hunger Tide",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Bite Down",
          "target": "Shelob, Child of Ungoliant^Grist, the Hunger Tide"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Grist, the Hunger Tide",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Grist, the Hunger Tide",
          "count": 0
        }
      ]
    },
    {
      "name": "test_Shelob_AnimatedPermamentIsCopied",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Shelob, Child of Ungoliant",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Bite Down",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Ensoul Artifact",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Tropical Island",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Aradara Express",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Ensoul Artifact",
          "target": "Aradara Express"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "name": "Aradara Express",
            "player": 1
          },
          "count": 1,
          "checks": [
            {
              "property": "cardType",
              "value": "Creature",
              "expected": true
            }
          ]
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Bite Down",
          "target": {
            "groups": [
              [
                "Shelob, Child of Ungoliant"
              ],
              [
                "Aradara Express"
              ]
            ]
          }
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "execute"
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Aradara Express",
          "count": 1
        },
        {
          "op": "assertTokenCount",
          "player": 0,
          "name": "Aradara Express",
          "count": 1
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "name": "Aradara Express",
            "player": 0
          },
          "count": 1,
          "checks": [
            {
              "property": "cardType",
              "value": "Artifact",
              "expected": true
            },
            {
              "property": "cardType",
              "value": "Creature",
              "expected": false
            },
            {
              "property": "subtype",
              "value": "Food",
              "expected": true
            },
            {
              "property": "subtype",
              "value": "Lizard",
              "expected": false
            },
            {
              "property": "subtype",
              "value": "Vehicle",
              "expected": false
            }
          ]
        }
      ]
    },
    {
      "name": "test_Shelob_TokenIsCopied",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Shelob, Child of Ungoliant",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Bite Down",
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
          "zone": "HAND",
          "player": 1,
          "name": "Errand of Duty",
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
          "phase": "UPKEEP",
          "player": 1,
          "name": "Errand of Duty"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Bite Down",
          "target": "Shelob, Child of Ungoliant"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Knight Token"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Knight Token",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "assertTokenCount(playerA, \"Knight Token\", 1)"
        },
        {
          "op": "unsupported",
          "source": "assertType(\"Knight Token\", CardType.ARTIFACT, true)"
        },
        {
          "op": "unsupported",
          "source": "assertSubtype(\"Knight Token\", SubType.FOOD)"
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Knight Token",
          "ability": "Banding",
          "expected": true
        }
      ]
    },
    {
      "name": "test_Shelob_EarlySpiderDamageCounts",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Shelob, Child of Ungoliant",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Giant Spider",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Bite Down",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Doom Blade",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Bayou",
          "count": 10
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Colossapede",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Bite Down",
          "target": "Giant Spider^Colossapede"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Shelob, Child of Ungoliant"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Doom Blade",
          "target": "Colossapede"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Colossapede",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "assertTokenCount(playerA, \"Colossapede\", 1)"
        },
        {
          "op": "unsupported",
          "source": "assertType(\"Colossapede\", CardType.ARTIFACT, true)"
        },
        {
          "op": "unsupported",
          "source": "assertSubtype(\"Colossapede\", SubType.FOOD)"
        }
      ]
    }
  ]
});
