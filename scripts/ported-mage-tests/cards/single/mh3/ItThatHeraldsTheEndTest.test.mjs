import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/mh3/ItThatHeraldsTheEndTest.java",
  "tests": [
    {
      "name": "test_Simple",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "It That Heralds the End",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 6
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Forest",
          "count": 7
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Ebony Rhino",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Ebony Rhino",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Ebony Rhino"
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Ebony Rhino"
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
          "op": "assertTappedCount",
          "name": "Swamp",
          "tapped": true,
          "count": 6
        },
        {
          "op": "assertTappedCount",
          "name": "Forest",
          "tapped": true,
          "count": 7
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Ebony Rhino",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Ebony Rhino",
          "power": 5,
          "toughness": 6
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Ebony Rhino",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 1,
          "name": "Ebony Rhino",
          "power": 4,
          "toughness": 5
        }
      ]
    }
  ]
});
