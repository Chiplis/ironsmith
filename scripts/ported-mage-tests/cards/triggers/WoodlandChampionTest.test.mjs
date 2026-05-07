import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/triggers/WoodlandChampionTest.java",
  "tests": [
    {
      "name": "test_TriggerOnTwoTokens",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Woodland Champion",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Acorn Harvest",
          "count": 6
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 4
        },
        {
          "op": "unsupported",
          "source": "checkPermanentCounters(\"before\", 1, PhaseStep.PRECOMBAT_MAIN, playerA, \"Woodland Champion\", CounterType.P1P1, 0)"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Acorn Harvest"
        },
        {
          "op": "assertPermanentCount",
          "turn": 1,
          "phase": "BEGIN_COMBAT",
          "player": 0,
          "name": "Squirrel Token",
          "count": 2
        },
        {
          "op": "unsupported",
          "source": "checkPermanentCounters(\"after\", 1, PhaseStep.BEGIN_COMBAT, playerA, \"Woodland Champion\", CounterType.P1P1, 2)"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        }
      ]
    }
  ]
});
