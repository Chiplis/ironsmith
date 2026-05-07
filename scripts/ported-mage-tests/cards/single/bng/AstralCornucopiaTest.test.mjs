import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/bng/AstralCornucopiaTest.java",
  "tests": [
    {
      "name": "testCorrectManaAmount",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 6
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Astral Cornucopia",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Arcane Signet",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Astral Cornucopia"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "X=2"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Arcane Signet"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Astral Cornucopia",
          "count": 1
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Astral Cornucopia",
          "counter": "CHARGE",
          "count": 2
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Arcane Signet",
          "count": 1
        }
      ]
    }
  ]
});
