import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/continuous/UntilEndCombatYourNextTurnTest.java",
  "tests": [
    {
      "name": "testSameTurnPre",
      "operations": []
    },
    {
      "name": "testSameTurnPost",
      "operations": []
    },
    {
      "name": "testOppTurnPre",
      "operations": []
    },
    {
      "name": "testOppTurnPost",
      "operations": []
    },
    {
      "name": "testTurnCyclePre",
      "operations": []
    },
    {
      "name": "testTurnCycleFalse",
      "operations": []
    },
    {
      "name": "testTimeStopTurnCyclePre",
      "operations": []
    },
    {
      "name": "testTimeStopTurnCycleFalse",
      "operations": []
    },
    {
      "name": "testTimeStop2TurnCyclePre",
      "operations": []
    },
    {
      "name": "testTimeStop2TurnCycleFalse",
      "operations": []
    }
  ]
});
