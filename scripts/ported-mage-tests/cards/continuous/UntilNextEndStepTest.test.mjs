import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/continuous/UntilNextEndStepTest.java",
  "tests": [
    {
      "name": "testSameTurnTrue",
      "operations": []
    },
    {
      "name": "testSameTurnFalse",
      "operations": []
    },
    {
      "name": "testNextTurnTrue",
      "operations": []
    },
    {
      "name": "testNextTurnFalse",
      "operations": []
    },
    {
      "name": "testTurnCycleTrue",
      "operations": []
    },
    {
      "name": "testTurnCycleFalse",
      "operations": []
    },
    {
      "name": "testOpponentTurnTrue",
      "operations": []
    },
    {
      "name": "testOpponentTurnFalse",
      "operations": []
    }
  ]
});
