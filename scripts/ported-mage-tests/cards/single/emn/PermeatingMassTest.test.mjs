import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/emn/PermeatingMassTest.java",
  "tests": [
    {
      "name": "testWhenDiesInCombatMakesCopyStill",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Hill Giant",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Permeating Mass",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Hill Giant",
          "defender": 1
        },
        {
          "op": "block",
          "turn": 1,
          "player": 1,
          "blocker": "Permeating Mass",
          "attacker": "Hill Giant"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "END_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 20
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Permeating Mass",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Permeating Mass",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Permeating Mass",
          "power": 1,
          "toughness": 3
        }
      ]
    },
    {
      "name": "damagedCreatureWithVaryingPTbecomesCopyOfPermeatingMass",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Permeating Mass",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Forest",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Dungrove Elder",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 2,
          "player": 1,
          "attacker": "Dungrove Elder",
          "defender": 1
        },
        {
          "op": "block",
          "turn": 2,
          "player": 0,
          "blocker": "Permeating Mass",
          "attacker": "Dungrove Elder"
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "END_COMBAT"
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
          "life": 20
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Permeating Mass",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Permeating Mass",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 1,
          "name": "Permeating Mass",
          "power": 1,
          "toughness": 3
        }
      ]
    }
  ]
});
