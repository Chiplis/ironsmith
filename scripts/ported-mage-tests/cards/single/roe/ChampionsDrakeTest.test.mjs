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
          "player": 1,
          "name": "UPKEEP",
          "power": 0,
          "toughness": "Champion's Drake"
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
          "player": 1,
          "name": "BEGIN_COMBAT",
          "power": 0,
          "toughness": "Champion's Drake"
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
          "player": 1,
          "name": "END_TURN",
          "power": 0,
          "toughness": "Champion's Drake"
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
          "player": "3 level counters",
          "name": 3,
          "power": "BEGIN_COMBAT",
          "toughness": 0
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
          "player": "4 level counters",
          "name": 3,
          "power": "END_TURN",
          "toughness": 0
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
