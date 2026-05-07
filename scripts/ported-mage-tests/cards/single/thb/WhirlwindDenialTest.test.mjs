import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/thb/WhirlwindDenialTest.java",
  "tests": [
    {
      "name": "testWhirlwindPayAllCosts",
      "operations": []
    },
    {
      "name": "testWhirlwindPayTrigger",
      "operations": []
    },
    {
      "name": "testWhirlwindPaySpell",
      "operations": []
    },
    {
      "name": "testWhirlwindPayNone",
      "operations": []
    }
  ]
});
