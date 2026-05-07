import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/continuous/UntilYourNextUpkeep.java",
  "tests": [
    {
      "name": "testSameTurn",
      "operations": []
    },
    {
      "name": "testOppTurn",
      "operations": []
    },
    {
      "name": "testTurnCycle",
      "operations": []
    },
    {
      "name": "testParadoxHazeOppSameTurn",
      "operations": []
    },
    {
      "name": "testParadoxHazeSameTurn",
      "operations": []
    },
    {
      "name": "testEonHubSameTurn",
      "operations": []
    },
    {
      "name": "testEonHubCycleTurn",
      "operations": []
    }
  ]
});
