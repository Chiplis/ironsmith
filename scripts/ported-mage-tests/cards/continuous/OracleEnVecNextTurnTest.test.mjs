import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/continuous/OracleEnVecNextTurnTest.java",
  "tests": [
    {
      "name": "test_SingleOpponentMustAttack",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Oracle en-Vec",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Balduvian Bears",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Angelic Wall",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Ashcoat Bear",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{T}: Target opponent",
          "target": 1
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": "Balduvian Bears"
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": "Angelic Wall"
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": "TestPlayer.CHOICE_SKIP"
        },
        {
          "op": "unsupported",
          "source": "checkLife(\"turn 1\", 1, PhaseStep.POSTCOMBAT_MAIN, playerA, 20)"
        },
        {
          "op": "assertPermanentCount",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 1,
          "name": "Balduvian Bears",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 1,
          "name": "Angelic Wall",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 1,
          "name": "Ashcoat Bear",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "checkLife(\"turn 2\", 2, PhaseStep.POSTCOMBAT_MAIN, playerA, 20 - 2)"
        },
        {
          "op": "assertPermanentCount",
          "turn": 2,
          "phase": "POSTCOMBAT_MAIN",
          "player": 1,
          "name": "Balduvian Bears",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "turn": 2,
          "phase": "POSTCOMBAT_MAIN",
          "player": 1,
          "name": "Angelic Wall",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "turn": 2,
          "phase": "POSTCOMBAT_MAIN",
          "player": 1,
          "name": "Ashcoat Bear",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "checkLife(\"turn 3\", 3, PhaseStep.POSTCOMBAT_MAIN, playerA, 20 - 2)"
        },
        {
          "op": "assertPermanentCount",
          "turn": 3,
          "phase": "POSTCOMBAT_MAIN",
          "player": 1,
          "name": "Balduvian Bears",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "turn": 3,
          "phase": "POSTCOMBAT_MAIN",
          "player": 1,
          "name": "Angelic Wall",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "turn": 3,
          "phase": "POSTCOMBAT_MAIN",
          "player": 1,
          "name": "Ashcoat Bear",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "checkLife(\"turn 4\", 4, PhaseStep.POSTCOMBAT_MAIN, playerA, 20 - 2)"
        },
        {
          "op": "assertPermanentCount",
          "turn": 4,
          "phase": "POSTCOMBAT_MAIN",
          "player": 1,
          "name": "Balduvian Bears",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "turn": 4,
          "phase": "POSTCOMBAT_MAIN",
          "player": 1,
          "name": "Angelic Wall",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "turn": 4,
          "phase": "POSTCOMBAT_MAIN",
          "player": 1,
          "name": "Ashcoat Bear",
          "count": 1
        },
        {
          "op": "setStopAt",
          "turn": 4,
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
