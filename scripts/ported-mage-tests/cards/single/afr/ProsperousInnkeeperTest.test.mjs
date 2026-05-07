import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/afr/ProsperousInnkeeperTest.java",
  "tests": [
    {
      "name": "createTreasureToken",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Prosperous Innkeeper",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 2
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Prosperous Innkeeper"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Treasure Token",
          "count": 1
        }
      ]
    },
    {
      "name": "gainLife",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Prosperous Innkeeper",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 2
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Prosperous Innkeeper"
        },
        {
          "op": "castSpell",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Prosperous Innkeeper"
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 21
        }
      ]
    }
  ]
});
