import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/triggers/dies/SilumgarScavengerTest.java",
  "tests": [
    {
      "name": "test_BoostOnDies",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Silumgar Scavenger",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 5
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Balduvian Bears",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Silumgar Scavenger"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Balduvian Bears"
        },
        {
          "op": "unsupported",
          "source": "checkPermanentCounters(\"boost\", 1, PhaseStep.BEGIN_COMBAT, playerA, \"Silumgar Scavenger\", CounterType.P1P1, 1)"
        },
        {
          "op": "assertAbility",
          "player": 1,
          "name": "BEGIN_COMBAT",
          "ability": 0,
          "expected": "Silumgar Scavenger"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "END_TURN"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "execute"
        }
      ]
    }
  ]
});
