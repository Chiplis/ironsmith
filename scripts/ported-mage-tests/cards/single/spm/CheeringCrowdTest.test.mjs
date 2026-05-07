import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/spm/CheeringCrowdTest.java",
  "tests": [
    {
      "name": "testCheeringCrowd",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Cheering Crowd",
          "count": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": true
        },
        {
          "op": "setChoice",
          "player": "playerC",
          "value": true
        },
        {
          "op": "setChoice",
          "player": "playerD",
          "value": true
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0
        },
        {
          "op": "unsupported",
          "source": "checkManaPool(\"PlayerA should have 1 Mana\", 1, PhaseStep.PRECOMBAT_MAIN, playerA, \"C\", 1)"
        },
        {
          "op": "waitStackResolved",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": "playerD"
        },
        {
          "op": "unsupported",
          "source": "checkManaPool(\"PlayerD should have 2 Mana\", 2, PhaseStep.PRECOMBAT_MAIN, playerD, \"C\", 2)"
        },
        {
          "op": "waitStackResolved",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": "playerC"
        },
        {
          "op": "unsupported",
          "source": "checkManaPool(\"PlayerC should have 3 Mana\", 3, PhaseStep.PRECOMBAT_MAIN, playerC, \"C\", 3)"
        },
        {
          "op": "waitStackResolved",
          "turn": 4,
          "phase": "PRECOMBAT_MAIN",
          "player": 1
        },
        {
          "op": "unsupported",
          "source": "checkManaPool(\"PlayerB should have 4 Mana\", 4, PhaseStep.PRECOMBAT_MAIN, playerB, \"C\", 4)"
        },
        {
          "op": "setStopAt",
          "turn": 4,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Cheering Crowd",
          "counter": "P1P1",
          "count": 4
        }
      ]
    }
  ]
});
