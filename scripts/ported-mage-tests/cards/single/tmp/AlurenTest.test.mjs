import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/tmp/AlurenTest.java",
  "tests": [
    {
      "name": "testAluren",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Aluren",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Bear Cub",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Bear Cub",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "UPKEEP",
          "player": 0,
          "name": "Bear Cub"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Cast without paying its mana cost"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "END_TURN",
          "player": 1,
          "name": "Bear Cub"
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": "Cast without paying its mana cost"
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "UPKEEP"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Bear Cub",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Bear Cub",
          "count": 1
        }
      ]
    }
  ]
});
