import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/ncc/WeatheredSentinelsTest.java",
  "tests": [
    {
      "name": "testCantAttackNonAttacker",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Weathered Sentinels",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Weathered Sentinels",
          "defender": 1,
          "expectedLegal": false
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
          "op": "assertTapped",
          "player": 0,
          "name": "Weathered Sentinels",
          "tapped": false
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 20
        }
      ]
    },
    {
      "name": "testCanAttackAttacker",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Weathered Sentinels",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Gingerbrute",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 2,
          "player": 1,
          "attacker": "Gingerbrute",
          "defender": 0
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Weathered Sentinels",
          "defender": 1
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
          "op": "assertAbility",
          "player": 0,
          "name": "Weathered Sentinels",
          "ability": "Indestructible",
          "expected": true
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Weathered Sentinels",
          "power": 5,
          "toughness": 8
        },
        {
          "op": "setStopAt",
          "turn": 4,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "execute"
        }
      ]
    }
  ]
});
