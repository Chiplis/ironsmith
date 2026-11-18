import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/dis/AzoriusAethermageTest.java",
  "tests": [
    {
      "name": "testBouncedLand",
      "operations": [],
      "skip": "upstream body is entirely commented out"
    }
  ]
});
