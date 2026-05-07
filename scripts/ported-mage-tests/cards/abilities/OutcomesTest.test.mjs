import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/abilities/OutcomesTest.java",
  "tests": [
    {
      "name": "test_FromEffects_Single",
      "operations": []
    },
    {
      "name": "test_FromEffects_Multi",
      "operations": []
    },
    {
      "name": "test_FromEffects_MultiCombine",
      "operations": []
    },
    {
      "name": "test_FromEffects_Default",
      "operations": []
    },
    {
      "name": "test_FromAbility_Single",
      "operations": []
    },
    {
      "name": "test_FromAbility_Multi",
      "operations": []
    },
    {
      "name": "test_FromAbility_MultiCombine",
      "operations": []
    }
  ]
});
