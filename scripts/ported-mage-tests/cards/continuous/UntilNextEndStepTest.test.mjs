import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/continuous/UntilNextEndStepTest.java",
  "tests": [
    {
      "name": "testSameTurnTrue",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "tester",
          "custom": true,
          "typeLine": "Creature",
          "manaCost": "",
          "oracleText": "{0}: This creature gets +1/+1 until your next end step.",
          "fixtureSource": true,
          "power": "1",
          "toughness": "1"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{0}"
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
          "op": "assertPowerToughness",
          "player": 0,
          "name": "tester",
          "power": 2,
          "toughness": 2
        }
      ]
    },
    {
      "name": "testSameTurnFalse",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "tester",
          "custom": true,
          "typeLine": "Creature",
          "manaCost": "",
          "oracleText": "{0}: This creature gets +1/+1 until your next end step.",
          "fixtureSource": true,
          "power": "1",
          "toughness": "1"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{0}"
        },
        {
          "op": "setStrictChooseMode",
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
          "op": "assertPowerToughness",
          "player": 0,
          "name": "tester",
          "power": 1,
          "toughness": 1
        }
      ]
    },
    {
      "name": "testNextTurnTrue",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "tester",
          "custom": true,
          "typeLine": "Creature",
          "manaCost": "",
          "oracleText": "{0}: This creature gets +1/+1 until your next end step.",
          "fixtureSource": true,
          "power": "1",
          "toughness": "1"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "END_TURN",
          "player": 0,
          "ability": "{0}"
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
          "name": "tester",
          "power": 2,
          "toughness": 2
        }
      ]
    },
    {
      "name": "testNextTurnFalse",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "tester",
          "custom": true,
          "typeLine": "Creature",
          "manaCost": "",
          "oracleText": "{0}: This creature gets +1/+1 until your next end step.",
          "fixtureSource": true,
          "power": "1",
          "toughness": "1"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{0}"
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
          "name": "tester",
          "power": 1,
          "toughness": 1
        }
      ]
    },
    {
      "name": "testTurnCycleTrue",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "tester",
          "custom": true,
          "typeLine": "Creature",
          "manaCost": "",
          "oracleText": "{0}: This creature gets +1/+1 until your next end step.",
          "fixtureSource": true,
          "power": "1",
          "toughness": "1"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "END_TURN",
          "player": 0,
          "ability": "{0}"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "tester",
          "power": 2,
          "toughness": 2
        }
      ]
    },
    {
      "name": "testTurnCycleFalse",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "tester",
          "custom": true,
          "typeLine": "Creature",
          "manaCost": "",
          "oracleText": "{0}: This creature gets +1/+1 until your next end step.",
          "fixtureSource": true,
          "power": "1",
          "toughness": "1"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "END_TURN",
          "player": 0,
          "ability": "{0}"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "tester",
          "power": 1,
          "toughness": 1
        }
      ]
    },
    {
      "name": "testOpponentTurnTrue",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "tester",
          "custom": true,
          "typeLine": "Creature",
          "manaCost": "",
          "oracleText": "{0}: This creature gets +1/+1 until your next end step.",
          "fixtureSource": true,
          "power": "1",
          "toughness": "1"
        },
        {
          "op": "activateAbility",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{0}"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "tester",
          "power": 2,
          "toughness": 2
        }
      ]
    },
    {
      "name": "testOpponentTurnFalse",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "tester",
          "custom": true,
          "typeLine": "Creature",
          "manaCost": "",
          "oracleText": "{0}: This creature gets +1/+1 until your next end step.",
          "fixtureSource": true,
          "power": "1",
          "toughness": "1"
        },
        {
          "op": "activateAbility",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{0}"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "tester",
          "power": 1,
          "toughness": 1
        }
      ]
    }
  ]
});
