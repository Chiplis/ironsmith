import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/soi/ThingInTheIceTest.java",
  "tests": [
    {
      "name": "testClueTokens",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Tireless Tracker",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Forest",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Thing in the Ice",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Expedite",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Mountain",
          "count": 4
        },
        {
          "op": "playLand",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Forest"
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Expedite"
        },
        {
          "op": "addTarget",
          "player": 1,
          "target": "Thing in the Ice"
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Expedite"
        },
        {
          "op": "addTarget",
          "player": 1,
          "target": "Thing in the Ice"
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Expedite"
        },
        {
          "op": "addTarget",
          "player": 1,
          "target": "Thing in the Ice"
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Expedite"
        },
        {
          "op": "addTarget",
          "player": 1,
          "target": "Thing in the Ice"
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Clue Token",
          "count": 1
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "Tireless Tracker",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Tireless Tracker",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Awoken Horror",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Expedite",
          "count": 4
        }
      ]
    }
  ]
});
