import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/abilities/keywords/GraftTest.java",
  "tests": [
    {
      "name": "testGraft",
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
          "name": "Sporeback Troll",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Sporeback Troll"
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
          "name": "Sporeback Troll",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Sporeback Troll",
          "power": 2,
          "toughness": 2
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Sporeback Troll",
          "counter": "P1P1",
          "count": 2
        }
      ]
    },
    {
      "name": "testMoveCounters",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 8
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Cytoplast Root-Kin",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Sporeback Troll",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Cytoplast Root-Kin"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Sporeback Troll"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Sporeback Troll",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Cytoplast Root-Kin",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Sporeback Troll",
          "power": 3,
          "toughness": 3
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Cytoplast Root-Kin",
          "power": 3,
          "toughness": 3
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Sporeback Troll",
          "counter": "P1P1",
          "count": 3
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Cytoplast Root-Kin",
          "counter": "P1P1",
          "count": 3
        }
      ]
    },
    {
      "name": "testDontMoveCounters",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 8
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Cytoplast Root-Kin",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Sporeback Troll",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Cytoplast Root-Kin"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Sporeback Troll"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": false
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Sporeback Troll",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Cytoplast Root-Kin",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Sporeback Troll",
          "power": 2,
          "toughness": 2
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Cytoplast Root-Kin",
          "power": 4,
          "toughness": 4
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Sporeback Troll",
          "counter": "P1P1",
          "count": 2
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Cytoplast Root-Kin",
          "counter": "P1P1",
          "count": 4
        }
      ]
    }
  ]
});
