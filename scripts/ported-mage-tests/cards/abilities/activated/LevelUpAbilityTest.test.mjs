import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/abilities/activated/LevelUpAbilityTest.java",
  "tests": [
    {
      "name": "testFirstLevel",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Transcendent Master",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 15
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Transcendent Master",
          "counter": "LEVEL",
          "count": 0
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Transcendent Master",
          "power": 3,
          "toughness": 3
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Transcendent Master",
          "ability": "Lifelink",
          "expected": false
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Transcendent Master",
          "ability": "Indestructible",
          "expected": false
        }
      ]
    },
    {
      "name": "testFirstLevelWithOneCounter",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Transcendent Master",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 15
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "source": "Transcendent Master",
          "ability": "Level up {1}",
          "resolve": true
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Transcendent Master",
          "counter": "LEVEL",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Transcendent Master",
          "power": 3,
          "toughness": 3
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Transcendent Master",
          "ability": "Lifelink",
          "expected": false
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Transcendent Master",
          "ability": "Indestructible",
          "expected": false
        }
      ]
    },
    {
      "name": "testSecondLevel",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Transcendent Master",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 15
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "source": "Transcendent Master",
          "ability": "Level up {1}",
          "resolve": true
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "source": "Transcendent Master",
          "ability": "Level up {1}",
          "resolve": true
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "source": "Transcendent Master",
          "ability": "Level up {1}",
          "resolve": true
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "source": "Transcendent Master",
          "ability": "Level up {1}",
          "resolve": true
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "source": "Transcendent Master",
          "ability": "Level up {1}",
          "resolve": true
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "source": "Transcendent Master",
          "ability": "Level up {1}",
          "resolve": true
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Transcendent Master",
          "counter": "LEVEL",
          "count": 6
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Transcendent Master",
          "power": 6,
          "toughness": 6
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Transcendent Master",
          "ability": "Lifelink",
          "expected": true
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Transcendent Master",
          "ability": "Indestructible",
          "expected": false
        }
      ]
    },
    {
      "name": "testThirdLevel",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Transcendent Master",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 15
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "source": "Transcendent Master",
          "ability": "Level up {1}",
          "resolve": true
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "source": "Transcendent Master",
          "ability": "Level up {1}",
          "resolve": true
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "source": "Transcendent Master",
          "ability": "Level up {1}",
          "resolve": true
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "source": "Transcendent Master",
          "ability": "Level up {1}",
          "resolve": true
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "source": "Transcendent Master",
          "ability": "Level up {1}",
          "resolve": true
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "source": "Transcendent Master",
          "ability": "Level up {1}",
          "resolve": true
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "source": "Transcendent Master",
          "ability": "Level up {1}",
          "resolve": true
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "source": "Transcendent Master",
          "ability": "Level up {1}",
          "resolve": true
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "source": "Transcendent Master",
          "ability": "Level up {1}",
          "resolve": true
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "source": "Transcendent Master",
          "ability": "Level up {1}",
          "resolve": true
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "source": "Transcendent Master",
          "ability": "Level up {1}",
          "resolve": true
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "source": "Transcendent Master",
          "ability": "Level up {1}",
          "resolve": true
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Transcendent Master",
          "counter": "LEVEL",
          "count": 12
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Transcendent Master",
          "power": 9,
          "toughness": 9
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Transcendent Master",
          "ability": "Lifelink",
          "expected": true
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Transcendent Master",
          "ability": "Indestructible",
          "expected": true
        }
      ]
    },
    {
      "name": "testExtraCounters",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Transcendent Master",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 15
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "source": "Transcendent Master",
          "ability": "Level up {1}",
          "resolve": true
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "source": "Transcendent Master",
          "ability": "Level up {1}",
          "resolve": true
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "source": "Transcendent Master",
          "ability": "Level up {1}",
          "resolve": true
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "source": "Transcendent Master",
          "ability": "Level up {1}",
          "resolve": true
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "source": "Transcendent Master",
          "ability": "Level up {1}",
          "resolve": true
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "source": "Transcendent Master",
          "ability": "Level up {1}",
          "resolve": true
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "source": "Transcendent Master",
          "ability": "Level up {1}",
          "resolve": true
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "source": "Transcendent Master",
          "ability": "Level up {1}",
          "resolve": true
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "source": "Transcendent Master",
          "ability": "Level up {1}",
          "resolve": true
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "source": "Transcendent Master",
          "ability": "Level up {1}",
          "resolve": true
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "source": "Transcendent Master",
          "ability": "Level up {1}",
          "resolve": true
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "source": "Transcendent Master",
          "ability": "Level up {1}",
          "resolve": true
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "source": "Transcendent Master",
          "ability": "Level up {1}",
          "resolve": true
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "source": "Transcendent Master",
          "ability": "Level up {1}",
          "resolve": true
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "source": "Transcendent Master",
          "ability": "Level up {1}",
          "resolve": true
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Transcendent Master",
          "counter": "LEVEL",
          "count": 15
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Transcendent Master",
          "power": 9,
          "toughness": 9
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Transcendent Master",
          "ability": "Lifelink",
          "expected": true
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Transcendent Master",
          "ability": "Indestructible",
          "expected": true
        }
      ]
    }
  ]
});
