import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/copy/EssenceOfTheWildCopyTest.java",
  "tests": [
    {
      "name": "test_CopyCreature",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Essence of the Wild@original",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Silvercoat Lion@copy",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Silvercoat Lion"
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
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Essence of the Wild",
          "count": 2
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "@copy",
          "power": 6,
          "toughness": 6
        }
      ]
    },
    {
      "name": "test_CopyCreatureByCopied",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Essence of the Wild@original",
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
          "zone": "HAND",
          "name": "Silvercoat Lion@first",
          "count": 1,
          "player": 0
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "name": "Silvercoat Lion@second",
          "count": 1,
          "player": 0
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Silvercoat Lion",
          "objectAlias": "@first"
        },
        {
          "op": "setOptionChoices",
          "description": "Choose which replacement effect to apply",
          "choices": [
            {
              "description": "Apply Essence of the Wild copy effect",
              "object": "@first"
            }
          ],
          "kind": "replacement_effect",
          "player": 0
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Silvercoat Lion",
          "objectAlias": "@second"
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
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Essence of the Wild",
          "count": 3
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 0,
            "alias": "@first"
          },
          "count": 1,
          "checks": [
            {
              "property": "name",
              "value": null,
              "expected": "Essence of the Wild"
            },
            {
              "property": "power",
              "value": null,
              "expected": 6
            },
            {
              "property": "toughness",
              "value": null,
              "expected": 6
            }
          ]
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 0,
            "alias": "@second"
          },
          "count": 1,
          "checks": [
            {
              "property": "name",
              "value": null,
              "expected": "Essence of the Wild"
            },
            {
              "property": "power",
              "value": null,
              "expected": 6
            },
            {
              "property": "toughness",
              "value": null,
              "expected": 6
            }
          ]
        }
      ],
      "deferredPriorityAnalysis": true
    },
    {
      "name": "test_CopyManyTokens",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Essence of the Wild",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "March of the Multitudes",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 6
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 2
        },
        {
          "op": "activateManaAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "source": "Forest",
          "ability": "{G}",
          "count": 6
        },
        {
          "op": "activateManaAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "source": "Plains",
          "ability": "{W}",
          "count": 2
        },
        {
          "op": "assertManaPool",
          "player": 0,
          "colors": "WUBRGC",
          "amount": 8
        },
        {
          "op": "assertChoiceMenu",
          "description": "Choose value for X",
          "player": 0,
          "kind": "number",
          "value": 5
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "March of the Multitudes",
          "holdStack": true
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "March of the Multitudes",
          "count": 1,
          "isSpell": true,
          "xValue": 5
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
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
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Soldier Token",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Essence of the Wild",
          "count": 6
        },
        {
          "op": "assertTokenCount",
          "player": 0,
          "name": "Essence of the Wild",
          "count": 5
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 0,
            "name": "Essence of the Wild"
          },
          "count": 6,
          "checks": [
            {
              "property": "power",
              "value": null,
              "expected": 6
            },
            {
              "property": "toughness",
              "value": null,
              "expected": 6
            }
          ]
        }
      ],
      "deferredPriorityAnalysis": true
    },
    {
      "name": "test_CopyCreatureWithContinuousEffect",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Essence of the Wild@original",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Liliana, Death Wielder",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Silvercoat Lion@copy",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "+2:",
          "target": {
            "groups": [
              [
                "@original"
              ]
            ]
          },
          "source": "Liliana, Death Wielder"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Silvercoat Lion"
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
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Essence of the Wild",
          "count": 2
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "@original",
          "power": 5,
          "toughness": 5
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "@copy",
          "power": 6,
          "toughness": 6
        }
      ]
    },
    {
      "name": "testCopyFromTheCopy",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Essence of the Wild",
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
          "zone": "HAND",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Terror",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Swamp",
          "count": 2
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Silvercoat Lion"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "BEGIN_COMBAT",
          "player": 1,
          "name": "Terror",
          "target": "Essence of the Wild[no copy]"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Silvercoat Lion"
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
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Terror",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Essence of the Wild",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Essence of the Wild",
          "count": 2
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Essence of the Wild",
          "power": 6,
          "toughness": 6
        }
      ]
    }
  ]
});
