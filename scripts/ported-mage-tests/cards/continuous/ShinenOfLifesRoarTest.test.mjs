import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/continuous/ShinenOfLifesRoarTest.java",
  "tests": [
    {
      "name": "test_SingleOpponentMustBlock",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Shinen of Life's Roar",
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
          "attacker": "Shinen of Life's Roar",
          "defender": 1
        },
        {
          "op": "assertBlockDeclarationRejected",
          "turn": 1,
          "player": 1,
          "reason": "must block",
          "declarations": []
        },
        {
          "op": "block",
          "turn": 1,
          "player": 1,
          "blocker": "Balduvian Bears",
          "attacker": "Shinen of Life's Roar"
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "END_TURN"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "execute"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Shinen of Life's Roar",
          "count": 1
        }
      ]
    }
  ]
});
