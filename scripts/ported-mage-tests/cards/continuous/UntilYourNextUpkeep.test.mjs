import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/continuous/UntilYourNextUpkeep.java",
  "tests": [
    {
      "name": "testSameTurn",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "tester",
          "custom": true,
          "typeLine": "Creature",
          "manaCost": "",
          "oracleText": "{0}: Until your next upkeep, this creature gets +1/+1.",
          "fixtureSource": true,
          "power": "1",
          "toughness": "1"
        },
        {
          "op": "sequence",
          "operations": [
            {
              "op": "activateAbility",
              "turn": 1,
              "phase": "UPKEEP",
              "player": 0,
              "ability": "{0}"
            }
          ]
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
      "name": "testOppTurn",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "tester",
          "custom": true,
          "typeLine": "Creature",
          "manaCost": "",
          "oracleText": "{0}: Until your next upkeep, this creature gets +1/+1.",
          "fixtureSource": true,
          "power": "1",
          "toughness": "1"
        },
        {
          "op": "sequence",
          "operations": [
            {
              "op": "activateAbility",
              "turn": 1,
              "phase": "UPKEEP",
              "player": 0,
              "ability": "{0}"
            }
          ]
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
      "name": "testTurnCycle",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "tester",
          "custom": true,
          "typeLine": "Creature",
          "manaCost": "",
          "oracleText": "{0}: Until your next upkeep, this creature gets +1/+1.",
          "fixtureSource": true,
          "power": "1",
          "toughness": "1"
        },
        {
          "op": "sequence",
          "operations": [
            {
              "op": "activateAbility",
              "turn": 1,
              "phase": "UPKEEP",
              "player": 0,
              "ability": "{0}"
            }
          ]
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
          "power": 1,
          "toughness": 1
        }
      ]
    },
    {
      "name": "testParadoxHazeOppSameTurn",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "tester",
          "custom": true,
          "typeLine": "Creature",
          "manaCost": "",
          "oracleText": "{0}: Until your next upkeep, this creature gets +1/+1.",
          "fixtureSource": true,
          "power": "1",
          "toughness": "1"
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Paradox Haze"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Paradox Haze",
          "target": 0
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "resolutions": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Paradox Haze",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 2,
          "phase": "UPKEEP",
          "player": 0,
          "source": "tester",
          "ability": "{0}"
        },
        {
          "op": "waitStackResolved",
          "turn": 2,
          "phase": "UPKEEP",
          "player": 0,
          "resolutions": 1
        },
        {
          "op": "assertPowerToughness",
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
      "name": "testParadoxHazeSameTurn",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "tester",
          "custom": true,
          "typeLine": "Creature",
          "manaCost": "",
          "oracleText": "{0}: Until your next upkeep, this creature gets +1/+1.",
          "fixtureSource": true,
          "power": "1",
          "toughness": "1"
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Paradox Haze"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Paradox Haze",
          "target": 0
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "resolutions": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Paradox Haze",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 3,
          "phase": "UPKEEP",
          "player": 0,
          "source": "tester",
          "ability": "{0}"
        },
        {
          "op": "waitStackResolved",
          "turn": 3,
          "phase": "UPKEEP",
          "player": 0,
          "resolutions": 1
        },
        {
          "op": "assertPowerToughness",
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
          "power": 1,
          "toughness": 1
        }
      ]
    },
    {
      "name": "testEonHubSameTurn",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "tester",
          "custom": true,
          "typeLine": "Creature",
          "manaCost": "",
          "oracleText": "{0}: Until your next upkeep, this creature gets +1/+1.",
          "fixtureSource": true,
          "power": "1",
          "toughness": "1"
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Eon Hub"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "source": "tester",
          "ability": "{0}"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "resolutions": 1
        },
        {
          "op": "assertPowerToughness",
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
      "name": "testEonHubCycleTurn",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "tester",
          "custom": true,
          "typeLine": "Creature",
          "manaCost": "",
          "oracleText": "{0}: Until your next upkeep, this creature gets +1/+1.",
          "fixtureSource": true,
          "power": "1",
          "toughness": "1"
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Eon Hub"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "source": "tester",
          "ability": "{0}"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "resolutions": 1
        },
        {
          "op": "assertPowerToughness",
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
          "power": 2,
          "toughness": 2
        }
      ]
    }
  ]
});
