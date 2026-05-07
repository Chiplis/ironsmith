import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/wwk/MindbreakTrapTest.java",
  "tests": [
    {
      "name": "mindBreakTrap_Exile_All_Spells",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Mountain",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Mindbreak Trap",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Shock",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Grapeshot",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Shock",
          "target": 0
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Shock",
          "target": 0
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "POSTCOMBAT_MAIN",
          "player": 1,
          "name": "Grapeshot",
          "target": 0
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": false
        },
        {
          "op": "waitStackResolved",
          "turn": 2,
          "phase": "POSTCOMBAT_MAIN",
          "player": 1
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Mindbreak Trap",
          "target": "Grapeshot^Grapeshot^Grapeshot"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Cast with alternative cost: {0} (source: Mindbreak Trap"
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Shock",
          "count": 2
        },
        {
          "op": "assertExileCount",
          "player": 1,
          "name": "Grapeshot",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Mindbreak Trap",
          "count": 1
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 16
        }
      ]
    }
  ]
});
