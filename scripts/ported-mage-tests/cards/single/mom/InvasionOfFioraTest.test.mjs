import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/mom/InvasionOfFioraTest.java",
  "tests": [
    {
      "name": "testSiege",
      "operations": [
        {
          "op": "skipInitShuffling"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setLife",
          "player": 0,
          "life": 20
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Invasion of Fiora",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Bellowing Bruiser",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 6
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Blood Researcher",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Emrakul, the Promised End",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Swamp",
          "count": 5
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Lich's Caress",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Invasion of Fiora"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 1
        },
        {
          "op": "setModeChoice",
          "player": 0,
          "value": "1"
        },
        {
          "op": "setModeChoice",
          "player": 0,
          "value": "TestPlayer.MODE_SKIP"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 1
        },
        {
          "op": "assertPermanentCount",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Invasion of Fiora",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Emrakul, the Promised End",
          "count": 0
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Bellowing Bruiser",
          "defender": "Invasion of Fiora"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "assertPermanentCount",
          "turn": 1,
          "phase": "END_TURN",
          "player": 0,
          "name": "Marchesa, Resolute Monarch",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Lich's Caress",
          "target": "Bellowing Bruiser"
        },
        {
          "op": "waitStackResolved",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "unsupported",
          "source": "checkPermanentCounters(\"Researcher gained one counter\", 2, PhaseStep.BEGIN_COMBAT, playerB, researcher, CounterType.P1P1, 1)"
        },
        {
          "op": "attack",
          "turn": 2,
          "player": 1,
          "attacker": "Blood Researcher",
          "defender": 1
        },
        {
          "op": "unsupported",
          "source": "checkLife(\"PlayerA life after turn 2\", 2, PhaseStep.END_TURN, playerA, 17)"
        },
        {
          "op": "assertHandCount",
          "player": "PlayerA hand after turn 2",
          "name": 2,
          "count": "END_TURN"
        },
        {
          "op": "assertHandCount",
          "player": "PlayerA hand after turn 3 draw",
          "name": 3,
          "count": "PRECOMBAT_MAIN"
        },
        {
          "op": "unsupported",
          "source": "checkLife(\"PlayerA life after turn 3 draw\", 3, PhaseStep.PRECOMBAT_MAIN, playerA, 17)"
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Marchesa, Resolute Monarch",
          "defender": 1
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Blood Researcher"
        },
        {
          "op": "unsupported",
          "source": "checkPermanentCounters(\"Researcher lost its counter\", 3, PhaseStep.END_COMBAT, playerB, researcher, CounterType.P1P1, 0)"
        },
        {
          "op": "assertHandCount",
          "player": "PlayerA hand in turn 5 precombat main",
          "name": 5,
          "count": "PRECOMBAT_MAIN"
        },
        {
          "op": "unsupported",
          "source": "checkLife(\"PlayerA life in turn 5 precombat main\", 5, PhaseStep.PRECOMBAT_MAIN, playerA, 16)"
        },
        {
          "op": "setStopAt",
          "turn": 5,
          "phase": "DECLARE_ATTACKERS"
        },
        {
          "op": "execute"
        }
      ]
    }
  ]
});
