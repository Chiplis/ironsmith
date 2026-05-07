import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/abilities/keywords/FadingTest.java",
  "tests": [
    {
      "name": "testFading",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Blastoderm",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Blastoderm"
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
          "name": "Blastoderm",
          "count": 1
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Blastoderm",
          "counter": "FADE",
          "count": 3
        }
      ]
    },
    {
      "name": "testFades",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Blastoderm",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Blastoderm"
        },
        {
          "op": "setStopAt",
          "turn": 5,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Blastoderm",
          "count": 1
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Blastoderm",
          "counter": "FADE",
          "count": 1
        }
      ]
    },
    {
      "name": "testFadesAway",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Blastoderm",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Blastoderm"
        },
        {
          "op": "setStopAt",
          "turn": 9,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Blastoderm",
          "count": 0
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Blastoderm",
          "count": 1
        }
      ]
    }
  ]
});
