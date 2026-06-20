import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/continuous/EndOfTurnOneOpponentTest.java",
  "tests": [
    {
      "name": "test_EndOfTurnSingle",
      "operations": [
        {
          "op": "unsupported",
          "source": "addCustomCardWithAbility(\"boost1\", playerA, new SimpleStaticAbility(Zone.ALL, new BoostAllEffect(1, 1, Duration.EndOfTurn)))"
        },
        {
          "op": "unsupported",
          "source": "for (PhaseStep step : PhaseStep.values()) { switch (step) { case UNTAP: case DRAW: case UPKEEP: case FIRST_COMBAT_DAMAGE: case CLEANUP: continue; case PRECOMBAT_MAIN: case POSTCOMBAT_MAIN: case END_TURN: break; case BEGIN_COMBAT: case DECLARE_ATTACKERS: case DECLARE_BLOCKERS: case COMBAT_DAMAGE: case END_COMBAT: if (!true) continue; break; default: throw new IllegalStateException(\"Unknown phase step \" + step); } int permP = 2; int permT = 2; String existsStr = \"must NOT EXISTS\"; if (PhaseStep.END_TURN != null && step.getIndex() <= PhaseStep.END_TURN.getIndex()) { permP++; permT++; existsStr = \"must EXISTS\"; } this.checkPT(\"Duration.EndOfTurn effect\" + \" \" + existsStr + \" on turn \" + 1 + \" - \" + step.toString() + \" for \" + playerA.getName(), 1, step, playerA, cardBear2, permP, permT); }"
        },
        {
          "op": "unsupported",
          "source": "for (PhaseStep step : PhaseStep.values()) { switch (step) { case UNTAP: case DRAW: case UPKEEP: case FIRST_COMBAT_DAMAGE: case CLEANUP: continue; case PRECOMBAT_MAIN: case POSTCOMBAT_MAIN: case END_TURN: break; case BEGIN_COMBAT: case DECLARE_ATTACKERS: case DECLARE_BLOCKERS: case COMBAT_DAMAGE: case END_COMBAT: if (!true) continue; break; default: throw new IllegalStateException(\"Unknown phase step \" + step); } int permP = 2; int permT = 2; String existsStr = \"must NOT EXISTS\"; if (null != null && step.getIndex() <= null.getIndex()) { permP++; permT++; existsStr = \"must EXISTS\"; } this.checkPT(\"Duration.EndOfTurn effect\" + \" \" + existsStr + \" on turn \" + 2 + \" - \" + step.toString() + \" for \" + playerA.getName(), 2, step, playerA, cardBear2, permP, permT); }"
        },
        {
          "op": "unsupported",
          "source": "for (PhaseStep step : PhaseStep.values()) { switch (step) { case UNTAP: case DRAW: case UPKEEP: case FIRST_COMBAT_DAMAGE: case CLEANUP: continue; case PRECOMBAT_MAIN: case POSTCOMBAT_MAIN: case END_TURN: break; case BEGIN_COMBAT: case DECLARE_ATTACKERS: case DECLARE_BLOCKERS: case COMBAT_DAMAGE: case END_COMBAT: if (!true) continue; break; default: throw new IllegalStateException(\"Unknown phase step \" + step); } int permP = 2; int permT = 2; String existsStr = \"must NOT EXISTS\"; if (null != null && step.getIndex() <= null.getIndex()) { permP++; permT++; existsStr = \"must EXISTS\"; } this.checkPT(\"Duration.EndOfTurn effect\" + \" \" + existsStr + \" on turn \" + 3 + \" - \" + step.toString() + \" for \" + playerA.getName(), 3, step, playerA, cardBear2, permP, permT); }"
        },
        {
          "op": "unsupported",
          "source": "for (PhaseStep step : PhaseStep.values()) { switch (step) { case UNTAP: case DRAW: case UPKEEP: case FIRST_COMBAT_DAMAGE: case CLEANUP: continue; case PRECOMBAT_MAIN: case POSTCOMBAT_MAIN: case END_TURN: break; case BEGIN_COMBAT: case DECLARE_ATTACKERS: case DECLARE_BLOCKERS: case COMBAT_DAMAGE: case END_COMBAT: if (!true) continue; break; default: throw new IllegalStateException(\"Unknown phase step \" + step); } int permP = 2; int permT = 2; String existsStr = \"must NOT EXISTS\"; if (null != null && step.getIndex() <= null.getIndex()) { permP++; permT++; existsStr = \"must EXISTS\"; } this.checkPT(\"Duration.EndOfTurn effect\" + \" \" + existsStr + \" on turn \" + 4 + \" - \" + step.toString() + \" for \" + playerA.getName(), 4, step, playerA, cardBear2, permP, permT); }"
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
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Balduvian Bears",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 2,
          "player": 1,
          "attacker": "Balduvian Bears",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Balduvian Bears",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 4,
          "player": 1,
          "attacker": "Balduvian Bears",
          "defender": 1
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
      "name": "test_UntilYourNextTurnSingle",
      "operations": [
        {
          "op": "unsupported",
          "source": "addCustomCardWithAbility(\"boost1\", playerA, new SimpleStaticAbility(Zone.ALL, new BoostAllEffect(1, 1, Duration.UntilYourNextTurn)))"
        },
        {
          "op": "unsupported",
          "source": "for (PhaseStep step : PhaseStep.values()) { switch (step) { case UNTAP: case DRAW: case UPKEEP: case FIRST_COMBAT_DAMAGE: case CLEANUP: continue; case PRECOMBAT_MAIN: case POSTCOMBAT_MAIN: case END_TURN: break; case BEGIN_COMBAT: case DECLARE_ATTACKERS: case DECLARE_BLOCKERS: case COMBAT_DAMAGE: case END_COMBAT: if (!true) continue; break; default: throw new IllegalStateException(\"Unknown phase step \" + step); } int permP = 2; int permT = 2; String existsStr = \"must NOT EXISTS\"; if (PhaseStep.END_TURN != null && step.getIndex() <= PhaseStep.END_TURN.getIndex()) { permP++; permT++; existsStr = \"must EXISTS\"; } this.checkPT(\"Duration.UntilYourNextTurn effect\" + \" \" + existsStr + \" on turn \" + 1 + \" - \" + step.toString() + \" for \" + playerA.getName(), 1, step, playerA, cardBear2, permP, permT); }"
        },
        {
          "op": "unsupported",
          "source": "for (PhaseStep step : PhaseStep.values()) { switch (step) { case UNTAP: case DRAW: case UPKEEP: case FIRST_COMBAT_DAMAGE: case CLEANUP: continue; case PRECOMBAT_MAIN: case POSTCOMBAT_MAIN: case END_TURN: break; case BEGIN_COMBAT: case DECLARE_ATTACKERS: case DECLARE_BLOCKERS: case COMBAT_DAMAGE: case END_COMBAT: if (!true) continue; break; default: throw new IllegalStateException(\"Unknown phase step \" + step); } int permP = 2; int permT = 2; String existsStr = \"must NOT EXISTS\"; if (PhaseStep.END_TURN != null && step.getIndex() <= PhaseStep.END_TURN.getIndex()) { permP++; permT++; existsStr = \"must EXISTS\"; } this.checkPT(\"Duration.UntilYourNextTurn effect\" + \" \" + existsStr + \" on turn \" + 2 + \" - \" + step.toString() + \" for \" + playerA.getName(), 2, step, playerA, cardBear2, permP, permT); }"
        },
        {
          "op": "unsupported",
          "source": "for (PhaseStep step : PhaseStep.values()) { switch (step) { case UNTAP: case DRAW: case UPKEEP: case FIRST_COMBAT_DAMAGE: case CLEANUP: continue; case PRECOMBAT_MAIN: case POSTCOMBAT_MAIN: case END_TURN: break; case BEGIN_COMBAT: case DECLARE_ATTACKERS: case DECLARE_BLOCKERS: case COMBAT_DAMAGE: case END_COMBAT: if (!true) continue; break; default: throw new IllegalStateException(\"Unknown phase step \" + step); } int permP = 2; int permT = 2; String existsStr = \"must NOT EXISTS\"; if (null != null && step.getIndex() <= null.getIndex()) { permP++; permT++; existsStr = \"must EXISTS\"; } this.checkPT(\"Duration.UntilYourNextTurn effect\" + \" \" + existsStr + \" on turn \" + 3 + \" - \" + step.toString() + \" for \" + playerA.getName(), 3, step, playerA, cardBear2, permP, permT); }"
        },
        {
          "op": "unsupported",
          "source": "for (PhaseStep step : PhaseStep.values()) { switch (step) { case UNTAP: case DRAW: case UPKEEP: case FIRST_COMBAT_DAMAGE: case CLEANUP: continue; case PRECOMBAT_MAIN: case POSTCOMBAT_MAIN: case END_TURN: break; case BEGIN_COMBAT: case DECLARE_ATTACKERS: case DECLARE_BLOCKERS: case COMBAT_DAMAGE: case END_COMBAT: if (!true) continue; break; default: throw new IllegalStateException(\"Unknown phase step \" + step); } int permP = 2; int permT = 2; String existsStr = \"must NOT EXISTS\"; if (null != null && step.getIndex() <= null.getIndex()) { permP++; permT++; existsStr = \"must EXISTS\"; } this.checkPT(\"Duration.UntilYourNextTurn effect\" + \" \" + existsStr + \" on turn \" + 4 + \" - \" + step.toString() + \" for \" + playerA.getName(), 4, step, playerA, cardBear2, permP, permT); }"
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
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Balduvian Bears",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 2,
          "player": 1,
          "attacker": "Balduvian Bears",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Balduvian Bears",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 4,
          "player": 1,
          "attacker": "Balduvian Bears",
          "defender": 1
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
      "name": "test_UntilEndOfYourNextTurnSingle",
      "operations": [
        {
          "op": "unsupported",
          "source": "addCustomCardWithAbility(\"boost1\", playerA, new SimpleStaticAbility(Zone.ALL, new BoostAllEffect(1, 1, Duration.UntilEndOfYourNextTurn)))"
        },
        {
          "op": "unsupported",
          "source": "for (PhaseStep step : PhaseStep.values()) { switch (step) { case UNTAP: case DRAW: case UPKEEP: case FIRST_COMBAT_DAMAGE: case CLEANUP: continue; case PRECOMBAT_MAIN: case POSTCOMBAT_MAIN: case END_TURN: break; case BEGIN_COMBAT: case DECLARE_ATTACKERS: case DECLARE_BLOCKERS: case COMBAT_DAMAGE: case END_COMBAT: if (!true) continue; break; default: throw new IllegalStateException(\"Unknown phase step \" + step); } int permP = 2; int permT = 2; String existsStr = \"must NOT EXISTS\"; if (PhaseStep.END_TURN != null && step.getIndex() <= PhaseStep.END_TURN.getIndex()) { permP++; permT++; existsStr = \"must EXISTS\"; } this.checkPT(\"Duration.UntilYourNextTurn effect\" + \" \" + existsStr + \" on turn \" + 1 + \" - \" + step.toString() + \" for \" + playerA.getName(), 1, step, playerA, cardBear2, permP, permT); }"
        },
        {
          "op": "unsupported",
          "source": "for (PhaseStep step : PhaseStep.values()) { switch (step) { case UNTAP: case DRAW: case UPKEEP: case FIRST_COMBAT_DAMAGE: case CLEANUP: continue; case PRECOMBAT_MAIN: case POSTCOMBAT_MAIN: case END_TURN: break; case BEGIN_COMBAT: case DECLARE_ATTACKERS: case DECLARE_BLOCKERS: case COMBAT_DAMAGE: case END_COMBAT: if (!true) continue; break; default: throw new IllegalStateException(\"Unknown phase step \" + step); } int permP = 2; int permT = 2; String existsStr = \"must NOT EXISTS\"; if (PhaseStep.END_TURN != null && step.getIndex() <= PhaseStep.END_TURN.getIndex()) { permP++; permT++; existsStr = \"must EXISTS\"; } this.checkPT(\"Duration.UntilYourNextTurn effect\" + \" \" + existsStr + \" on turn \" + 2 + \" - \" + step.toString() + \" for \" + playerA.getName(), 2, step, playerA, cardBear2, permP, permT); }"
        },
        {
          "op": "unsupported",
          "source": "for (PhaseStep step : PhaseStep.values()) { switch (step) { case UNTAP: case DRAW: case UPKEEP: case FIRST_COMBAT_DAMAGE: case CLEANUP: continue; case PRECOMBAT_MAIN: case POSTCOMBAT_MAIN: case END_TURN: break; case BEGIN_COMBAT: case DECLARE_ATTACKERS: case DECLARE_BLOCKERS: case COMBAT_DAMAGE: case END_COMBAT: if (!true) continue; break; default: throw new IllegalStateException(\"Unknown phase step \" + step); } int permP = 2; int permT = 2; String existsStr = \"must NOT EXISTS\"; if (PhaseStep.END_TURN != null && step.getIndex() <= PhaseStep.END_TURN.getIndex()) { permP++; permT++; existsStr = \"must EXISTS\"; } this.checkPT(\"Duration.UntilYourNextTurn effect\" + \" \" + existsStr + \" on turn \" + 3 + \" - \" + step.toString() + \" for \" + playerA.getName(), 3, step, playerA, cardBear2, permP, permT); }"
        },
        {
          "op": "unsupported",
          "source": "for (PhaseStep step : PhaseStep.values()) { switch (step) { case UNTAP: case DRAW: case UPKEEP: case FIRST_COMBAT_DAMAGE: case CLEANUP: continue; case PRECOMBAT_MAIN: case POSTCOMBAT_MAIN: case END_TURN: break; case BEGIN_COMBAT: case DECLARE_ATTACKERS: case DECLARE_BLOCKERS: case COMBAT_DAMAGE: case END_COMBAT: if (!true) continue; break; default: throw new IllegalStateException(\"Unknown phase step \" + step); } int permP = 2; int permT = 2; String existsStr = \"must NOT EXISTS\"; if (null != null && step.getIndex() <= null.getIndex()) { permP++; permT++; existsStr = \"must EXISTS\"; } this.checkPT(\"Duration.UntilYourNextTurn effect\" + \" \" + existsStr + \" on turn \" + 4 + \" - \" + step.toString() + \" for \" + playerA.getName(), 4, step, playerA, cardBear2, permP, permT); }"
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
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Balduvian Bears",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 2,
          "player": 1,
          "attacker": "Balduvian Bears",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Balduvian Bears",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 4,
          "player": 1,
          "attacker": "Balduvian Bears",
          "defender": 1
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
    }
  ]
});
