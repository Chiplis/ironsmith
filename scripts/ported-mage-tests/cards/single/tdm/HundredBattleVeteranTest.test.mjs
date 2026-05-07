import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/tdm/HundredBattleVeteranTest.java",
  "tests": [
    {
      "name": "testBoostEffect",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Hundred-Battle Veteran",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Bear Cub",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "addCounters(1, PhaseStep.PRECOMBAT_MAIN, playerA, CUB, CounterType.FINALITY, 1)"
        },
        {
          "op": "unsupported",
          "source": "addCounters(1, PhaseStep.PRECOMBAT_MAIN, playerA, CUB, CounterType.P1P1, 3)"
        },
        {
          "op": "assertPowerToughness",
          "player": 1,
          "name": "BEGIN_COMBAT",
          "power": 0,
          "toughness": "Hundred-Battle Veteran"
        },
        {
          "op": "unsupported",
          "source": "addCounters(1, PhaseStep.POSTCOMBAT_MAIN, playerA, CUB, CounterType.LIFELINK, 1)"
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
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Bear Cub",
          "count": 1
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Hundred-Battle Veteran",
          "counter": "FINALITY",
          "count": 0
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Hundred-Battle Veteran",
          "power": 6,
          "toughness": 6
        }
      ]
    },
    {
      "name": "testCastFromGraveyard",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Hundred-Battle Veteran",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 4
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Hundred-Battle Veteran"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Hundred-Battle Veteran",
          "count": 1
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Hundred-Battle Veteran",
          "counter": "FINALITY",
          "count": 1
        }
      ]
    }
  ]
});
