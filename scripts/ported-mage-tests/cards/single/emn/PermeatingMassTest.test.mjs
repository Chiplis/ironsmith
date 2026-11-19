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
          "name": "Hill Giant@giant",
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
          "op": "assertStackObject",
          "turn": 1,
          "phase": "COMBAT_DAMAGE",
          "source": "Permeating Mass",
          "count": 1,
          "abilityKind": "Triggered",
          "completeChoices": true
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Permeating Mass",
          "count": 1
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "COMBAT_DAMAGE"
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
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "@giant",
          "count": 1
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
          "defender": 0,
          "expectedLegal": true
        },
        {
          "op": "block",
          "turn": 2,
          "player": 0,
          "blocker": "Permeating Mass",
          "attacker": "Dungrove Elder"
        },
        {
          "op": "assertStackObject",
          "turn": 2,
          "phase": "COMBAT_DAMAGE",
          "source": "Permeating Mass",
          "count": 1,
          "abilityKind": "Triggered"
        },
        {
          "op": "assertGraveyardCount",
          "name": "Permeating Mass",
          "player": 0,
          "count": 1
        },
        {
          "op": "assertDamageReceived",
          "name": "Dungrove Elder",
          "player": 1,
          "damage": 1,
          "seatIndependent": true
        },
        {
          "op": "waitStackResolved",
          "turn": 2,
          "phase": "COMBAT_DAMAGE",
          "resolutions": 1
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
