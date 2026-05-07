import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/continuous/BecomesColorEffectTest.java",
  "tests": [
    {
      "name": "testBecomesColorSource",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Ancient Kavu",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 2
        },
        {
          "op": "unsupported",
          "source": "checkColor(\"Ancient Kavu is red\", 1, PhaseStep.PRECOMBAT_MAIN, playerA, kavu, \"R\", true)"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{2}: {this}"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "unsupported",
          "source": "checkColor(\"Ancient Kavu is colorless\", 1, PhaseStep.PRECOMBAT_MAIN, playerA, kavu, \"C\", true)"
        },
        {
          "op": "unsupported",
          "source": "checkColor(\"Ancient Kavu is red again\", 2, PhaseStep.PRECOMBAT_MAIN, playerA, kavu, \"R\", true)"
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "execute"
        }
      ]
    },
    {
      "name": "testBecomesColorTarget",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Ancient Kavu",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Alchor's Tomb",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "checkColor(\"Ancient Kavu is red\", 1, PhaseStep.PRECOMBAT_MAIN, playerA, kavu, \"R\", true)"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{2}, {T}: Target permanent",
          "target": "Ancient Kavu"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Green"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "unsupported",
          "source": "checkColor(\"Ancient Kavu is green\", 1, PhaseStep.PRECOMBAT_MAIN, playerA, kavu, \"G\", true)"
        },
        {
          "op": "unsupported",
          "source": "checkColor(\"Ancient Kavu is still green the following turn\", 2, PhaseStep.PRECOMBAT_MAIN, playerA, kavu, \"G\", true)"
        },
        {
          "op": "activateAbility",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{2}: {this}"
        },
        {
          "op": "unsupported",
          "source": "checkColor(\"Ancient Kavu is colorless\", 3, PhaseStep.POSTCOMBAT_MAIN, playerA, kavu, \"C\", true)"
        },
        {
          "op": "unsupported",
          "source": "checkColor(\"Ancient Kavu is green again\", 4, PhaseStep.PRECOMBAT_MAIN, playerA, kavu, \"G\", true)"
        },
        {
          "op": "setStopAt",
          "turn": 4,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "execute"
        }
      ]
    }
  ]
});
