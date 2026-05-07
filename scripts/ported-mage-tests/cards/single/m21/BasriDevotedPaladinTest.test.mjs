import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/m21/BasriDevotedPaladinTest.java",
  "tests": [
    {
      "name": "testAddCounter",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Basri, Devoted Paladin",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Savannah Lions",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "+1: "
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Savannah Lions"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
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
          "op": "assertAbility",
          "player": 0,
          "name": "Savannah Lions",
          "ability": "Vigilance",
          "expected": true
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Savannah Lions",
          "counter": "P1P1",
          "count": 1
        }
      ]
    },
    {
      "name": "testAttackTrigger",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Basri, Devoted Paladin",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Savannah Lions",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "-1: "
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Savannah Lions",
          "defender": 1
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 5,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 17
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Savannah Lions",
          "counter": "P1P1",
          "count": 1
        }
      ]
    },
    {
      "name": "testUltimate",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Basri, Devoted Paladin",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Savannah Lions",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "+1: "
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Savannah Lions"
        },
        {
          "op": "activateAbility",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "+1: "
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Savannah Lions"
        },
        {
          "op": "activateAbility",
          "turn": 5,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "-6: "
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 5,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Savannah Lions",
          "power": 6,
          "toughness": 5
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Savannah Lions",
          "ability": "Flying",
          "expected": true
        }
      ]
    }
  ]
});
