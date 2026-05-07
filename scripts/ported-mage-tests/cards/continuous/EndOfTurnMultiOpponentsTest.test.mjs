import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/continuous/EndOfTurnMultiOpponentsTest.java",
  "tests": [
    {
      "name": "test_EndOfTurnMulti",
      "operations": [
        {
          "op": "addCard",
          "zone": "boost1",
          "player": 0,
          "name": "ALL",
          "custom": true,
          "oracleText": ""
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "EndOfTurnOneOpponentTest.cardBear2",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "EndOfTurnOneOpponentTest.cardBear2",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": "playerC",
          "name": "EndOfTurnOneOpponentTest.cardBear2",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": "playerD",
          "name": "EndOfTurnOneOpponentTest.cardBear2",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "EndOfTurnOneOpponentTest.cardBear2",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 2,
          "player": "playerD",
          "attacker": "EndOfTurnOneOpponentTest.cardBear2",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 3,
          "player": "playerC",
          "attacker": "EndOfTurnOneOpponentTest.cardBear2",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 4,
          "player": 1,
          "attacker": "EndOfTurnOneOpponentTest.cardBear2",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 5,
          "player": 0,
          "attacker": "EndOfTurnOneOpponentTest.cardBear2",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 6,
          "player": "playerD",
          "attacker": "EndOfTurnOneOpponentTest.cardBear2",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 7,
          "player": "playerC",
          "attacker": "EndOfTurnOneOpponentTest.cardBear2",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 8,
          "player": 1,
          "attacker": "EndOfTurnOneOpponentTest.cardBear2",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 9,
          "player": 0,
          "attacker": "EndOfTurnOneOpponentTest.cardBear2",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 10,
          "player": "playerD",
          "attacker": "EndOfTurnOneOpponentTest.cardBear2",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 11,
          "player": "playerC",
          "attacker": "EndOfTurnOneOpponentTest.cardBear2",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 12,
          "player": 1,
          "attacker": "EndOfTurnOneOpponentTest.cardBear2",
          "defender": 1
        },
        {
          "op": "setStopAt",
          "turn": 12,
          "phase": "CLEANUP"
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
      "name": "test_UntilYourNextTurnMulti",
      "operations": [
        {
          "op": "addCard",
          "zone": "boost1",
          "player": 0,
          "name": "ALL",
          "custom": true,
          "oracleText": ""
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "EndOfTurnOneOpponentTest.cardBear2",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "EndOfTurnOneOpponentTest.cardBear2",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": "playerC",
          "name": "EndOfTurnOneOpponentTest.cardBear2",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": "playerD",
          "name": "EndOfTurnOneOpponentTest.cardBear2",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "EndOfTurnOneOpponentTest.cardBear2",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 2,
          "player": "playerD",
          "attacker": "EndOfTurnOneOpponentTest.cardBear2",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 3,
          "player": "playerC",
          "attacker": "EndOfTurnOneOpponentTest.cardBear2",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 4,
          "player": 1,
          "attacker": "EndOfTurnOneOpponentTest.cardBear2",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 5,
          "player": 0,
          "attacker": "EndOfTurnOneOpponentTest.cardBear2",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 6,
          "player": "playerD",
          "attacker": "EndOfTurnOneOpponentTest.cardBear2",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 7,
          "player": "playerC",
          "attacker": "EndOfTurnOneOpponentTest.cardBear2",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 8,
          "player": 1,
          "attacker": "EndOfTurnOneOpponentTest.cardBear2",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 9,
          "player": 0,
          "attacker": "EndOfTurnOneOpponentTest.cardBear2",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 10,
          "player": "playerD",
          "attacker": "EndOfTurnOneOpponentTest.cardBear2",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 11,
          "player": "playerC",
          "attacker": "EndOfTurnOneOpponentTest.cardBear2",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 12,
          "player": 1,
          "attacker": "EndOfTurnOneOpponentTest.cardBear2",
          "defender": 1
        },
        {
          "op": "setStopAt",
          "turn": 12,
          "phase": "CLEANUP"
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
      "name": "test_UntilEndOfYourNextTurnMulti",
      "operations": [
        {
          "op": "addCard",
          "zone": "boost1",
          "player": 0,
          "name": "ALL",
          "custom": true,
          "oracleText": ""
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "EndOfTurnOneOpponentTest.cardBear2",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "EndOfTurnOneOpponentTest.cardBear2",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": "playerC",
          "name": "EndOfTurnOneOpponentTest.cardBear2",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": "playerD",
          "name": "EndOfTurnOneOpponentTest.cardBear2",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "EndOfTurnOneOpponentTest.cardBear2",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 2,
          "player": "playerD",
          "attacker": "EndOfTurnOneOpponentTest.cardBear2",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 3,
          "player": "playerC",
          "attacker": "EndOfTurnOneOpponentTest.cardBear2",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 4,
          "player": 1,
          "attacker": "EndOfTurnOneOpponentTest.cardBear2",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 5,
          "player": 0,
          "attacker": "EndOfTurnOneOpponentTest.cardBear2",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 6,
          "player": "playerD",
          "attacker": "EndOfTurnOneOpponentTest.cardBear2",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 7,
          "player": "playerC",
          "attacker": "EndOfTurnOneOpponentTest.cardBear2",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 8,
          "player": 1,
          "attacker": "EndOfTurnOneOpponentTest.cardBear2",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 9,
          "player": 0,
          "attacker": "EndOfTurnOneOpponentTest.cardBear2",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 10,
          "player": "playerD",
          "attacker": "EndOfTurnOneOpponentTest.cardBear2",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 11,
          "player": "playerC",
          "attacker": "EndOfTurnOneOpponentTest.cardBear2",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 12,
          "player": 1,
          "attacker": "EndOfTurnOneOpponentTest.cardBear2",
          "defender": 1
        },
        {
          "op": "setStopAt",
          "turn": 12,
          "phase": "CLEANUP"
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
      "name": "test_UntilYourNextTurnMulti_Leaved",
      "operations": [
        {
          "op": "addCard",
          "zone": "boost1",
          "player": 0,
          "name": "ALL",
          "custom": true,
          "oracleText": ""
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "EndOfTurnOneOpponentTest.cardBear2",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "EndOfTurnOneOpponentTest.cardBear2",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": "playerC",
          "name": "EndOfTurnOneOpponentTest.cardBear2",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": "playerD",
          "name": "EndOfTurnOneOpponentTest.cardBear2",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": "playerC",
          "name": "Eye of Doom",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": "playerC",
          "name": "Forest",
          "count": 4
        },
        {
          "op": "unsupported",
          "source": "checkPlayerInGame(\"A must plays in 1\", 1, PhaseStep.PRECOMBAT_MAIN, playerA, playerA, true)"
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "EndOfTurnOneOpponentTest.cardBear2",
          "defender": 1
        },
        {
          "op": "unsupported",
          "source": "checkPlayerInGame(\"A must plays in 2\", 2, PhaseStep.PRECOMBAT_MAIN, playerD, playerA, true)"
        },
        {
          "op": "attack",
          "turn": 2,
          "player": "playerD",
          "attacker": "EndOfTurnOneOpponentTest.cardBear2",
          "defender": 1
        },
        {
          "op": "unsupported",
          "source": "checkPlayerInGame(\"A must plays in 3 before\", 3, PhaseStep.PRECOMBAT_MAIN, playerC, playerA, true)"
        },
        {
          "op": "attack",
          "turn": 3,
          "player": "playerC",
          "attacker": "EndOfTurnOneOpponentTest.cardBear2",
          "defender": 1
        },
        {
          "op": "unsupported",
          "source": "concede(3, PhaseStep.PRECOMBAT_MAIN, playerA)"
        },
        {
          "op": "unsupported",
          "source": "checkPlayerInGame(\"A must leaved in 3 after\", 3, PhaseStep.POSTCOMBAT_MAIN, playerC, playerA, false)"
        },
        {
          "op": "castSpell",
          "turn": 3,
          "phase": "POSTCOMBAT_MAIN",
          "player": "playerC",
          "name": "Eye of Doom"
        },
        {
          "op": "addTarget",
          "player": "playerC",
          "target": "EndOfTurnOneOpponentTest.cardBear2"
        },
        {
          "op": "addTarget",
          "player": 1,
          "target": "EndOfTurnOneOpponentTest.cardBear2"
        },
        {
          "op": "addTarget",
          "player": "playerD",
          "target": "EndOfTurnOneOpponentTest.cardBear2"
        },
        {
          "op": "unsupported",
          "source": "checkPlayerInGame(\"A must leaved in 4\", 4, PhaseStep.POSTCOMBAT_MAIN, playerB, playerA, false)"
        },
        {
          "op": "attack",
          "turn": 4,
          "player": 1,
          "attacker": "EndOfTurnOneOpponentTest.cardBear2",
          "defender": 1
        },
        {
          "op": "unsupported",
          "source": "checkPlayerInGame(\"A must leaved in 5\", 5, PhaseStep.POSTCOMBAT_MAIN, playerD, playerA, false)"
        },
        {
          "op": "attack",
          "turn": 5,
          "player": "playerD",
          "attacker": "EndOfTurnOneOpponentTest.cardBear2",
          "defender": 1
        },
        {
          "op": "setStopAt",
          "turn": 5,
          "phase": "CLEANUP"
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
