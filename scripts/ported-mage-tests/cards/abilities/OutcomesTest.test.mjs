import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/abilities/OutcomesTest.java",
  "tests": [
    {
      "name": "test_FromEffects_Single",
      "skip": "Checks XMage AI outcome scores of Java effect classes"
    },
    {
      "name": "test_FromEffects_Multi",
      "skip": "Checks XMage AI score aggregation over Java effect classes"
    },
    {
      "name": "test_FromEffects_MultiCombine",
      "skip": "Checks XMage AI score aggregation over mixed Java effect classes"
    },
    {
      "name": "test_FromEffects_Default",
      "skip": "Checks XMage AI default outcome and fallback enum values"
    },
    {
      "name": "test_FromAbility_Single",
      "skip": "Checks XMage AI outcome overrides on Java ability classes"
    },
    {
      "name": "test_FromAbility_Multi",
      "skip": "Checks XMage AI outcome overrides over multiple Java effects"
    },
    {
      "name": "test_FromAbility_MultiCombine",
      "skip": "Checks XMage AI outcome override of mixed Java effects"
    }
  ]
});
