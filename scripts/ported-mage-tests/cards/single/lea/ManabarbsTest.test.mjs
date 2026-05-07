import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/lea/ManabarbsTest.java",
  "tests": [
    {
      "name": "testMultiTriggers",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Manabarbs",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 7
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Lightning Elemental",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Ball Lightning",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Ball Lightning"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Lightning Elemental"
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Ball Lightning",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Lightning Elemental",
          "defender": 1
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "UNTAP"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 13
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 10
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Lightning Elemental",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Ball Lightning",
          "count": 0
        }
      ]
    }
  ]
});
