import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/akh/ApproachOfTheSecondSunTest.java",
  "tests": [
    {
      "name": "testWinGameTest",
      "operations": [
        {
          "op": "clearZone",
          "player": 0,
          "zone": "library"
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Approach of the Second Sun",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 14
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Approach of the Second Sun"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Approach of the Second Sun"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 27
        },
        {
          "op": "assertLibraryCount",
          "player": 0,
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "count": 1
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "count": 0
        },
        {
          "op": "unsupported",
          "source": "assertResult(playerA, GameResult.WON)"
        }
      ]
    },
    {
      "name": "testDontCountOpponentCast",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Approach of the Second Sun",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Approach of the Second Sun",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 7
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Plains",
          "count": 7
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Approach of the Second Sun"
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Approach of the Second Sun"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        },
        {
          "op": "unsupported",
          "source": "assertResult(playerA, GameResult.DRAW)"
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 27
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 27
        }
      ]
    },
    {
      "name": "testRightPositionInDeck",
      "operations": [
        {
          "op": "clearZone",
          "player": 0,
          "zone": "library"
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Plains",
          "count": 15
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Approach of the Second Sun",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Concentrate",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Tundra",
          "count": 15
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Approach of the Second Sun"
        },
        {
          "op": "unsupported",
          "source": "checkLibraryCount(\"Approach of the Second Sun went to library\", 1, PhaseStep.PRECOMBAT_MAIN, playerA, \"Approach of the Second Sun\", 1)"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Concentrate"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Concentrate"
        },
        {
          "op": "unsupported",
          "source": "checkLibraryCount(\"Approach of the Second Sun is not in top 6 cards\", 3, PhaseStep.UPKEEP, playerA, \"Approach of the Second Sun\", 1)"
        },
        {
          "op": "unsupported",
          "source": "checkLibraryCount(\"Approach of the Second Sun is 7th card\", 3, PhaseStep.PRECOMBAT_MAIN, playerA, \"Approach of the Second Sun\", 0)"
        },
        {
          "op": "assertHandCount",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Approach of the Second Sun",
          "count": 1
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 27
        },
        {
          "op": "assertLibraryCount",
          "player": 0,
          "count": 9
        },
        {
          "op": "unsupported",
          "source": "assertResult(playerA, GameResult.DRAW)"
        }
      ]
    },
    {
      "name": "testCastSameCard",
      "operations": [
        {
          "op": "clearZone",
          "player": 0,
          "zone": "library"
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Plains",
          "count": 6
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Approach of the Second Sun",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Concentrate",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Equal Treatment",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Tundra",
          "count": 24
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Approach of the Second Sun"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Concentrate"
        },
        {
          "op": "assertStackSize",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "count": 0
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Concentrate"
        },
        {
          "op": "assertStackSize",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "count": 0
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Equal Treatment"
        },
        {
          "op": "assertStackSize",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "count": 0
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Approach of the Second Sun"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 27
        },
        {
          "op": "assertLibraryCount",
          "player": 0,
          "count": 0
        },
        {
          "op": "unsupported",
          "source": "assertResult(playerA, GameResult.WON)"
        }
      ]
    },
    {
      "name": "testRightPositionInSmallDeck",
      "operations": [
        {
          "op": "clearZone",
          "player": 0,
          "zone": "library"
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Plains",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Approach of the Second Sun",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Concentrate",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Tundra",
          "count": 15
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Approach of the Second Sun"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Concentrate"
        },
        {
          "op": "unsupported",
          "source": "checkLibraryCount(\"Approach of the Second Sun is not in top 3 cards\", 3, PhaseStep.UPKEEP, playerA, \"Approach of the Second Sun\", 1)"
        },
        {
          "op": "unsupported",
          "source": "checkLibraryCount(\"Approach of the Second Sun is 4th card\", 3, PhaseStep.PRECOMBAT_MAIN, playerA, \"Approach of the Second Sun\", 0)"
        },
        {
          "op": "assertHandCount",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Approach of the Second Sun",
          "count": 1
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 27
        },
        {
          "op": "assertLibraryCount",
          "player": 0,
          "count": 0
        },
        {
          "op": "unsupported",
          "source": "assertResult(playerA, GameResult.DRAW)"
        }
      ]
    },
    {
      "name": "testCastFromGraveyard",
      "operations": [
        {
          "op": "clearZone",
          "player": 0,
          "zone": "library"
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Plains",
          "count": 6
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Approach of the Second Sun",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Approach of the Second Sun",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Finale of Promise",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mystic Monastery",
          "count": 25
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Finale of Promise"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "X=7"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "TARGET_SKIP"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Approach of the Second Sun"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Yes"
        },
        {
          "op": "unsupported",
          "source": "checkLife(\"Approach of the Second Sun cast from graveyard gains life\", 1, PhaseStep.PRECOMBAT_MAIN, playerA, 27)"
        },
        {
          "op": "unsupported",
          "source": "checkLibraryCount(\"Approach of the Second Sun cast from graveyard goes to library\", 1, PhaseStep.PRECOMBAT_MAIN, playerA, \"Approach of the Second Sun\", 1)"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Finale of Promise"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "X=7"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "TARGET_SKIP"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Approach of the Second Sun"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Yes"
        },
        {
          "op": "unsupported",
          "source": "checkLife(\"Approach of the Second Sun cast from graveyard gains life\", 1, PhaseStep.PRECOMBAT_MAIN, playerA, 34)"
        },
        {
          "op": "unsupported",
          "source": "checkLibraryCount(\"Approach of the Second Sun cast from graveyard goes to library\", 1, PhaseStep.PRECOMBAT_MAIN, playerA, \"Approach of the Second Sun\", 2)"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Approach of the Second Sun"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        },
        {
          "op": "unsupported",
          "source": "assertResult(playerA, GameResult.WON)"
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 34
        },
        {
          "op": "assertLibraryCount",
          "player": 0,
          "name": "Approach of the Second Sun",
          "count": 2
        }
      ]
    },
    {
      "name": "testCastCopyCard",
      "operations": [
        {
          "op": "clearZone",
          "player": 0,
          "zone": "library"
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Plains",
          "count": 6
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Approach of the Second Sun",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Approach of the Second Sun",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mystic Monastery",
          "count": 25
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Demilich",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Demilich",
          "defender": 1
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Approach of the Second Sun"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Yes"
        },
        {
          "op": "unsupported",
          "source": "checkLife(\"Approach of the Second Sun cast from graveyard gains life\", 1, PhaseStep.POSTCOMBAT_MAIN, playerA, 27)"
        },
        {
          "op": "unsupported",
          "source": "checkLibraryCount(\"Copied Card Approach of the Second Sun cast from graveyard does not go to library\", 1, PhaseStep.PRECOMBAT_MAIN, playerA, \"Approach of the Second Sun\", 0)"
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Demilich",
          "defender": 1
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Approach of the Second Sun"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Yes"
        },
        {
          "op": "unsupported",
          "source": "checkLife(\"Approach of the Second Sun cast from graveyard gains life\", 1, PhaseStep.POSTCOMBAT_MAIN, playerA, 27)"
        },
        {
          "op": "unsupported",
          "source": "checkLibraryCount(\"Copied Card Approach of the Second Sun cast from graveyard does not go to library\", 3, PhaseStep.PRECOMBAT_MAIN, playerA, \"Approach of the Second Sun\", 0)"
        },
        {
          "op": "castSpell",
          "turn": 3,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Approach of the Second Sun"
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
        },
        {
          "op": "unsupported",
          "source": "assertResult(playerA, GameResult.WON)"
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 34
        },
        {
          "op": "assertLibraryCount",
          "player": 0,
          "name": "Approach of the Second Sun",
          "count": 0
        }
      ]
    },
    {
      "name": "testCastCopy",
      "operations": [
        {
          "op": "clearZone",
          "player": 0,
          "zone": "library"
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Plains",
          "count": 6
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Approach of the Second Sun",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Fork",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mystic Monastery",
          "count": 18
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Approach of the Second Sun"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Fork",
          "target": "Approach of the Second Sun"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 2
        },
        {
          "op": "assertStackSize",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "checkLife(\"Copy of Approach of the Second Sun gains life\", 1, PhaseStep.PRECOMBAT_MAIN, playerA, 27)"
        },
        {
          "op": "unsupported",
          "source": "checkLibraryCount(\"Copy of Approach of the Second Sun does not put a card in library\", 1, PhaseStep.PRECOMBAT_MAIN, playerA, \"Approach of the Second Sun\", 0)"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "unsupported",
          "source": "checkLife(\"Approach of the Second Sun gains life\", 1, PhaseStep.PRECOMBAT_MAIN, playerA, 34)"
        },
        {
          "op": "unsupported",
          "source": "checkLibraryCount(\"Approach of the Second Sun goes to library\", 1, PhaseStep.PRECOMBAT_MAIN, playerA, \"Approach of the Second Sun\", 1)"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Approach of the Second Sun"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Fork",
          "target": "Approach of the Second Sun"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 2
        },
        {
          "op": "unsupported",
          "source": "checkLife(\"Copy of 2nd Approach of the Second Sun gains life\", 1, PhaseStep.POSTCOMBAT_MAIN, playerA, 41)"
        },
        {
          "op": "unsupported",
          "source": "checkLibraryCount(\"Copy of 2nd Approach of the Second Sun does not put a card in library\", 1, PhaseStep.POSTCOMBAT_MAIN, playerA, \"Approach of the Second Sun\", 1)"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        },
        {
          "op": "unsupported",
          "source": "assertResult(playerA, GameResult.WON)"
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 41
        },
        {
          "op": "assertLibraryCount",
          "player": 0,
          "name": "Approach of the Second Sun",
          "count": 1
        }
      ]
    },
    {
      "name": "testFirstCountered",
      "operations": [
        {
          "op": "clearZone",
          "player": 0,
          "zone": "library"
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Approach of the Second Sun",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Counterspell",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Tundra",
          "count": 16
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Approach of the Second Sun"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Counterspell",
          "target": "Approach of the Second Sun"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Approach of the Second Sun"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 20
        },
        {
          "op": "assertLibraryCount",
          "player": 0,
          "name": "Approach of the Second Sun",
          "count": 0
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Approach of the Second Sun",
          "count": 2
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "count": 0
        },
        {
          "op": "unsupported",
          "source": "assertResult(playerA, GameResult.WON)"
        }
      ]
    },
    {
      "name": "testSecondCountered",
      "operations": [
        {
          "op": "clearZone",
          "player": 0,
          "zone": "library"
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Approach of the Second Sun",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Counterspell",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Tundra",
          "count": 16
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Approach of the Second Sun"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Approach of the Second Sun"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Counterspell",
          "target": "Approach of the Second Sun"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 27
        },
        {
          "op": "assertLibraryCount",
          "player": 0,
          "name": "Approach of the Second Sun",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Approach of the Second Sun",
          "count": 1
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "count": 0
        },
        {
          "op": "unsupported",
          "source": "assertResult(playerA, GameResult.DRAW)"
        }
      ]
    }
  ]
});
