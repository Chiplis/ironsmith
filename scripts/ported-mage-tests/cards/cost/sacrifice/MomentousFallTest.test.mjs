import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/cost/sacrifice/MomentousFallTest.java",
  "tests": [
    {
      "name": "testSacrificeCostAndLKI",
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
          "name": "Momentous Fall",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Geralf's Messenger",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Glorious Anthem",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Momentous Fall"
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
          "name": "Geralf's Messenger",
          "count": 1
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Geralf's Messenger",
          "counter": "P1P1",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Geralf's Messenger",
          "power": 5,
          "toughness": 4
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "count": 0,
          "name": 4
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 23
        }
      ]
    },
    {
      "name": "testSacrificeCostForProGreen",
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
          "name": "Momentous Fall",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mirran Crusader",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Momentous Fall"
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
          "name": "Mirran Crusader",
          "count": 0
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "count": 0,
          "name": 2
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 22
        }
      ]
    }
  ]
});
