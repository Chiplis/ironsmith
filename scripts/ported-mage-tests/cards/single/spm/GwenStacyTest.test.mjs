import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/spm/GwenStacyTest.java",
  "tests": [
    {
      "name": "testGhostSpider",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Gwen Stacy",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plateau",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Ghost-Spider"
        },
        {
          "op": "unsupported",
          "source": "addCounters(1, PhaseStep.PRECOMBAT_MAIN, playerA, ghostSpider, CounterType.P1P1, 1)"
        },
        {
          "op": "unsupported",
          "source": "addCounters(1, PhaseStep.PRECOMBAT_MAIN, playerA, ghostSpider, CounterType.CHARGE, 1)"
        },
        {
          "op": "unsupported",
          "source": "addCounters(1, PhaseStep.PRECOMBAT_MAIN, playerA, ghostSpider, CounterType.P0P1, 1)"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "ability": "Remove two counters"
        },
        {
          "op": "unsupported",
          "source": "setChoiceAmount(playerA, 1)"
        },
        {
          "op": "unsupported",
          "source": "setChoiceAmount(playerA, 1)"
        },
        {
          "op": "unsupported",
          "source": "setChoiceAmount(playerA, 0)"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": null
        },
        {
          "op": "playLand",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Mountain"
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
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Ghost-Spider",
          "power": 6,
          "toughness": 6
        }
      ]
    }
  ]
});
