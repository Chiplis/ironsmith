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
          "target": 0
        },
        {
          "op": "setStopAt",
          "turn": 4,
          "phase": "END_TURN"
        },
        {
          "op": "unsupported",
          "source": "try { execute(); } catch (AssertionError e) { assertTrue(\"Shouldn't be able to attack playerA\", e.getMessage().contains(\"[targetPlayer=PlayerA], but not used\")); }"
        }
      ]
    }
  ]
});
