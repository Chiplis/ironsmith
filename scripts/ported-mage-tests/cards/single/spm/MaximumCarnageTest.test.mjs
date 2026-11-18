import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/spm/MaximumCarnageTest.java",
  "tests": [
    {
      "name": "testMaximumCarnage",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Maximum Carnage",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Bear Cub",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": "playerD",
          "name": "Bear Cub",
          "count": 1
        },
        {
          "op": "addTarget",
          "player": "playerD",
          "target": "playerC"
        },
        {
          "op": "addTarget",
          "player": 1,
          "target": "playerD"
        },
        {
          "op": "setStopAt",
          "turn": 4,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        }
      ]
    },
    {
      "name": "testMaximumCarnageCantAttackController",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Maximum Carnage",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Bear Cub",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": "playerD",
          "name": "Bear Cub",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 5
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Maximum Carnage",
          "resolve": true
        },
        {
          "op": "attack",
          "turn": 2,
          "phase": "DECLARE_ATTACKERS",
          "player": 1,
          "attacker": "Bear Cub",
          "defender": 3,
          "expectedLegal": true
        },
        {
          "op": "attack",
          "turn": 4,
          "phase": "DECLARE_ATTACKERS",
          "player": 3,
          "attacker": "Bear Cub",
          "defender": 2,
          "expectedLegal": true,
          "rejectedDefender": 0,
          "rejection": "attack requirement|illegal attack|invalid attackers|cannot attack"
        },
        {
          "op": "setStopAt",
          "turn": 4,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertLife",
          "player": 2,
          "life": 18
        }
      ]
    }
  ]
});
