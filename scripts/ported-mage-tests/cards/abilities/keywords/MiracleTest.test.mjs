import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/abilities/keywords/MiracleTest.java",
  "tests": [
    {
      "name": "testMiracleCost",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 2
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
          "zone": "LIBRARY",
          "player": 0,
          "name": "Terminus",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Think Twice",
          "count": 1
        },
        {
          "op": "skipInitShuffling"
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Elite Vanguard",
          "count": 1
        },
        {
          "op": "setOptionChoices",
          "kind": "options",
          "description": "Choose a Miracle reveal",
          "player": 0,
          "choices": [
            "Reveal using Miracle ({W})"
          ]
        },
        {
          "op": "setOptionChoices",
          "kind": "boolean",
          "description": "Cast Terminus for its miracle cost ({W})?",
          "player": 0,
          "choices": [
            "Yes"
          ]
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Think Twice"
        },
        {
          "op": "assertStackObject",
          "source": "Think Twice",
          "count": 1,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "resolutions": 1
        },
        {
          "op": "assertGraveyardCount",
          "name": "Think Twice",
          "count": 1,
          "player": 0
        },
        {
          "op": "assertHandCount",
          "name": "Terminus",
          "count": 1,
          "player": 0
        },
        {
          "op": "assertStackObject",
          "source": "Terminus",
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
          "op": "assertGraveyardCount",
          "name": "Terminus",
          "count": 1,
          "player": 0
        },
        {
          "op": "assertPermanentCount",
          "name": "Elite Vanguard",
          "count": 0,
          "player": 1
        },
        {
          "op": "assertLibraryCount",
          "name": "Elite Vanguard",
          "count": 1,
          "player": 1
        }
      ]
    },
    {
      "name": "testMiracleOnExtraTurn",
      "operations": [
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
          "name": "Plains",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Terminus",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Temporal Mastery",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Think Twice",
          "count": 1
        },
        {
          "op": "skipInitShuffling"
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Elite Vanguard",
          "count": 1
        },
        {
          "op": "setOptionChoices",
          "kind": "options",
          "description": "Choose a Miracle reveal",
          "player": 0,
          "choices": [
            "Reveal using Miracle ({1}{U})"
          ]
        },
        {
          "op": "setOptionChoices",
          "kind": "boolean",
          "description": "Cast Temporal Mastery for its miracle cost ({1}{U})?",
          "player": 0,
          "choices": [
            "Yes"
          ]
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Think Twice"
        },
        {
          "op": "assertStackObject",
          "source": "Think Twice",
          "count": 1,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "resolutions": 1
        },
        {
          "op": "assertGraveyardCount",
          "name": "Think Twice",
          "count": 1,
          "player": 0
        },
        {
          "op": "assertHandCount",
          "name": "Temporal Mastery",
          "count": 1,
          "player": 0
        },
        {
          "op": "assertStackObject",
          "source": "Temporal Mastery",
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
          "op": "assertExileCount",
          "name": "Temporal Mastery",
          "count": 1,
          "player": 0
        },
        {
          "op": "setOptionChoices",
          "kind": "options",
          "description": "Choose a Miracle reveal",
          "player": 0,
          "choices": [
            "Reveal using Miracle ({W})"
          ]
        },
        {
          "op": "setOptionChoices",
          "kind": "boolean",
          "description": "Cast Terminus for its miracle cost ({W})?",
          "player": 0,
          "choices": [
            "Yes"
          ]
        },
        {
          "op": "assertActivePlayer",
          "player": 0,
          "turn": 2,
          "phase": "DRAW"
        },
        {
          "op": "waitStackResolved",
          "turn": 2,
          "phase": "DRAW"
        },
        {
          "op": "assertGraveyardCount",
          "name": "Terminus",
          "count": 1,
          "player": 0
        },
        {
          "op": "assertPermanentCount",
          "name": "Elite Vanguard",
          "count": 0,
          "player": 1
        },
        {
          "op": "assertLibraryCount",
          "name": "Elite Vanguard",
          "count": 1,
          "player": 1
        }
      ]
    },
    {
      "name": "testMiracleWillWorkFromHand",
      "operations": [
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
          "player": 0,
          "name": "Mountain",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Plains",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Forest",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Thunderous Wrath",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Brainstorm",
          "count": 1
        },
        {
          "op": "skipInitShuffling"
        },
        {
          "op": "setOptionChoices",
          "kind": "options",
          "description": "Choose a Miracle reveal",
          "player": 0,
          "choices": [
            "Reveal using Miracle ({R})"
          ]
        },
        {
          "op": "setOptionChoices",
          "kind": "boolean",
          "description": "Cast Thunderous Wrath for its miracle cost ({R})?",
          "player": 0,
          "choices": [
            "Yes"
          ]
        },
        {
          "op": "setObjectChoices",
          "player": 0,
          "description": "Choose card in your hand",
          "choices": [
            "Forest",
            "Plains"
          ]
        },
        {
          "op": "setOptionChoices",
          "kind": "order",
          "description": "Order the selected cards for the top of the library. The first option becomes the topmost card.",
          "player": 0,
          "choices": [
            "Forest",
            "Plains"
          ]
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "UPKEEP",
          "player": 0,
          "name": "Brainstorm"
        },
        {
          "op": "assertStackObject",
          "source": "Brainstorm",
          "count": 1,
          "turn": 1,
          "phase": "UPKEEP"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": 1
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "UPKEEP",
          "resolutions": 1
        },
        {
          "op": "assertGraveyardCount",
          "name": "Brainstorm",
          "count": 1,
          "player": 0
        },
        {
          "op": "assertHandCount",
          "name": "Thunderous Wrath",
          "count": 1,
          "player": 0
        },
        {
          "op": "assertStackObject",
          "source": "Thunderous Wrath",
          "count": 1,
          "turn": 1,
          "phase": "UPKEEP",
          "abilityKind": "Triggered"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "UPKEEP"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Brainstorm",
          "count": 1
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "Thunderous Wrath",
          "count": 0
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Thunderous Wrath",
          "count": 1
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "count": 0
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 20
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 15
        }
      ]
    }
  ]
});
