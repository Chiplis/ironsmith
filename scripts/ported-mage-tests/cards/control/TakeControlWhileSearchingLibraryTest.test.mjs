import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/control/TakeControlWhileSearchingLibraryTest.java",
  "tests": [
    {
      "name": "test_SimpleSearchingLibrary_Normal",
      "operations": [
        {
          "op": "clearZone",
          "player": 0,
          "zone": "library"
        },
        {
          "op": "clearZone",
          "player": 1,
          "zone": "library"
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Balduvian Bears@found",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 1,
          "name": "Kitesail Corsair",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Buried Alive",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 3
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Balduvian Bears",
          "count": 0
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Kitesail Corsair",
          "count": 0
        },
        {
          "op": "activateManaAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "source": "Swamp",
          "ability": "{T}: Add {B}",
          "count": 3
        },
        {
          "op": "assertManaPool",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "colors": "B",
          "amount": 3
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Buried Alive"
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Buried Alive",
          "count": 1
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "@found"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "TestPlayer.TARGET_SKIP"
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
          "name": "Balduvian Bears",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Kitesail Corsair",
          "count": 0
        }
      ]
    },
    {
      "name": "test_SimpleSearchingLibrary_TakeControl",
      "operations": [
        {
          "op": "clearZone",
          "player": 0,
          "zone": "library"
        },
        {
          "op": "clearZone",
          "player": 1,
          "zone": "library"
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Balduvian Bears",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 1,
          "name": "Kitesail Corsair",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Buried Alive",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Search Custody Fixture",
          "typeLine": "",
          "custom": true,
          "fixtureSource": true,
          "oracleText": "You control your opponents while they're searching their libraries."
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Lightning Bolt",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Balduvian Bears",
          "count": 0,
          "seatIndependent": true
        },
        {
          "op": "assertGraveyardCount",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Kitesail Corsair",
          "count": 0,
          "seatIndependent": true
        },
        {
          "op": "activateManaAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{T}: Add {B}",
          "count": 3
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Buried Alive"
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": true
        },
        {
          "op": "addTarget",
          "player": 1,
          "target": "Balduvian Bears"
        },
        {
          "op": "addTarget",
          "player": 1,
          "target": "TestPlayer.TARGET_SKIP"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "assertGraveyardCount",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Balduvian Bears",
          "count": 1,
          "seatIndependent": true
        },
        {
          "op": "assertGraveyardCount",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Kitesail Corsair",
          "count": 0,
          "seatIndependent": true
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Lightning Bolt"
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
          "phase": "BEGIN_COMBAT"
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
    },
    {
      "name": "test_CastCardWhileSearchingLibrary_Normal",
      "operations": [
        {
          "op": "clearZone",
          "player": 0,
          "zone": "library"
        },
        {
          "op": "clearZone",
          "player": 1,
          "zone": "library"
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Balduvian Bears@found",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 1,
          "name": "Kitesail Corsair",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Panglacial Wurm",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Panglacial Wurm",
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
          "zone": "HAND",
          "player": 0,
          "name": "Buried Alive",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 3
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Balduvian Bears",
          "count": 0
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Kitesail Corsair",
          "count": 0
        },
        {
          "op": "activateManaAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "source": "Swamp",
          "ability": "{T}: Add {B}",
          "count": 3
        },
        {
          "op": "assertManaPool",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "colors": "B",
          "amount": 3
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Buried Alive"
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Buried Alive",
          "count": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Panglacial Wurm"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "@found"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "TestPlayer.TARGET_SKIP"
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
          "name": "Balduvian Bears",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Kitesail Corsair",
          "count": 0
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Panglacial Wurm",
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
          "name": "Panglacial Wurm",
          "count": 2
        }
      ]
    },
    {
      "name": "test_CastCardWhileSearchingLibrary_TakeControl",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "clearZone",
          "player": 0,
          "zone": "library"
        },
        {
          "op": "clearZone",
          "player": 1,
          "zone": "library"
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "name": "Balduvian Bears@found",
          "count": 1,
          "player": 0
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "name": "Panglacial Wurm@wurm",
          "count": 1,
          "player": 0
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "name": "Kitesail Corsair",
          "count": 1,
          "player": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "name": "Panglacial Wurm",
          "count": 1,
          "player": 0
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "name": "Opposition Agent",
          "count": 1,
          "player": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "name": "Forest",
          "count": 7,
          "player": 0
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "name": "Swamp",
          "count": 3,
          "player": 0
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "name": "Buried Alive",
          "count": 1,
          "player": 0
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Buried Alive",
          "holdStack": true
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Buried Alive",
          "count": 1,
          "isSpell": true
        },
        {
          "op": "setOptionChoices",
          "description": "cast Panglacial Wurm from Alice's library while searching it",
          "choices": [
            "Yes"
          ],
          "kind": "boolean",
          "player": 1
        },
        {
          "op": "assertChoiceMenu",
          "description": "objects",
          "player": 1,
          "kind": "select_objects",
          "options": [
            "Balduvian Bears"
          ]
        },
        {
          "op": "setObjectChoices",
          "description": "objects",
          "choices": [
            {
              "alias": "@found"
            }
          ],
          "player": 1
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertExileCount",
          "player": 0,
          "name": "Balduvian Bears",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Balduvian Bears",
          "count": 0
        },
        {
          "op": "assertLibraryCount",
          "player": 1,
          "name": "Kitesail Corsair",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Panglacial Wurm",
          "count": 2
        }
      ]
    },
    {
      "name": "test_UnderControlMustUseTestCommandsCorrectrly",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mindslaver",
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
          "zone": "HAND",
          "player": 1,
          "name": "Lightning Bolt@controlledBolt",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Mountain",
          "count": 1
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
          "source": "Mindslaver",
          "ability": "Control target player",
          "target": {
            "groups": [
              [
                1
              ]
            ]
          }
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Mindslaver",
          "count": 1,
          "abilityKind": "Activated",
          "playerTarget": 1
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertActivePlayer",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Lightning Bolt",
          "target": {
            "groups": [
              [
                1
              ]
            ]
          },
          "holdStack": true,
          "objectAlias": "@controlledBolt",
          "decisionPlayer": 0
        },
        {
          "op": "assertStackObject",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "source": "Lightning Bolt",
          "count": 1,
          "isSpell": true,
          "controller": 1,
          "playerTarget": 1
        },
        {
          "op": "waitStackResolved",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 17
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 20
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Lightning Bolt",
          "count": 1
        }
      ]
    },
    {
      "name": "test_OppositionAgent",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Opposition Agent",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Buried Alive",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Balduvian Bears@found",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Mountain",
          "count": 2
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Balduvian Bears",
          "count": 0
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Kitesail Corsair",
          "count": 0
        },
        {
          "op": "activateManaAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "source": "Swamp",
          "ability": "{T}: Add {B}",
          "count": 3
        },
        {
          "op": "assertManaPool",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "colors": "B",
          "amount": 3
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Buried Alive"
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Buried Alive",
          "count": 1
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": true
        },
        {
          "op": "addTarget",
          "player": 1,
          "target": "@found"
        },
        {
          "op": "addTarget",
          "player": 1,
          "target": "TestPlayer.TARGET_SKIP"
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
          "name": "Balduvian Bears",
          "count": 0
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Kitesail Corsair",
          "count": 0
        },
        {
          "op": "assertExileCount",
          "player": 0,
          "name": "Balduvian Bears",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Balduvian Bears",
          "objectAlias": "@found",
          "permissionSource": "Opposition Agent"
        },
        {
          "op": "assertStackObject",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "source": "Balduvian Bears",
          "count": 1,
          "controller": 1
        },
        {
          "op": "waitStackResolved",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Balduvian Bears",
          "count": 1
        }
      ]
    }
  ]
});
