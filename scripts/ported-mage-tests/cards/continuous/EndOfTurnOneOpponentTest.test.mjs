import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/continuous/EndOfTurnOneOpponentTest.java",
  "tests": [
    {
      "name": "test_EndOfTurnSingle",
      "operations": [
        {
          "op": "addCard",
          "zone": "boost1",
          "player": 0,
          "name": "ALL",
          "custom": true,
          "oracleText": ""
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
          "name": "Balduvian Bears",
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
          "turn": 2,
          "player": 1,
          "attacker": "Balduvian Bears",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Balduvian Bears",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 4,
          "player": 1,
          "attacker": "Balduvian Bears",
          "defender": 1
        },
        {
          "op": "setStopAt",
          "turn": 4,
          "phase": "CLEANUP"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "execute"
        }
      ]
    },
    {
      "name": "test_UntilYourNextTurnSingle",
      "operations": [
        {
          "op": "addCard",
          "zone": "boost1",
          "player": 0,
          "name": "ALL",
          "custom": true,
          "oracleText": ""
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
          "name": "Balduvian Bears",
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
          "turn": 2,
          "player": 1,
          "attacker": "Balduvian Bears",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Balduvian Bears",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 4,
          "player": 1,
          "attacker": "Balduvian Bears",
          "defender": 1
        },
        {
          "op": "setStopAt",
          "turn": 4,
          "phase": "CLEANUP"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "execute"
        }
      ]
    },
    {
      "name": "test_UntilEndOfYourNextTurnSingle",
      "operations": [
        {
          "op": "addCard",
          "zone": "boost1",
          "player": 0,
          "name": "ALL",
          "custom": true,
          "oracleText": ""
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
          "name": "Balduvian Bears",
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
          "turn": 2,
          "player": 1,
          "attacker": "Balduvian Bears",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Balduvian Bears",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 4,
          "player": 1,
          "attacker": "Balduvian Bears",
          "defender": 1
        },
        {
          "op": "setStopAt",
          "turn": 4,
          "phase": "CLEANUP"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "execute"
        }
      ]
    }
  ]
});
