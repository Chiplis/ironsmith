import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/who/LunarHatchlingTest.java",
  "tests": [
    {
      "name": "test_EscapeWithAdditionalCost",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Lunar Hatchling",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 3
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
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Balduvian Bears",
          "count": 5
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Lunar Hatchling with Escape"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Forest"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Balduvian Bears^Balduvian Bears^Balduvian Bears^Balduvian Bears^Balduvian Bears"
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
          "name": "Lunar Hatchling",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Forest",
          "count": 2
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Island",
          "count": 3
        },
        {
          "op": "assertExileCount",
          "player": 0,
          "name": "Forest",
          "count": 1
        },
        {
          "op": "assertExileCount",
          "player": 0,
          "name": "Balduvian Bears",
          "count": 5
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Balduvian Bears",
          "count": 0
        }
      ]
    }
  ]
});
