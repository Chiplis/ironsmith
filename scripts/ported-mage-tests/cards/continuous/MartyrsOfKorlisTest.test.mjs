import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/continuous/MartyrsOfKorlisTest.java",
  "tests": [
    {
      "name": "test_PreventDamageToGideonOnYourTurn",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Martyrs of Korlis",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Alloy Myr",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "checkDamage(\"turn 1 before\", 1, PhaseStep.PRECOMBAT_MAIN, playerB, \"Martyrs of Korlis\", 0)"
        },
        {
          "op": "unsupported",
          "source": "checkLife(\"turn 1 before\", 1, PhaseStep.PRECOMBAT_MAIN, playerB, 20)"
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Alloy Myr",
          "defender": 1
        },
        {
          "op": "unsupported",
          "source": "checkDamage(\"turn 1 after\", 1, PhaseStep.POSTCOMBAT_MAIN, playerB, \"Martyrs of Korlis\", 2)"
        },
        {
          "op": "unsupported",
          "source": "checkLife(\"turn 1 after\", 1, PhaseStep.POSTCOMBAT_MAIN, playerB, 20)"
        },
        {
          "op": "attack",
          "turn": 2,
          "player": 1,
          "attacker": "Martyrs of Korlis",
          "defender": 0
        },
        {
          "op": "unsupported",
          "source": "checkDamage(\"turn 3 before\", 3, PhaseStep.PRECOMBAT_MAIN, playerB, \"Martyrs of Korlis\", 0)"
        },
        {
          "op": "unsupported",
          "source": "checkLife(\"turn 3 before\", 3, PhaseStep.PRECOMBAT_MAIN, playerB, 20)"
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Alloy Myr",
          "defender": 1
        },
        {
          "op": "unsupported",
          "source": "checkDamage(\"turn 3 after\", 3, PhaseStep.POSTCOMBAT_MAIN, playerB, \"Martyrs of Korlis\", 0)"
        },
        {
          "op": "unsupported",
          "source": "checkLife(\"turn 3 after\", 3, PhaseStep.POSTCOMBAT_MAIN, playerB, 20 - 2)"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        }
      ]
    }
  ]
});
