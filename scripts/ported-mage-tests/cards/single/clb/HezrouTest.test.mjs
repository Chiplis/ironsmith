import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/clb/HezrouTest.java",
  "tests": [
    {
      "name": "testTrigger",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Kraken Hatchling",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Maritime Guard",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Fortress Crab",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Aegis Turtle",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Gloom Pangolin",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Wishcoin Crab",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Hezrou",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Kraken Hatchling",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Maritime Guard",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Fortress Crab",
          "defender": 1
        },
        {
          "op": "block",
          "turn": 1,
          "player": 1,
          "blocker": "Aegis Turtle",
          "attacker": "Kraken Hatchling"
        },
        {
          "op": "block",
          "turn": 1,
          "player": 1,
          "blocker": "Gloom Pangolin",
          "attacker": "Maritime Guard"
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
          "op": "assertLife",
          "player": 0,
          "life": 20
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 19
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Kraken Hatchling",
          "power": 0,
          "toughness": 4
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Maritime Guard",
          "power": 1,
          "toughness": 3
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Fortress Crab",
          "power": 1,
          "toughness": 6
        },
        {
          "op": "assertPowerToughness",
          "player": 1,
          "name": "Aegis Turtle",
          "power": -1,
          "toughness": 4
        },
        {
          "op": "assertPowerToughness",
          "player": 1,
          "name": "Gloom Pangolin",
          "power": 0,
          "toughness": 4
        },
        {
          "op": "assertPowerToughness",
          "player": 1,
          "name": "Wishcoin Crab",
          "power": 2,
          "toughness": 5
        }
      ]
    },
    {
      "name": "testAdventure",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Kraken Hatchling",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Maritime Guard",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Fortress Crab",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Aegis Turtle",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Gloom Pangolin",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Wishcoin Crab",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Hezrou",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Kraken Hatchling",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Maritime Guard",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Fortress Crab",
          "defender": 1
        },
        {
          "op": "block",
          "turn": 1,
          "player": 1,
          "blocker": "Aegis Turtle",
          "attacker": "Kraken Hatchling"
        },
        {
          "op": "block",
          "turn": 1,
          "player": 1,
          "blocker": "Gloom Pangolin",
          "attacker": "Maritime Guard"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "END_COMBAT",
          "player": 0,
          "name": "Demonic Stench"
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
          "op": "assertLife",
          "player": 0,
          "life": 20
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 19
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Kraken Hatchling",
          "power": 0,
          "toughness": 4
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Maritime Guard",
          "power": 1,
          "toughness": 3
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Fortress Crab",
          "power": 1,
          "toughness": 6
        },
        {
          "op": "assertPowerToughness",
          "player": 1,
          "name": "Aegis Turtle",
          "power": -1,
          "toughness": 4
        },
        {
          "op": "assertPowerToughness",
          "player": 1,
          "name": "Gloom Pangolin",
          "power": 0,
          "toughness": 4
        },
        {
          "op": "assertPowerToughness",
          "player": 1,
          "name": "Wishcoin Crab",
          "power": 2,
          "toughness": 5
        }
      ]
    }
  ]
});
