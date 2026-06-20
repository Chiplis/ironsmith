import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/triggers/RadCounterTriggerTest.java",
  "tests": [
    {
      "name": "test_Fallout_3_Multiple_Turns",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "skipInitShuffling"
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Nuclear Fallout",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 5
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Yawgmoth's Bargain",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Yawgmoth's Bargain",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Tundra",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Akoum Warrior",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Tropical Island",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Lightning Bolt",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Bayou",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 1,
          "name": "Elite Vanguard",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 1,
          "name": "Fire // Ice",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 1,
          "name": "Brainstorm",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 1,
          "name": "Plains",
          "count": 2
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "X=3"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Nuclear Fallout"
        },
        {
          "op": "unsupported",
          "source": "checkLife(\"life playerB turn 2\", 2, PhaseStep.POSTCOMBAT_MAIN, playerB, 19)"
        },
        {
          "op": "unsupported",
          "source": "checkLife(\"life playerA turn 3\", 3, PhaseStep.POSTCOMBAT_MAIN, playerA, 20)"
        },
        {
          "op": "unsupported",
          "source": "checkLife(\"life playerB turn 4\", 4, PhaseStep.POSTCOMBAT_MAIN, playerB, 17)"
        },
        {
          "op": "unsupported",
          "source": "checkLife(\"life playerA turn 5\", 5, PhaseStep.POSTCOMBAT_MAIN, playerA, 18)"
        },
        {
          "op": "unsupported",
          "source": "checkLife(\"life playerB turn 6\", 6, PhaseStep.POSTCOMBAT_MAIN, playerB, 17)"
        },
        {
          "op": "unsupported",
          "source": "checkLife(\"life playerA turn 7\", 7, PhaseStep.POSTCOMBAT_MAIN, playerA, 18)"
        },
        {
          "op": "setStopAt",
          "turn": 7,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        }
      ]
    }
  ]
});
