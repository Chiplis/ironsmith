import { registerPortedMageTests } from "../../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/abilities/oneshot/exile/SearchNameExileTests.java",
  "tests": [
    {
      "name": "testSearchAndExileSplitCards",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Surgical Extraction",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 1,
          "name": "Breaking // Entering@first",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 1,
          "name": "Breaking // Entering@second",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Breaking // Entering",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 1,
          "name": "Breaking // Entering",
          "count": 1
        },
        {
          "op": "setOptionChoices",
          "player": 0,
          "kind": "hybrid_payment",
          "description": "Choose how to pay pip 1 of Surgical Extraction",
          "choices": [
            "2 life (Phyrexian)"
          ]
        },
        {
          "op": "castSpell",
          "name": "Surgical Extraction",
          "player": 0,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "target": "@first"
        },
        {
          "op": "assertStackObject",
          "source": "Surgical Extraction",
          "count": 1,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "isSpell": true,
          "targetCount": 1,
          "objectTarget": "Breaking"
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 18
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Breaking // Entering"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Breaking // Entering"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Breaking // Entering"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Breaking // Entering"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Surgical Extraction",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Breaking // Entering",
          "count": 0
        },
        {
          "op": "assertLibraryCount",
          "player": 1,
          "name": "Breaking // Entering",
          "count": 0
        },
        {
          "op": "assertHandCount",
          "player": 1,
          "name": "Breaking // Entering",
          "count": 0
        },
        {
          "op": "assertHandCount",
          "player": 1,
          "count": 0
        },
        {
          "op": "assertExileCount",
          "player": 1,
          "name": "Breaking // Entering",
          "count": 4
        }
      ]
    },
    {
      "name": "testSearchAndExileSplitSpell",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Test of Talents",
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
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 1,
          "name": "Ready // Willing",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Ready // Willing",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 1,
          "name": "Ready // Willing",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Plains",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Forest",
          "count": 2
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Ready"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Test of Talents",
          "target": "Ready"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Ready // Willing^Ready // Willing"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Ready // Willing"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Ready // Willing"
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
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Test of Talents",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Ready // Willing",
          "count": 0
        },
        {
          "op": "assertLibraryCount",
          "player": 1,
          "name": "Ready // Willing",
          "count": 0
        },
        {
          "op": "assertHandCount",
          "player": 1,
          "name": "Ready // Willing",
          "count": 0
        },
        {
          "op": "assertHandCount",
          "player": 1,
          "count": 1
        },
        {
          "op": "assertExileCount",
          "player": 1,
          "name": "Ready // Willing",
          "count": 4
        }
      ]
    },
    {
      "name": "testFailSearchAndExileMDFCSpell",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Test of Talents",
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
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 1,
          "name": "Flamescroll Celebrant",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Flamescroll Celebrant",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 1,
          "name": "Flamescroll Celebrant",
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
          "player": 1,
          "name": "Revel in Silence"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Test of Talents",
          "target": "Revel in Silence"
        },
        {
          "op": "setStrictChooseMode",
          "value": false
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
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Flamescroll Celebrant",
          "count": 2
        },
        {
          "op": "assertLibraryCount",
          "player": 1,
          "name": "Flamescroll Celebrant",
          "count": 1
        },
        {
          "op": "assertHandCount",
          "player": 1,
          "name": "Flamescroll Celebrant",
          "count": 1
        },
        {
          "op": "assertHandCount",
          "player": 1,
          "count": 1
        },
        {
          "op": "assertExileCount",
          "player": 1,
          "name": "Flamescroll Celebrant",
          "count": 0
        }
      ]
    },
    {
      "name": "testSearchAndExileSplitSpellNonstrict",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Test of Talents",
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
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 1,
          "name": "Ready // Willing",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Ready // Willing@split.original",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 1,
          "name": "Ready // Willing",
          "count": 2
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
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Forest",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Swamp",
          "count": 2
        },
        {
          "op": "assertCastMethods",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Ready // Willing",
          "objectAlias": "@split.original",
          "methods": [
            "normal",
            "split_other_half",
            "fuse"
          ]
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "fused Ready // Willing",
          "objectAlias": "@split.original",
          "castingMethod": "fuse"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Test of Talents",
          "target": "Ready // Willing"
        },
        {
          "op": "setObjectChoices",
          "player": 0,
          "description": "Search your library for up to 6 cards with the same name as Ready // Willing",
          "choices": [
            "Ready // Willing",
            "Ready",
            "Ready",
            "Ready",
            "Ready",
            "Ready"
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
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Test of Talents",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Ready // Willing",
          "count": 0
        },
        {
          "op": "assertLibraryCount",
          "player": 1,
          "name": "Ready // Willing",
          "count": 0
        },
        {
          "op": "assertHandCount",
          "player": 1,
          "name": "Ready // Willing",
          "count": 0
        },
        {
          "op": "assertHandCount",
          "player": 1,
          "count": 1
        },
        {
          "op": "assertExileCount",
          "player": 1,
          "name": "Ready // Willing",
          "count": 6
        }
      ]
    }
  ]
});
