import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/cost/splitcards/SplitCardsReboundTest.java",
  "tests": [
    {
      "name": "test_FireIce",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "name": "Cast Through Time",
          "count": 1,
          "player": 0
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "name": "Mountain",
          "count": 1,
          "player": 0
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "name": "Swamp",
          "count": 1,
          "player": 0
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "name": "Fire // Ice",
          "count": 1,
          "player": 0
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Fire",
          "target": {
            "groups": [
              [
                1
              ]
            ]
          },
          "holdStack": true
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Fire",
          "count": 1,
          "isSpell": true,
          "playerTarget": 1
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertExileCount",
          "player": 0,
          "name": "Fire // Ice",
          "count": 1
        },
        {
          "op": "setOptionChoices",
          "description": "Perform the effect",
          "choices": [
            "Yes"
          ],
          "kind": "boolean",
          "player": 0
        },
        {
          "op": "setOptionChoices",
          "description": "Choose which spell to cast",
          "choices": [
            "Ice"
          ],
          "kind": "options",
          "player": 0
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Mountain"
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
          "op": "assertLife",
          "player": 1,
          "life": 18
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Fire // Ice",
          "count": 1
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "count": 2
        },
        {
          "op": "assertTapped",
          "player": 0,
          "name": "Mountain",
          "tapped": true
        },
        {
          "op": "assertTapped",
          "player": 0,
          "name": "Swamp",
          "tapped": false
        }
      ]
    }
  ]
});
