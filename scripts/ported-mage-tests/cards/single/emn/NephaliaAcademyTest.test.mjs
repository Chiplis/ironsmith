import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/emn/NephaliaAcademyTest.java",
  "tests": [
    {
      "name": "testReplacementEffectBySpell",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Duress",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Giant Growth",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Nephalia Academy",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Duress"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Giant Growth"
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": true
        },
        {
          "op": "execute"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Duress",
          "count": 1
        },
        {
          "op": "assertHandCount",
          "player": 1,
          "name": "Giant Growth",
          "count": 0
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Giant Growth",
          "count": 0
        },
        {
          "op": "assertLibraryCount",
          "player": 1,
          "name": "Giant Growth",
          "count": 1
        }
      ]
    },
    {
      "name": "testDeclineReplacementEffectBySpell",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Duress",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Giant Growth",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Nephalia Academy",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Duress"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Giant Growth"
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": false
        },
        {
          "op": "execute"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Duress",
          "count": 1
        },
        {
          "op": "assertHandCount",
          "player": 1,
          "name": "Giant Growth",
          "count": 0
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Giant Growth",
          "count": 1
        }
      ]
    },
    {
      "name": "testShouldNotApplyToOwnDiscardSpell",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Nephalia Academy",
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
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Memnite",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Sift",
          "count": 1
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Sift"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Memnite"
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
          "name": "Sift",
          "count": 1
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "Memnite",
          "count": 0
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Memnite",
          "count": 1
        }
      ]
    }
  ]
});
