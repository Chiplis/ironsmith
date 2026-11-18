import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/continuous/GideonJuraAndRowanKenrithNextTurnTest.java",
  "tests": [
    {
      "name": "test_SingleOpponentMustAttackGideonJura",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Gideon Jura",
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
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "+2:",
          "target": 1
        },
        {
          "op": "assertCounterCount",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Gideon Jura",
          "counter": "LOYALTY",
          "count": 8,
          "seatIndependent": true
        },
        {
          "op": "assertCounterCount",
          "turn": 2,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Gideon Jura",
          "counter": "LOYALTY",
          "count": 6,
          "seatIndependent": true
        },
        {
          "op": "assertCounterCount",
          "turn": 3,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Gideon Jura",
          "counter": "LOYALTY",
          "count": 6,
          "seatIndependent": true
        },
        {
          "op": "assertCounterCount",
          "turn": 4,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Gideon Jura",
          "counter": "LOYALTY",
          "count": 6,
          "seatIndependent": true
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
    },
    {
      "name": "test_SingleOpponentMustAttackRowanKenrith",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Rowan Kenrith",
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
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "+2:",
          "target": 1
        },
        {
          "op": "addTarget",
          "player": 1,
          "target": 0
        },
        {
          "op": "unsupported",
          "source": "checkLife(\"turn 1\", 1, PhaseStep.POSTCOMBAT_MAIN, playerA, 20)"
        },
        {
          "op": "unsupported",
          "source": "checkLife(\"turn 2\", 2, PhaseStep.POSTCOMBAT_MAIN, playerA, 20 - 2)"
        },
        {
          "op": "unsupported",
          "source": "checkLife(\"turn 3\", 3, PhaseStep.POSTCOMBAT_MAIN, playerA, 20 - 2)"
        },
        {
          "op": "unsupported",
          "source": "checkLife(\"turn 4\", 4, PhaseStep.POSTCOMBAT_MAIN, playerA, 20 - 2)"
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
