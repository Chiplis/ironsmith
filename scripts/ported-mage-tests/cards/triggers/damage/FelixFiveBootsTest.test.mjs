import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/triggers/damage/FelixFiveBootsTest.java",
  "tests": [
    {
      "name": "testBasicFelixFunctionality",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Felix Five-Boots",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Belligerent Guest",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Belligerent Guest",
          "defender": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Whenever {this} deals combat damage to a player, create a Blood token."
        },
        {
          "op": "assertStackSize",
          "turn": 1,
          "phase": "COMBAT_DAMAGE",
          "player": 0,
          "count": 2
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        }
      ]
    },
    {
      "name": "testDoubleTriggerDeadAttacker",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Felix Five-Boots",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Belligerent Guest",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Moss Viper",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Belligerent Guest",
          "defender": 1
        },
        {
          "op": "block",
          "turn": 1,
          "player": 1,
          "blocker": "Moss Viper",
          "attacker": "Belligerent Guest"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "X=1"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Whenever {this} deals combat damage to a player, create a Blood token."
        },
        {
          "op": "assertStackSize",
          "turn": 1,
          "phase": "COMBAT_DAMAGE",
          "player": 0,
          "count": 2
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        }
      ]
    },
    {
      "name": "testNoBonusTriggerForEnemy",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Belligerent Guest",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Felix Five-Boots",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Belligerent Guest",
          "defender": 1
        },
        {
          "op": "assertStackSize",
          "turn": 1,
          "phase": "COMBAT_DAMAGE",
          "player": 0,
          "count": 1
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        }
      ]
    },
    {
      "name": "testNoTriggerOnNonCombatDamage",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Felix Five-Boots",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Nettle Drone",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Curiosity",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Curiosity",
          "target": "Nettle Drone"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "BEGIN_COMBAT",
          "player": 0,
          "ability": "{T}: {this} deals 1 damage to each opponent"
        },
        {
          "op": "assertStackSize",
          "turn": 1,
          "phase": "BEGIN_COMBAT",
          "player": 0,
          "count": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        }
      ]
    },
    {
      "name": "testBatchEvent",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Felix Five-Boots",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Olivia's Attendants",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Olivia's Attendants",
          "defender": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Whenever {this} deals damage, create that many Blood tokens."
        },
        {
          "op": "assertStackSize",
          "turn": 1,
          "phase": "COMBAT_DAMAGE",
          "player": 0,
          "count": 2
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
          "name": "Blood Token",
          "count": 12
        }
      ]
    },
    {
      "name": "testSelectRightPartOfBatch",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Felix Five-Boots",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Umezawa's Jitte",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 2
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
          "name": "Raging Goblin",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Wall of Blossoms",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "Equip",
          "target": "Elite Vanguard"
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Elite Vanguard",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Raging Goblin",
          "defender": 1
        },
        {
          "op": "block",
          "turn": 1,
          "player": 1,
          "blocker": "Wall of Blossoms",
          "attacker": "Elite Vanguard"
        },
        {
          "op": "assertStackSize",
          "turn": 1,
          "phase": "COMBAT_DAMAGE",
          "player": 0,
          "count": 1
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
          "op": "assertCounterCount",
          "player": 0,
          "name": "Umezawa's Jitte",
          "counter": "CHARGE",
          "count": 2
        }
      ]
    }
  ]
});
