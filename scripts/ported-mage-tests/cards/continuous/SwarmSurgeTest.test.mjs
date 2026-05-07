import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/continuous/SwarmSurgeTest.java",
  "tests": [
    {
      "name": "testSwarmSurge",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 9
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Birthing Hulk",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Swarm Surge",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Birthing Hulk"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Swarm Surge"
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Silvercoat Lion",
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
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Swarm Surge",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Birthing Hulk",
          "power": 7,
          "toughness": 4
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Birthing Hulk",
          "ability": "FirstStrike",
          "expected": true
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Eldrazi Scion Token",
          "power": 3,
          "toughness": 1
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Eldrazi Scion Token",
          "ability": "FirstStrike",
          "expected": true
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Silvercoat Lion",
          "power": 4,
          "toughness": 2
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Silvercoat Lion",
          "ability": "FirstStrike",
          "expected": false
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 20
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 16
        }
      ]
    }
  ]
});
