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
          "count": 1,
          "tapped": true
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
          "player": null,
          "resolutions": 1
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
          "player": null,
          "resolutions": 1
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
          "op": "assertCounterCount",
          "turn": 2,
          "phase": "BEGIN_COMBAT",
          "player": 1,
          "name": "Blood Researcher",
          "counter": "P1P1",
          "count": 1,
          "seatIndependent": true
        },
        {
          "op": "attack",
          "turn": 2,
          "player": 1,
          "attacker": "Blood Researcher",
          "defender": 0
        },
        {
          "op": "unsupported",
          "source": "checkLife(\"PlayerA life after turn 2\", 2, PhaseStep.END_TURN, playerA, 17)"
        },
        {
          "op": "assertHandCount",
          "turn": 2,
          "phase": "END_TURN",
          "player": 0,
          "count": 0
        },
        {
          "op": "assertHandCount",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "count": 1
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
          "op": "assertCounterCount",
          "turn": 3,
          "phase": "END_COMBAT",
          "player": 1,
          "name": "Blood Researcher",
          "counter": "P1P1",
          "count": 0,
          "seatIndependent": true
        },
        {
          "op": "assertHandCount",
          "turn": 5,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "count": 3
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
      ],
      "startingLife": 20
    }
  ]
});
