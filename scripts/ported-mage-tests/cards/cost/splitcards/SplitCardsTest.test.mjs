import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/cost/splitcards/SplitCardsTest.java",
  "tests": [
    {
      "name": "testReturnCardFromSoulfireGrandMaster",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Volcanic Island",
          "count": 6
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Soulfire Grand Master",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Fire // Ice@split",
          "count": 1
        },
        {
          "op": "activateAbility",
          "source": "Soulfire Grand Master",
          "ability": "The next time you cast",
          "player": 0,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertStackObject",
          "source": "Soulfire Grand Master",
          "count": 1,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "abilityKind": "Activated"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "castSpell",
          "name": "Fire",
          "player": 0,
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "objectAlias": "@split",
          "castingMethod": "normal",
          "target": 1
        },
        {
          "op": "assertStackObject",
          "source": "Fire",
          "count": 1,
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "isSpell": true,
          "targetCount": 1
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 22
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 18
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "@split",
          "count": 1
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "Fire // Ice",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "@split",
          "count": 0
        }
      ]
    }
  ]
});
