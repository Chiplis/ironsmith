import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/c16/DivergentTransformationsTest.java",
  "tests": [
    {
      "name": "testDivergentTransformationsExiles",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Divergent Transformations",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Memnite",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Grizzly Bears",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 6
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Hill Giant",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 1,
          "name": "Mogg Flunkies",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Divergent Transformations",
          "target": "Memnite'^'Grizzly Bears"
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
          "name": "Divergent Transformations",
          "count": 1
        },
        {
          "op": "assertTappedCount",
          "name": "Mountain",
          "tapped": true,
          "count": 6
        },
        {
          "op": "assertExileCount",
          "player": 0,
          "name": "Memnite",
          "count": 1
        },
        {
          "op": "assertExileCount",
          "player": 0,
          "count": 0,
          "name": 1
        },
        {
          "op": "assertExileCount",
          "player": 1,
          "name": "Grizzly Bears",
          "count": 1
        },
        {
          "op": "assertExileCount",
          "player": 0,
          "count": 1,
          "name": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Memnite",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Grizzly Bears",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Hill Giant",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Mogg Flunkies",
          "count": 1
        }
      ]
    }
  ]
});
