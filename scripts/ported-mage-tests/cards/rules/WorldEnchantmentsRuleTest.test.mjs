import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/rules/WorldEnchantmentsRuleTest.java",
  "tests": [
    {
      "name": "TestTwoWorldEnchantments",
      "operations": [
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
          "player": 0,
          "name": "Plains",
          "count": 5
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Nether Void",
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
          "player": 3,
          "name": "Swamp",
          "count": 7
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 3,
          "name": "Nether Void",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Nether Void"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Nether Void",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Silvercoat Lion"
        },
        {
          "op": "setOptionChoices",
          "player": 0,
          "kind": "boolean",
          "description": "{3} to prevent effect?",
          "choices": [
            "Yes"
          ]
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 3,
          "name": "Nether Void"
        },
        {
          "op": "setOptionChoices",
          "player": 3,
          "kind": "boolean",
          "description": "{3} to prevent effect?",
          "choices": [
            "Yes"
          ]
        },
        {
          "op": "waitStackResolved",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Nether Void",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "name": "Nether Void",
          "count": 1,
          "player": 3
        }
      ],
      "turnOrder": [
        0,
        3,
        2,
        1
      ],
      "rangeOfInfluence": 1
    },
    {
      "name": "TestTwoWorldEnchantmentsNotInRangeOfInfluence",
      "operations": [
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
          "player": 0,
          "name": "Plains",
          "count": 5
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Nether Void",
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
          "player": 2,
          "name": "Swamp",
          "count": 7
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 2,
          "name": "Nether Void",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Nether Void"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Nether Void",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Silvercoat Lion"
        },
        {
          "op": "setOptionChoices",
          "player": 0,
          "kind": "boolean",
          "description": "{3} to prevent effect?",
          "choices": [
            "Yes"
          ]
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "castSpell",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 2,
          "name": "Nether Void"
        },
        {
          "op": "waitStackResolved",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Nether Void",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "name": "Nether Void",
          "count": 1,
          "player": 2
        }
      ],
      "turnOrder": [
        0,
        3,
        2,
        1
      ],
      "rangeOfInfluence": 1
    },
    {
      "name": "TestTwoWorldEnchantmentsFromTriggers",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Oblivion Ring",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Concordant Crossroads@world.a",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 3
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
          "player": 3,
          "name": "Oblivion Ring",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 3,
          "name": "Anarchy",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 3,
          "name": "Concordant Crossroads@world.d",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 3,
          "name": "Plains",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 3,
          "name": "Mountain",
          "count": 6
        },
        {
          "op": "activateManaAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "source": "Forest",
          "ability": "{T}: Add {G}",
          "count": 2
        },
        {
          "op": "activateManaAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "source": "Plains",
          "ability": "{T}: Add {W}",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Oblivion Ring"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "@world.d"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Concordant Crossroads",
          "resolve": true
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 3,
          "name": "Oblivion Ring"
        },
        {
          "op": "addTarget",
          "player": 3,
          "target": "@world.a"
        },
        {
          "op": "waitStackResolved",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "POSTCOMBAT_MAIN",
          "player": 3,
          "name": "Anarchy"
        },
        {
          "op": "waitStackResolved",
          "turn": 2,
          "phase": "POSTCOMBAT_MAIN",
          "resolutions": 1
        },
        {
          "op": "assertStackObject",
          "turn": 2,
          "phase": "POSTCOMBAT_MAIN",
          "source": "Oblivion Ring",
          "count": 2,
          "abilityKind": "Triggered"
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "name": "Concordant Crossroads",
          "count": 0,
          "player": 3
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Concordant Crossroads",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Oblivion Ring",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Concordant Crossroads",
          "count": 0
        },
        {
          "op": "assertGraveyardCount",
          "name": "Oblivion Ring",
          "count": 1,
          "player": 3
        },
        {
          "op": "assertGraveyardCount",
          "name": "Concordant Crossroads",
          "count": 1,
          "player": 3
        },
        {
          "op": "assertGraveyardCount",
          "name": "Anarchy",
          "count": 1,
          "player": 3
        }
      ],
      "turnOrder": [
        0,
        3,
        2,
        1
      ],
      "rangeOfInfluence": 1
    },
    {
      "name": "TestTwoWorldEnchantmentsWithSameOrder",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Charmed Griffin",
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
          "player": 3,
          "name": "Concordant Crossroads",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Concordant Crossroads",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Charmed Griffin"
        },
        {
          "op": "setChoice",
          "player": 3,
          "value": true
        },
        {
          "op": "setChoice",
          "player": 3,
          "value": "Concordant Crossroads"
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": true
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": "Concordant Crossroads"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "concede",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 2
        },
        {
          "op": "assertPermanentCount",
          "turn": 1,
          "phase": "BEGIN_COMBAT",
          "seatIndependent": true,
          "player": 3,
          "name": "Concordant Crossroads",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "turn": 1,
          "phase": "BEGIN_COMBAT",
          "seatIndependent": true,
          "player": 1,
          "name": "Concordant Crossroads",
          "count": 1
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
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Charmed Griffin",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 3,
          "name": "Concordant Crossroads",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Concordant Crossroads",
          "count": 0
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Concordant Crossroads",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 3,
          "name": "Concordant Crossroads",
          "count": 1
        }
      ],
      "turnOrder": [
        0,
        3,
        2,
        1
      ],
      "rangeOfInfluence": 1
    }
  ]
});
