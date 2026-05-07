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
          "ability": "Level up {1}"
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
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
          "op": "execute"
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
          "op": "execute"
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
          "op": "execute"
        }
      ]
    }
  ]
});
