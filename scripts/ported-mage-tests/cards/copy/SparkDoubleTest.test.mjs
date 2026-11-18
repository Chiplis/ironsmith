import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/copy/SparkDoubleTest.java",
  "tests": [
    {
      "name": "test_CopyCreatureAndGetOneCounter",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Abbey Griffin",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Spark Double@spark",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 4
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Spark Double"
        },
        {
          "op": "setOptionChoices",
          "player": 0,
          "kind": "options",
          "description": "Choose what to copy as this permanent enters",
          "choices": [
            "Enter as a copy of Abbey Griffin"
          ]
        },
        {
          "op": "setStrictChooseMode",
          "value": true
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
          "name": "Abbey Griffin",
          "count": 2
        },
        {
          "op": "assertCounterCount",
          "name": "@spark",
          "player": 0,
          "counter": "P1P1",
          "count": 1
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 0,
            "alias": "@spark"
          },
          "count": 1,
          "checks": [
            {
              "property": "power",
              "expected": 3
            },
            {
              "property": "toughness",
              "expected": 3
            }
          ]
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "@spark",
          "ability": "Vigilance",
          "expected": true
        }
      ]
    },
    {
      "name": "test_CopyPlaneswalkerWithoutLegendaryWithOneCounter",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Ajani, the Greathearted",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Spark Double@spark",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 4
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Spark Double"
        },
        {
          "op": "setOptionChoices",
          "player": 0,
          "kind": "options",
          "description": "Choose what to copy as this permanent enters",
          "choices": [
            "Enter as a copy of Ajani, the Greathearted"
          ]
        },
        {
          "op": "setStrictChooseMode",
          "value": true
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
          "name": "Ajani, the Greathearted",
          "count": 2
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 0,
            "alias": "@spark"
          },
          "count": 1,
          "checks": [
            {
              "property": "calculatedName",
              "expected": "Ajani, the Greathearted"
            },
            {
              "property": "supertype",
              "value": "Legendary",
              "expected": false
            }
          ]
        },
        {
          "op": "assertCounterCount",
          "name": "@spark",
          "player": 0,
          "counter": "LOYALTY",
          "count": 6
        }
      ]
    },
    {
      "name": "test_CopyCreatureAndGetDoubleCounter",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Abbey Griffin",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Doubling Season",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Spark Double@spark",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 4
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Spark Double"
        },
        {
          "op": "setOptionChoices",
          "player": 0,
          "kind": "options",
          "description": "Choose what to copy as this permanent enters",
          "choices": [
            "Enter as a copy of Abbey Griffin"
          ]
        },
        {
          "op": "setStrictChooseMode",
          "value": true
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
          "name": "Abbey Griffin",
          "count": 2
        },
        {
          "op": "assertCounterCount",
          "name": "@spark",
          "player": 0,
          "counter": "P1P1",
          "count": 2
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 0,
            "alias": "@spark"
          },
          "count": 1,
          "checks": [
            {
              "property": "power",
              "expected": 4
            },
            {
              "property": "toughness",
              "expected": 4
            }
          ]
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "@spark",
          "ability": "Vigilance",
          "expected": true
        }
      ]
    },
    {
      "name": "test_CopyPlaneswalkerWithCreatureActivated",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Gideon, Ally of Zendikar",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Spark Double@spark",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 4
        },
        {
          "op": "assertType",
          "turn": 1,
          "phase": "UPKEEP",
          "player": 0,
          "name": "Gideon, Ally of Zendikar",
          "cardType": "CREATURE",
          "extra": "false"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "source": "Gideon, Ally of Zendikar",
          "ability": "+1:"
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "count": 1,
          "source": "Gideon, Ally of Zendikar",
          "abilityKind": "Activated"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "resolutions": 1
        },
        {
          "op": "assertType",
          "turn": 1,
          "phase": "BEGIN_COMBAT",
          "player": 0,
          "name": "Gideon, Ally of Zendikar",
          "cardType": "CREATURE",
          "extra": "true"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Spark Double"
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "count": 1,
          "text": "Cast Spark Double"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Gideon, Ally of Zendikar"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "resolutions": 1
        },
        {
          "op": "assertPermanentCount",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Gideon, Ally of Zendikar",
          "count": 2,
          "seatIndependent": true
        },
        {
          "op": "assertEachObject",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "selector": {
            "zone": "battlefield",
            "player": 0,
            "name": "Gideon, Ally of Zendikar",
            "cardType": "Planeswalker",
            "excludeCardType": "Creature"
          },
          "count": 1,
          "checks": [
            {
              "property": "supertype",
              "value": "Legendary",
              "expected": false
            },
            {
              "property": "counter",
              "value": "LOYALTY",
              "expected": 5
            },
            {
              "property": "counter",
              "value": "P1P1",
              "expected": 0
            }
          ]
        },
        {
          "op": "assertEachObject",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "selector": {
            "zone": "battlefield",
            "player": 0,
            "name": "Gideon, Ally of Zendikar",
            "cardType": "Creature"
          },
          "count": 1,
          "checks": [
            {
              "property": "supertype",
              "value": "Legendary",
              "expected": true
            }
          ]
        },
        {
          "op": "assertCounterCount",
          "name": "@spark",
          "player": 0,
          "counter": "LOYALTY",
          "count": 5
        },
        {
          "op": "assertCounterCount",
          "name": "@spark",
          "player": 0,
          "counter": "P1P1",
          "count": 0
        }
      ]
    },
    {
      "name": "test_CopyPlaneswalkerWithCreatureTypeChangedEffect",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Gideon Blackblade",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Spark Double",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 4
        },
        {
          "op": "assertType",
          "turn": 1,
          "phase": "UPKEEP",
          "player": 0,
          "name": "Gideon Blackblade",
          "cardType": "CREATURE",
          "extra": "true"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Spark Double"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Gideon Blackblade"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "END_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Gideon Blackblade",
          "count": 2
        }
      ],
      "skip": "upstream @Ignore"
    },
    {
      "name": "test_CopyOfSparksCopy_BySpell",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Spark Double@spark",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Akroma, Angel of Wrath",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Eyes in the Skies",
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
          "name": "Quasiduplicate",
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
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Spark Double"
        },
        {
          "op": "setOptionChoices",
          "player": 0,
          "kind": "options",
          "description": "Choose what to copy as this permanent enters",
          "choices": [
            "Enter as a copy of Akroma, Angel of Wrath"
          ]
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0
        },
        {
          "op": "assertPermanentCount",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Akroma, Angel of Wrath",
          "count": 2
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Quasiduplicate"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": {
            "groups": [
              [
                "@spark"
              ]
            ]
          }
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0
        },
        {
          "op": "assertPermanentCount",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Akroma, Angel of Wrath",
          "count": 3
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "END_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 0,
            "name": "Akroma, Angel of Wrath",
            "token": true
          },
          "count": 1,
          "checks": [
            {
              "property": "supertype",
              "value": "Legendary",
              "expected": false
            }
          ]
        }
      ]
    },
    {
      "name": "test_CopyOfSparksCopy_ByAbility",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Replica Fixture",
          "typeLine": "",
          "custom": true,
          "fixtureSource": true,
          "oracleText": "{0}: Create a token that's a copy of target permanent."
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Spark Double@copy",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Akroma, Angel of Wrath",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Eyes in the Skies",
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
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Spark Double"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Enter as a copy of Akroma, Angel of Wrath"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0
        },
        {
          "op": "assertPermanentCount",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Akroma, Angel of Wrath",
          "count": 2
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "create a token that"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "@copy"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0
        },
        {
          "op": "assertPermanentCount",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Akroma, Angel of Wrath",
          "count": 3
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "END_COMBAT"
        },
        {
          "op": "execute"
        }
      ]
    },
    {
      "name": "test_SparkCopyEachOther",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Spark Double@spark",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": "4 * 2"
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
          "name": "Spark Double",
          "objectAlias": "@spark.1"
        },
        {
          "op": "setOptionChoices",
          "player": 0,
          "kind": "options",
          "description": "Choose what to copy as this permanent enters",
          "choices": [
            "Enter as a copy of Grizzly Bears"
          ]
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "assertPermanentCount",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Grizzly Bears",
          "count": 2
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Spark Double",
          "objectAlias": "@spark.2"
        },
        {
          "op": "setOptionChoices",
          "player": 0,
          "kind": "options",
          "description": "Choose what to copy as this permanent enters",
          "choices": [
            {
              "description": "Enter as a copy of Grizzly Bears",
              "object": "@spark.1"
            }
          ]
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "assertPermanentCount",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Grizzly Bears",
          "count": 3
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
          "name": "@spark.1",
          "player": 0,
          "counter": "P1P1",
          "count": 1
        },
        {
          "op": "assertCounterCount",
          "name": "@spark.2",
          "player": 0,
          "counter": "P1P1",
          "count": 1
        }
      ]
    }
  ]
});
