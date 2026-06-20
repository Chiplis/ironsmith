import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/continuous/WallOfDustNextTurnTest.java",
  "tests": [
    {
      "name": "test_SingleOpponentMustAttack",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Wall of Dust",
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
          "name": "Ashcoat Bear",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "addCustomCardWithAbility(\"all attacks\", playerA, ability)"
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
          "name": "Ashcoat Bear",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "checkLife(\"turn 1\", 1, PhaseStep.POSTCOMBAT_MAIN, playerA, 20)"
        },
        {
          "op": "block",
          "turn": 2,
          "player": 0,
          "blocker": "Wall of Dust",
          "attacker": "Balduvian Bears"
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
          "name": "Ashcoat Bear",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "checkLife(\"turn 2\", 2, PhaseStep.POSTCOMBAT_MAIN, playerA, 20 - 2)"
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
          "name": "Ashcoat Bear",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "checkLife(\"turn 3\", 3, PhaseStep.POSTCOMBAT_MAIN, playerA, 20 - 2)"
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
          "name": "Ashcoat Bear",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "checkLife(\"turn 4\", 4, PhaseStep.POSTCOMBAT_MAIN, playerA, 20 - 2 * 2)"
        },
        {
          "op": "assertPermanentCount",
          "turn": 5,
          "phase": "POSTCOMBAT_MAIN",
          "player": 1,
          "name": "Balduvian Bears",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "turn": 5,
          "phase": "POSTCOMBAT_MAIN",
          "player": 1,
          "name": "Ashcoat Bear",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "checkLife(\"turn 5\", 5, PhaseStep.POSTCOMBAT_MAIN, playerA, 20 - 2 * 2)"
        },
        {
          "op": "assertPermanentCount",
          "turn": 6,
          "phase": "POSTCOMBAT_MAIN",
          "player": 1,
          "name": "Balduvian Bears",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "turn": 6,
          "phase": "POSTCOMBAT_MAIN",
          "player": 1,
          "name": "Ashcoat Bear",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "checkLife(\"turn 6\", 6, PhaseStep.POSTCOMBAT_MAIN, playerA, 20 - 2 * 2 - 2 * 2)"
        },
        {
          "op": "setStopAt",
          "turn": 6,
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
