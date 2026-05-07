import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/avr/CraterhoofBehemothTest.java",
  "tests": [
    {
      "name": "test_simple",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Craterhoof Behemoth",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Elite Vanguard",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Grizzly Bears",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 8
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Goblin Piker",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Merfolk of the Pearl Trident",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Craterhoof Behemoth"
        },
        {
          "op": "assertAbility",
          "player": 1,
          "name": "POSTCOMBAT_MAIN",
          "ability": 0,
          "expected": "Craterhoof Behemoth"
        },
        {
          "op": "assertAbility",
          "player": 1,
          "name": "POSTCOMBAT_MAIN",
          "ability": 0,
          "expected": "Elite Vanguard"
        },
        {
          "op": "assertAbility",
          "player": 1,
          "name": "POSTCOMBAT_MAIN",
          "ability": 0,
          "expected": "Grizzly Bears"
        },
        {
          "op": "assertAbility",
          "player": 1,
          "name": "POSTCOMBAT_MAIN",
          "ability": 1,
          "expected": "Goblin Piker"
        },
        {
          "op": "assertAbility",
          "player": 1,
          "name": "POSTCOMBAT_MAIN",
          "ability": 1,
          "expected": "Merfolk of the Pearl Trident"
        },
        {
          "op": "assertPowerToughness",
          "player": 1,
          "name": "POSTCOMBAT_MAIN",
          "power": 0,
          "toughness": "Craterhoof Behemoth"
        },
        {
          "op": "assertPowerToughness",
          "player": 1,
          "name": "POSTCOMBAT_MAIN",
          "power": 0,
          "toughness": "Elite Vanguard"
        },
        {
          "op": "assertPowerToughness",
          "player": 1,
          "name": "POSTCOMBAT_MAIN",
          "power": 0,
          "toughness": "Grizzly Bears"
        },
        {
          "op": "assertPowerToughness",
          "player": 1,
          "name": "POSTCOMBAT_MAIN",
          "power": 1,
          "toughness": "Goblin Piker"
        },
        {
          "op": "assertPowerToughness",
          "player": 1,
          "name": "POSTCOMBAT_MAIN",
          "power": 1,
          "toughness": "Merfolk of the Pearl Trident"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Craterhoof Behemoth",
          "power": 5,
          "toughness": 5
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Elite Vanguard",
          "power": 2,
          "toughness": 1
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Craterhoof Behemoth",
          "ability": "Trample",
          "expected": false
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Elite Vanguard",
          "ability": "Trample",
          "expected": false
        }
      ]
    }
  ]
});
