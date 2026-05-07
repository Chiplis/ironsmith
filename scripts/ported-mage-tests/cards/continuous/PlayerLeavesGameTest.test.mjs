import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/continuous/PlayerLeavesGameTest.java",
  "tests": [
    {
      "name": "test_PlayerLeaveGame",
      "operations": [
        {
          "op": "unsupported",
          "source": "checkPlayerInGame(\"turn 1\", 1, PhaseStep.PRECOMBAT_MAIN, playerD, playerA, true)"
        },
        {
          "op": "unsupported",
          "source": "checkPlayerInGame(\"turn 2\", 2, PhaseStep.PRECOMBAT_MAIN, playerD, playerA, true)"
        },
        {
          "op": "unsupported",
          "source": "checkPlayerInGame(\"turn 3 before\", 3, PhaseStep.PRECOMBAT_MAIN, playerD, playerA, true)"
        },
        {
          "op": "unsupported",
          "source": "concede(3, PhaseStep.POSTCOMBAT_MAIN, playerA)"
        },
        {
          "op": "unsupported",
          "source": "checkPlayerInGame(\"turn 3 after\", 3, PhaseStep.END_TURN, playerD, playerA, false)"
        },
        {
          "op": "unsupported",
          "source": "checkPlayerInGame(\"turn 4\", 4, PhaseStep.PRECOMBAT_MAIN, playerD, playerA, false)"
        },
        {
          "op": "setStopAt",
          "turn": 4,
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
      "name": "test_PlayerLeaveGameWithOwnPermanent",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Balduvian Bears",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "checkPlayerInGame(\"turn 1\", 1, PhaseStep.PRECOMBAT_MAIN, playerD, playerA, true)"
        },
        {
          "op": "assertPermanentCount",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": "playerD",
          "name": 0,
          "count": "Balduvian Bears"
        },
        {
          "op": "unsupported",
          "source": "checkPlayerInGame(\"turn 2\", 2, PhaseStep.PRECOMBAT_MAIN, playerD, playerA, true)"
        },
        {
          "op": "assertPermanentCount",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": "playerD",
          "name": 0,
          "count": "Balduvian Bears"
        },
        {
          "op": "unsupported",
          "source": "checkPlayerInGame(\"turn 3 before\", 3, PhaseStep.PRECOMBAT_MAIN, playerD, playerA, true)"
        },
        {
          "op": "assertPermanentCount",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": "playerD",
          "name": 0,
          "count": "Balduvian Bears"
        },
        {
          "op": "unsupported",
          "source": "concede(3, PhaseStep.POSTCOMBAT_MAIN, playerA)"
        },
        {
          "op": "unsupported",
          "source": "checkPlayerInGame(\"turn 3 after\", 3, PhaseStep.END_TURN, playerD, playerA, false)"
        },
        {
          "op": "assertPermanentCount",
          "turn": 3,
          "phase": "END_TURN",
          "player": "playerD",
          "name": 0,
          "count": "Balduvian Bears"
        },
        {
          "op": "unsupported",
          "source": "checkPlayerInGame(\"turn 4\", 4, PhaseStep.PRECOMBAT_MAIN, playerD, playerA, false)"
        },
        {
          "op": "assertPermanentCount",
          "turn": 4,
          "phase": "PRECOMBAT_MAIN",
          "player": "playerD",
          "name": 0,
          "count": "Balduvian Bears"
        },
        {
          "op": "setStopAt",
          "turn": 4,
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
      "name": "test_PlayerLeaveGameWithOwnPermanentAndCustomEffect",
      "operations": []
    },
    {
      "name": "test_PlayerLeaveGameWithOwnPermanentAndWhileOnBattlefieldEffect",
      "operations": []
    },
    {
      "name": "test_PlayerLeaveGameWithOwnPermanentAndEndOfGameEffect",
      "operations": []
    },
    {
      "name": "test_PlayerLeaveGameWithOwnPermanentAndUntilSourceLeavesBattlefielEffect",
      "operations": []
    },
    {
      "name": "test_EndOfTurnMultiLeave",
      "operations": [
        {
          "op": "addCard",
          "zone": "boost",
          "player": 0,
          "name": "ALL",
          "custom": true,
          "oracleText": ""
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Balduvian Bears",
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
          "player": "playerC",
          "name": "Balduvian Bears",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": "playerD",
          "name": "Balduvian Bears",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "concede(1, PhaseStep.POSTCOMBAT_MAIN, playerA)"
        },
        {
          "op": "assertPowerToughness",
          "player": 1,
          "name": "PRECOMBAT_MAIN",
          "power": "playerD",
          "toughness": "Balduvian Bears"
        },
        {
          "op": "assertPowerToughness",
          "player": 1,
          "name": "POSTCOMBAT_MAIN",
          "power": "playerD",
          "toughness": "Balduvian Bears"
        },
        {
          "op": "assertPowerToughness",
          "player": 1,
          "name": "END_TURN",
          "power": "playerD",
          "toughness": "Balduvian Bears"
        },
        {
          "op": "assertPowerToughness",
          "player": "turn 2",
          "name": 2,
          "power": "PRECOMBAT_MAIN",
          "toughness": "playerD"
        },
        {
          "op": "setStopAt",
          "turn": 2,
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
      "name": "test_UntilYourNextTurnMultiLeave",
      "operations": []
    },
    {
      "name": "test_UntilEndOfYourNextTurnMultiLeave",
      "operations": []
    }
  ]
});
