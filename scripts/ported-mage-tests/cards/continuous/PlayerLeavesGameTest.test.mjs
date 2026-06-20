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
      "operations": [
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
          "player": "playerD",
          "name": "Balduvian Bears",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "addCustomCardWithAbility(\"effect\", playerA, new SimpleStaticAbility(new BoostAllEffect(1, 1, Duration.WhileOnBattlefield)))"
        },
        {
          "op": "unsupported",
          "source": "checkPlayerInGame(Duration.WhileOnBattlefield.toString() + \" - turn 1\", 1, PhaseStep.PRECOMBAT_MAIN, playerD, playerA, true)"
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
          "op": "assertPowerToughness",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": "playerD",
          "name": "Balduvian Bears",
          "power": 3,
          "toughness": 3
        },
        {
          "op": "unsupported",
          "source": "checkPlayerInGame(Duration.WhileOnBattlefield.name() + \" - turn 2\", 2, PhaseStep.PRECOMBAT_MAIN, playerD, playerA, true)"
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
          "op": "assertPowerToughness",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": "playerD",
          "name": "Balduvian Bears",
          "power": 3,
          "toughness": 3
        },
        {
          "op": "unsupported",
          "source": "checkPlayerInGame(Duration.WhileOnBattlefield.name() + \" - turn 3 before\", 3, PhaseStep.PRECOMBAT_MAIN, playerD, playerA, true)"
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
          "op": "assertPowerToughness",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": "playerD",
          "name": "Balduvian Bears",
          "power": 3,
          "toughness": 3
        },
        {
          "op": "unsupported",
          "source": "concede(3, PhaseStep.POSTCOMBAT_MAIN, playerA)"
        },
        {
          "op": "unsupported",
          "source": "checkPlayerInGame(Duration.WhileOnBattlefield.name() + \" - turn 3 after\", 3, PhaseStep.END_TURN, playerD, playerA, false)"
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
          "op": "assertPowerToughness",
          "turn": 3,
          "phase": "END_TURN",
          "player": "playerD",
          "name": "Balduvian Bears",
          "power": 2,
          "toughness": 2
        },
        {
          "op": "unsupported",
          "source": "checkPlayerInGame(Duration.WhileOnBattlefield.name() + \" - turn 4\", 4, PhaseStep.PRECOMBAT_MAIN, playerD, playerA, false)"
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
          "op": "assertPowerToughness",
          "turn": 4,
          "phase": "PRECOMBAT_MAIN",
          "player": "playerD",
          "name": "Balduvian Bears",
          "power": 2,
          "toughness": 2
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
      "name": "test_PlayerLeaveGameWithOwnPermanentAndEndOfGameEffect",
      "operations": [
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
          "player": "playerD",
          "name": "Balduvian Bears",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "addCustomCardWithAbility(\"effect\", playerA, new SimpleStaticAbility(new BoostAllEffect(1, 1, Duration.EndOfGame)))"
        },
        {
          "op": "unsupported",
          "source": "checkPlayerInGame(Duration.EndOfGame.toString() + \" - turn 1\", 1, PhaseStep.PRECOMBAT_MAIN, playerD, playerA, true)"
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
          "op": "assertPowerToughness",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": "playerD",
          "name": "Balduvian Bears",
          "power": 3,
          "toughness": 3
        },
        {
          "op": "unsupported",
          "source": "checkPlayerInGame(Duration.EndOfGame.name() + \" - turn 2\", 2, PhaseStep.PRECOMBAT_MAIN, playerD, playerA, true)"
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
          "op": "assertPowerToughness",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": "playerD",
          "name": "Balduvian Bears",
          "power": 3,
          "toughness": 3
        },
        {
          "op": "unsupported",
          "source": "checkPlayerInGame(Duration.EndOfGame.name() + \" - turn 3 before\", 3, PhaseStep.PRECOMBAT_MAIN, playerD, playerA, true)"
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
          "op": "assertPowerToughness",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": "playerD",
          "name": "Balduvian Bears",
          "power": 3,
          "toughness": 3
        },
        {
          "op": "unsupported",
          "source": "concede(3, PhaseStep.POSTCOMBAT_MAIN, playerA)"
        },
        {
          "op": "unsupported",
          "source": "checkPlayerInGame(Duration.EndOfGame.name() + \" - turn 3 after\", 3, PhaseStep.END_TURN, playerD, playerA, false)"
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
          "op": "assertPowerToughness",
          "turn": 3,
          "phase": "END_TURN",
          "player": "playerD",
          "name": "Balduvian Bears",
          "power": 2,
          "toughness": 2
        },
        {
          "op": "unsupported",
          "source": "checkPlayerInGame(Duration.EndOfGame.name() + \" - turn 4\", 4, PhaseStep.PRECOMBAT_MAIN, playerD, playerA, false)"
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
          "op": "assertPowerToughness",
          "turn": 4,
          "phase": "PRECOMBAT_MAIN",
          "player": "playerD",
          "name": "Balduvian Bears",
          "power": 2,
          "toughness": 2
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
      "name": "test_PlayerLeaveGameWithOwnPermanentAndUntilSourceLeavesBattlefielEffect",
      "operations": [
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
          "player": "playerD",
          "name": "Balduvian Bears",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "addCustomCardWithAbility(\"effect\", playerA, new SimpleStaticAbility(new BoostAllEffect(1, 1, Duration.UntilSourceLeavesBattlefield)))"
        },
        {
          "op": "unsupported",
          "source": "checkPlayerInGame(Duration.UntilSourceLeavesBattlefield.toString() + \" - turn 1\", 1, PhaseStep.PRECOMBAT_MAIN, playerD, playerA, true)"
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
          "op": "assertPowerToughness",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": "playerD",
          "name": "Balduvian Bears",
          "power": 3,
          "toughness": 3
        },
        {
          "op": "unsupported",
          "source": "checkPlayerInGame(Duration.UntilSourceLeavesBattlefield.name() + \" - turn 2\", 2, PhaseStep.PRECOMBAT_MAIN, playerD, playerA, true)"
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
          "op": "assertPowerToughness",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": "playerD",
          "name": "Balduvian Bears",
          "power": 3,
          "toughness": 3
        },
        {
          "op": "unsupported",
          "source": "checkPlayerInGame(Duration.UntilSourceLeavesBattlefield.name() + \" - turn 3 before\", 3, PhaseStep.PRECOMBAT_MAIN, playerD, playerA, true)"
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
          "op": "assertPowerToughness",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": "playerD",
          "name": "Balduvian Bears",
          "power": 3,
          "toughness": 3
        },
        {
          "op": "unsupported",
          "source": "concede(3, PhaseStep.POSTCOMBAT_MAIN, playerA)"
        },
        {
          "op": "unsupported",
          "source": "checkPlayerInGame(Duration.UntilSourceLeavesBattlefield.name() + \" - turn 3 after\", 3, PhaseStep.END_TURN, playerD, playerA, false)"
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
          "op": "assertPowerToughness",
          "turn": 3,
          "phase": "END_TURN",
          "player": "playerD",
          "name": "Balduvian Bears",
          "power": 2,
          "toughness": 2
        },
        {
          "op": "unsupported",
          "source": "checkPlayerInGame(Duration.UntilSourceLeavesBattlefield.name() + \" - turn 4\", 4, PhaseStep.PRECOMBAT_MAIN, playerD, playerA, false)"
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
          "op": "assertPowerToughness",
          "turn": 4,
          "phase": "PRECOMBAT_MAIN",
          "player": "playerD",
          "name": "Balduvian Bears",
          "power": 2,
          "toughness": 2
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
      "name": "test_EndOfTurnMultiLeave",
      "operations": [
        {
          "op": "unsupported",
          "source": "addCustomCardWithAbility(\"boost\", playerA, new SimpleStaticAbility(Zone.ALL, new BoostAllEffect(1, 1, Duration.EndOfTurn)))"
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
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": "playerD",
          "name": "Balduvian Bears",
          "power": 3,
          "toughness": 3
        },
        {
          "op": "assertPowerToughness",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": "playerD",
          "name": "Balduvian Bears",
          "power": 3,
          "toughness": 3
        },
        {
          "op": "assertPowerToughness",
          "turn": 1,
          "phase": "END_TURN",
          "player": "playerD",
          "name": "Balduvian Bears",
          "power": 3,
          "toughness": 3
        },
        {
          "op": "assertPowerToughness",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": "playerD",
          "name": "Balduvian Bears",
          "power": 2,
          "toughness": 2
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
      "operations": [
        {
          "op": "unsupported",
          "source": "addCustomCardWithAbility(\"boost\", playerA, new SimpleStaticAbility(Zone.ALL, new BoostAllEffect(1, 1, Duration.UntilYourNextTurn)))"
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
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": "playerD",
          "name": "Balduvian Bears",
          "power": 3,
          "toughness": 3
        },
        {
          "op": "assertPermanentCount",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": "playerD",
          "name": 0,
          "count": "boost"
        },
        {
          "op": "assertPowerToughness",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": "playerD",
          "name": "Balduvian Bears",
          "power": 3,
          "toughness": 3
        },
        {
          "op": "assertPermanentCount",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": "playerD",
          "name": 0,
          "count": "boost"
        },
        {
          "op": "assertPowerToughness",
          "turn": 1,
          "phase": "END_TURN",
          "player": "playerD",
          "name": "Balduvian Bears",
          "power": 3,
          "toughness": 3
        },
        {
          "op": "assertPowerToughness",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": "playerD",
          "name": "Balduvian Bears",
          "power": 3,
          "toughness": 3
        },
        {
          "op": "assertPowerToughness",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": "playerD",
          "name": "Balduvian Bears",
          "power": 3,
          "toughness": 3
        },
        {
          "op": "assertPowerToughness",
          "turn": 4,
          "phase": "PRECOMBAT_MAIN",
          "player": "playerD",
          "name": "Balduvian Bears",
          "power": 3,
          "toughness": 3
        },
        {
          "op": "assertPowerToughness",
          "turn": 5,
          "phase": "PRECOMBAT_MAIN",
          "player": "playerD",
          "name": "Balduvian Bears",
          "power": 2,
          "toughness": 2
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
    },
    {
      "name": "test_UntilEndOfYourNextTurnMultiLeave",
      "operations": [
        {
          "op": "unsupported",
          "source": "addCustomCardWithAbility(\"boost\", playerA, new SimpleStaticAbility(Zone.ALL, new BoostAllEffect(1, 1, Duration.UntilEndOfYourNextTurn)))"
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
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": "playerD",
          "name": "Balduvian Bears",
          "power": 3,
          "toughness": 3
        },
        {
          "op": "assertPermanentCount",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": "playerD",
          "name": 0,
          "count": "boost"
        },
        {
          "op": "assertPowerToughness",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": "playerD",
          "name": "Balduvian Bears",
          "power": 3,
          "toughness": 3
        },
        {
          "op": "assertPermanentCount",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": "playerD",
          "name": 0,
          "count": "boost"
        },
        {
          "op": "assertPowerToughness",
          "turn": 1,
          "phase": "END_TURN",
          "player": "playerD",
          "name": "Balduvian Bears",
          "power": 3,
          "toughness": 3
        },
        {
          "op": "assertPowerToughness",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": "playerD",
          "name": "Balduvian Bears",
          "power": 3,
          "toughness": 3
        },
        {
          "op": "assertPowerToughness",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": "playerD",
          "name": "Balduvian Bears",
          "power": 3,
          "toughness": 3
        },
        {
          "op": "assertPowerToughness",
          "turn": 4,
          "phase": "PRECOMBAT_MAIN",
          "player": "playerD",
          "name": "Balduvian Bears",
          "power": 3,
          "toughness": 3
        },
        {
          "op": "assertPowerToughness",
          "turn": 5,
          "phase": "PRECOMBAT_MAIN",
          "player": "playerD",
          "name": "Balduvian Bears",
          "power": 2,
          "toughness": 2
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
