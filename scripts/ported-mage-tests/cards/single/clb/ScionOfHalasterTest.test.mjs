import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/clb/ScionOfHalasterTest.java",
  "tests": [
    {
      "name": "TestScionOfHalaster",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Scion of Halaster",
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
          "name": "Island",
          "count": 4
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
          "zone": "COMMAND",
          "player": 0,
          "name": "Thrasios, Triton Hero",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Thrasios, Triton Hero"
        },
        {
          "op": "clearZone",
          "player": 0,
          "zone": "library"
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Lightning Bolt",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Mountain",
          "count": 1
        },
        {
          "op": "skipInitShuffling"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setObjectChoices",
          "player": 0,
          "description": "Choose exactly 1 card",
          "choices": [
            "Mountain"
          ]
        },
        {
          "op": "assertLibraryCount",
          "turn": 3,
          "phase": "UPKEEP",
          "player": 0,
          "count": 2
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertLibraryCount",
          "player": 0,
          "count": 0
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "count": 1
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "Lightning Bolt",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Mountain",
          "count": 1
        }
      ],
      "deferredPriorityAnalysis": true
    },
    {
      "name": "TestScionOfHalaster_withPartnerCommanders",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Scion of Halaster",
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
          "name": "Island",
          "count": 4
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
          "zone": "COMMAND",
          "player": 0,
          "name": "Thrasios, Triton Hero",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "COMMAND",
          "player": 0,
          "name": "Ishai, Ojutai Dragonspeaker",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Thrasios, Triton Hero",
          "resolve": true
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Ishai, Ojutai Dragonspeaker"
        },
        {
          "op": "clearZone",
          "player": 0,
          "zone": "library"
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Lightning Bolt",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Island",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Mountain",
          "count": 1
        },
        {
          "op": "skipInitShuffling"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setOptionChoices",
          "player": 0,
          "kind": "replacement_effect",
          "description": "Choose which replacement effect to apply",
          "choices": [
            "Thrasios, Triton Hero"
          ]
        },
        {
          "op": "setObjectChoices",
          "player": 0,
          "description": "Choose exactly 1 card",
          "choices": [
            "Mountain"
          ]
        },
        {
          "op": "setObjectChoices",
          "player": 0,
          "description": "Choose exactly 1 card",
          "choices": [
            "Island"
          ]
        },
        {
          "op": "assertLibraryCount",
          "turn": 3,
          "phase": "UPKEEP",
          "player": 0,
          "count": 3
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertLibraryCount",
          "player": 0,
          "count": 0
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "count": 1
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "Lightning Bolt",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "count": 2
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Mountain",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Island",
          "count": 1
        }
      ],
      "deferredPriorityAnalysis": true
    }
  ]
});
