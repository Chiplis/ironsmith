import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/roe/ChampionsDrakeTest.java",
  "tests": [
    {
      "name": "testCondition",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Champion's Drake",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Halimar Wavewatch",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Wastes",
          "count": 4
        },
        {
          "op": "unsupported",
          "source": "checkPermanentCounters(\"0 level counters\", 1, PhaseStep.UPKEEP, playerA, wavewatch, CounterType.LEVEL, 0)"
        },
        {
          "op": "assertPowerToughness",
          "turn": 1,
          "phase": "UPKEEP",
          "player": 0,
          "name": "Champion's Drake",
          "power": 1,
          "toughness": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "Level up"
        },
        {
          "op": "unsupported",
          "source": "checkPermanentCounters(\"1 level counter\", 1, PhaseStep.BEGIN_COMBAT, playerA, wavewatch, CounterType.LEVEL, 1)"
        },
        {
          "op": "assertPowerToughness",
          "turn": 1,
          "phase": "BEGIN_COMBAT",
          "player": 0,
          "name": "Champion's Drake",
          "power": 1,
          "toughness": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "ability": "Level up"
        },
        {
          "op": "unsupported",
          "source": "checkPermanentCounters(\"2 level counters\", 1, PhaseStep.END_TURN, playerA, wavewatch, CounterType.LEVEL, 2)"
        },
        {
          "op": "assertPowerToughness",
          "turn": 1,
          "phase": "END_TURN",
          "player": 0,
          "name": "Champion's Drake",
          "power": 1,
          "toughness": 1
        },
        {
          "op": "activateAbility",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "Level up"
        },
        {
          "op": "unsupported",
          "source": "checkPermanentCounters(\"3 level counters\", 3, PhaseStep.BEGIN_COMBAT, playerA, wavewatch, CounterType.LEVEL, 3)"
        },
        {
          "op": "assertPowerToughness",
          "turn": 3,
          "phase": "BEGIN_COMBAT",
          "player": 0,
          "name": "Champion's Drake",
          "power": 4,
          "toughness": 4
        },
        {
          "op": "activateAbility",
          "turn": 3,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "ability": "Level up"
        },
        {
          "op": "unsupported",
          "source": "checkPermanentCounters(\"4 level counters\", 3, PhaseStep.END_TURN, playerA, wavewatch, CounterType.LEVEL, 4)"
        },
        {
          "op": "assertPowerToughness",
          "turn": 3,
          "phase": "END_TURN",
          "player": 0,
          "name": "Champion's Drake",
          "power": 4,
          "toughness": 4
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 4,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Halimar Wavewatch",
          "power": 0,
          "toughness": 6
        }
      ]
    }
  ]
});
