import { registerPortedMageTests } from "../../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/abilities/oneshot/regenerate/NecrobiteRegenerateTest.java",
  "tests": [
    {
      "name": "testRegenerateAndDeathtouch",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Craw Wurm",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Elite Vanguard",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Swamp",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Necrobite",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 2,
          "player": 1,
          "attacker": "Elite Vanguard",
          "defender": 1
        },
        {
          "op": "block",
          "turn": 2,
          "player": 0,
          "blocker": "Craw Wurm",
          "attacker": "Elite Vanguard"
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "DECLARE_BLOCKERS",
          "player": 1,
          "name": "Necrobite",
          "target": "Elite Vanguard"
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 20
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 20
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Craw Wurm",
          "count": 0
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Craw Wurm",
          "count": 1
        }
      ]
    }
  ]
});
