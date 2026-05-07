import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/ncc/JoleneThePlunderQueenTest.java",
  "tests": [
    {
      "name": "testAttackingTwoOpponents",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Jolene, the Plunder Queen",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Elite Vanguard",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Jolene, the Plunder Queen",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Elite Vanguard",
          "defender": "playerC"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "DECLARE_BLOCKERS"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Treasure Token",
          "count": 2
        }
      ]
    },
    {
      "name": "testAttackingJoleneAndAnotherOpponent",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Elite Vanguard",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Balduvian Bears",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Jolene, the Plunder Queen",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Balduvian Bears",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Elite Vanguard",
          "defender": "playerC"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "DECLARE_BLOCKERS"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Treasure Token",
          "count": 1
        }
      ]
    },
    {
      "name": "testAttackingJoleneOnly",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Elite Vanguard",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Jolene, the Plunder Queen",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Elite Vanguard",
          "defender": 1
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "DECLARE_BLOCKERS"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Treasure Token",
          "count": 0
        }
      ]
    },
    {
      "name": "testEveryoneGotAJolene",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Elite Vanguard",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Jolene, the Plunder Queen",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Jolene, the Plunder Queen",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": "playerC",
          "name": "Jolene, the Plunder Queen",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Elite Vanguard",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Jolene, the Plunder Queen",
          "defender": "playerC"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "DECLARE_BLOCKERS"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Treasure Token",
          "count": 6
        }
      ]
    }
  ]
});
