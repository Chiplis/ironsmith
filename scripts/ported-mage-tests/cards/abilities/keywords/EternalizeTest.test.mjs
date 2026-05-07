import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/abilities/keywords/EternalizeTest.java",
  "tests": [
    {
      "name": "testEternalize",
      "operations": [
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Steadfast Sentinel",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 10
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "Eternalize"
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
          "name": "Steadfast Sentinel",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Steadfast Sentinel",
          "power": 4,
          "toughness": 4
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Steadfast Sentinel",
          "ability": "Vigilance",
          "expected": true
        }
      ]
    },
    {
      "name": "testEternalizeAndFatalPush",
      "operations": [
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Steadfast Sentinel",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 10
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Fatal Push",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Swamp",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "Eternalize"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "BEGIN_COMBAT",
          "player": 1,
          "name": "Fatal Push",
          "target": "Steadfast Sentinel"
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
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Fatal Push",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Steadfast Sentinel",
          "count": 0
        },
        {
          "op": "assertExileCount",
          "player": 0,
          "name": "Steadfast Sentinel",
          "count": 1
        }
      ]
    }
  ]
});
