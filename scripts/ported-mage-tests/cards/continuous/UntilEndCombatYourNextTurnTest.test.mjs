import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/continuous/UntilEndCombatYourNextTurnTest.java",
  "tests": [
    {
      "name": "testSameTurnPre",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "tester",
          "custom": true,
          "typeLine": "Creature",
          "manaCost": "",
          "oracleText": "{0}: This creature gets +1/+1 until end of combat on your next turn.",
          "fixtureSource": true,
          "power": "1",
          "toughness": "1"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "UPKEEP",
          "player": 0,
          "source": "tester",
          "ability": "{0}:"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "UPKEEP",
          "player": 0,
          "resolutions": 1
        },
        {
          "op": "assertPowerToughness",
          "turn": 1,
          "phase": "UPKEEP",
          "player": 0,
          "name": "tester",
          "power": 2,
          "toughness": 2
        },
        {
          "op": "setStopAt",
          "turn": 1,
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
      "name": "testSameTurnPost",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "tester",
          "custom": true,
          "typeLine": "Creature",
          "manaCost": "",
          "oracleText": "{0}: This creature gets +1/+1 until end of combat on your next turn.",
          "fixtureSource": true,
          "power": "1",
          "toughness": "1"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "UPKEEP",
          "player": 0,
          "source": "tester",
          "ability": "{0}:"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "UPKEEP",
          "player": 0,
          "resolutions": 1
        },
        {
          "op": "assertPowerToughness",
          "turn": 1,
          "phase": "UPKEEP",
          "player": 0,
          "name": "tester",
          "power": 2,
          "toughness": 2
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
      "name": "testOppTurnPre",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "tester",
          "custom": true,
          "typeLine": "Creature",
          "manaCost": "",
          "oracleText": "{0}: This creature gets +1/+1 until end of combat on your next turn.",
          "fixtureSource": true,
          "power": "1",
          "toughness": "1"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "UPKEEP",
          "player": 0,
          "source": "tester",
          "ability": "{0}:"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "UPKEEP",
          "player": 0,
          "resolutions": 1
        },
        {
          "op": "assertPowerToughness",
          "turn": 1,
          "phase": "UPKEEP",
          "player": 0,
          "name": "tester",
          "power": 2,
          "toughness": 2
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
      "name": "testOppTurnPost",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "tester",
          "custom": true,
          "typeLine": "Creature",
          "manaCost": "",
          "oracleText": "{0}: This creature gets +1/+1 until end of combat on your next turn.",
          "fixtureSource": true,
          "power": "1",
          "toughness": "1"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "UPKEEP",
          "player": 0,
          "source": "tester",
          "ability": "{0}:"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "UPKEEP",
          "player": 0,
          "resolutions": 1
        },
        {
          "op": "assertPowerToughness",
          "turn": 1,
          "phase": "UPKEEP",
          "player": 0,
          "name": "tester",
          "power": 2,
          "toughness": 2
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
      "name": "testTurnCyclePre",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "tester",
          "custom": true,
          "typeLine": "Creature",
          "manaCost": "",
          "oracleText": "{0}: This creature gets +1/+1 until end of combat on your next turn.",
          "fixtureSource": true,
          "power": "1",
          "toughness": "1"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "UPKEEP",
          "player": 0,
          "source": "tester",
          "ability": "{0}:"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "UPKEEP",
          "player": 0,
          "resolutions": 1
        },
        {
          "op": "assertPowerToughness",
          "turn": 1,
          "phase": "UPKEEP",
          "player": 0,
          "name": "tester",
          "power": 2,
          "toughness": 2
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
          "oracleText": "{0}: This creature gets +1/+1 until end of combat on your next turn.",
          "fixtureSource": true,
          "power": "1",
          "toughness": "1"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "UPKEEP",
          "player": 0,
          "source": "tester",
          "ability": "{0}:"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "UPKEEP",
          "player": 0,
          "resolutions": 1
        },
        {
          "op": "assertPowerToughness",
          "turn": 1,
          "phase": "UPKEEP",
          "player": 0,
          "name": "tester",
          "power": 2,
          "toughness": 2
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
          "op": "assertPowerToughness",
          "player": 0,
          "name": "tester",
          "power": 1,
          "toughness": 1
        }
      ]
    },
    {
      "name": "testTimeStopTurnCyclePre",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "tester",
          "custom": true,
          "typeLine": "Creature",
          "manaCost": "",
          "oracleText": "{0}: This creature gets +1/+1 until end of combat on your next turn.",
          "fixtureSource": true,
          "power": "1",
          "toughness": "1"
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Time Stop"
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 6
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "UPKEEP",
          "player": 0,
          "source": "tester",
          "ability": "{0}:"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "UPKEEP",
          "player": 0,
          "resolutions": 1
        },
        {
          "op": "assertPowerToughness",
          "turn": 1,
          "phase": "UPKEEP",
          "player": 0,
          "name": "tester",
          "power": 2,
          "toughness": 2
        },
        {
          "op": "castSpell",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Time Stop"
        },
        {
          "op": "assertStackObject",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "text": "Time Stop",
          "count": 1
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
      "name": "testTimeStopTurnCycleFalse",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "tester",
          "custom": true,
          "typeLine": "Creature",
          "manaCost": "",
          "oracleText": "{0}: This creature gets +1/+1 until end of combat on your next turn.",
          "fixtureSource": true,
          "power": "1",
          "toughness": "1"
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Time Stop"
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 6
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "UPKEEP",
          "player": 0,
          "source": "tester",
          "ability": "{0}:"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "UPKEEP",
          "player": 0,
          "resolutions": 1
        },
        {
          "op": "assertPowerToughness",
          "turn": 1,
          "phase": "UPKEEP",
          "player": 0,
          "name": "tester",
          "power": 2,
          "toughness": 2
        },
        {
          "op": "castSpell",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Time Stop"
        },
        {
          "op": "assertStackObject",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "text": "Time Stop",
          "count": 1
        },
        {
          "op": "waitStackResolved",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "resolutions": 1
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "CLEANUP"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertExileCount",
          "player": 0,
          "name": "Time Stop",
          "count": 1
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
      "name": "testTimeStop2TurnCyclePre",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "tester",
          "custom": true,
          "typeLine": "Creature",
          "manaCost": "",
          "oracleText": "{0}: This creature gets +1/+1 until end of combat on your next turn.",
          "fixtureSource": true,
          "power": "1",
          "toughness": "1"
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Time Stop"
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 6
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "UPKEEP",
          "player": 0,
          "source": "tester",
          "ability": "{0}:"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "UPKEEP",
          "player": 0,
          "resolutions": 1
        },
        {
          "op": "assertPowerToughness",
          "turn": 1,
          "phase": "UPKEEP",
          "player": 0,
          "name": "tester",
          "power": 2,
          "toughness": 2
        },
        {
          "op": "castSpell",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Time Stop"
        },
        {
          "op": "assertStackObject",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "text": "Time Stop",
          "count": 1
        },
        {
          "op": "waitStackResolved",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "resolutions": 1
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
          "op": "assertExileCount",
          "player": 0,
          "name": "Time Stop",
          "count": 1
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
      "name": "testTimeStop2TurnCycleFalse",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "tester",
          "custom": true,
          "typeLine": "Creature",
          "manaCost": "",
          "oracleText": "{0}: This creature gets +1/+1 until end of combat on your next turn.",
          "fixtureSource": true,
          "power": "1",
          "toughness": "1"
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Time Stop"
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 6
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "UPKEEP",
          "player": 0,
          "source": "tester",
          "ability": "{0}:"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "UPKEEP",
          "player": 0,
          "resolutions": 1
        },
        {
          "op": "assertPowerToughness",
          "turn": 1,
          "phase": "UPKEEP",
          "player": 0,
          "name": "tester",
          "power": 2,
          "toughness": 2
        },
        {
          "op": "castSpell",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Time Stop"
        },
        {
          "op": "assertStackObject",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "text": "Time Stop",
          "count": 1
        },
        {
          "op": "waitStackResolved",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "resolutions": 1
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
          "op": "assertExileCount",
          "player": 0,
          "name": "Time Stop",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "tester",
          "power": 2,
          "toughness": 2
        }
      ]
    }
  ]
});
