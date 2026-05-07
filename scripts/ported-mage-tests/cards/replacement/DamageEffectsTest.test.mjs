import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/replacement/DamageEffectsTest.java",
  "tests": [
    {
      "name": "testDamageIsDoubledWithLifelink",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Ob Nixilis, the Fallen",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Mountain",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Wurmcoil Engine",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Gratuitous Violence",
          "count": 1
        },
        {
          "op": "playLand",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Mountain"
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": true
        },
        {
          "op": "addTarget",
          "player": 1,
          "target": 0
        },
        {
          "op": "attack",
          "turn": 2,
          "player": 1,
          "attacker": "Ob Nixilis, the Fallen",
          "defender": 1
        },
        {
          "op": "block",
          "turn": 2,
          "player": 0,
          "blocker": "Wurmcoil Engine",
          "attacker": "Ob Nixilis, the Fallen"
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
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Ob Nixilis, the Fallen",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Wurmcoil Engine",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Phyrexian Wurm Token",
          "count": 2
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 20
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 29
        }
      ]
    },
    {
      "name": "testDamageToPlayer",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Wurmcoil Engine",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Gratuitous Violence",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Wurmcoil Engine",
          "defender": 1
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
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Wurmcoil Engine",
          "count": 1
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 8
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 32
        }
      ]
    }
  ]
});
