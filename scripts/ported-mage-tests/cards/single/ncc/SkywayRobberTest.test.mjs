import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/ncc/SkywayRobberTest.java",
  "tests": [
    {
      "name": "testNoOption",
      "operations": [
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Skyway Robber",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Mountain",
          "count": 5
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 4
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "Cast Skyway Robber with Escape"
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Skyway Robber",
          "defender": 1
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "END_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertExileCount",
          "player": 0,
          "name": "Mountain",
          "count": 5
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 17
        }
      ]
    },
    {
      "name": "testCast",
      "operations": [
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Skyway Robber",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Mountain",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Sol Ring",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 4
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "Cast Skyway Robber with Escape"
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Skyway Robber",
          "defender": 1
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
          "op": "assertExileCount",
          "player": 0,
          "name": "Mountain",
          "count": 4
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Sol Ring",
          "count": 1
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 17
        }
      ]
    }
  ]
});
