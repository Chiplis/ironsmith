import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/usg/SporogenesisTest.java",
  "tests": [
    {
      "name": "sporogenesisTest",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Sporogenesis",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Skophos Warleader",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Barony Vampire",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Warpath Ghoul",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Douser of Lights",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Skophos Warleader"
        },
        {
          "op": "unsupported",
          "source": "checkPermanentCounters(\"no fungus\", 1, PhaseStep.END_TURN, playerA, warleader, CounterType.FUNGUS, 1)"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Skophos Warleader"
        },
        {
          "op": "unsupported",
          "source": "checkPermanentCounters(\"fungus\", 3, PhaseStep.END_TURN, playerA, warleader, CounterType.FUNGUS, 2)"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Barony Vampire"
        },
        {
          "op": "unsupported",
          "source": "checkPermanentCounters(\"fungus\", 5, PhaseStep.END_TURN, playerA, vampire, CounterType.FUNGUS, 1)"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Warpath Ghoul"
        },
        {
          "op": "unsupported",
          "source": "checkPermanentCounters(\"fungus\", 7, PhaseStep.END_TURN, playerA, ghoul, CounterType.FUNGUS, 1)"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Warpath Ghoul"
        },
        {
          "op": "unsupported",
          "source": "checkPermanentCounters(\"fungus\", 9, PhaseStep.PRECOMBAT_MAIN, playerA, ghoul, CounterType.FUNGUS, 2)"
        },
        {
          "op": "activateAbility",
          "turn": 9,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "ability": "{R}, Sacrifice"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Warpath Ghoul"
        },
        {
          "op": "assertPowerToughness",
          "player": "boost",
          "name": 9,
          "power": "END_TURN",
          "toughness": 0
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Douser of Lights"
        },
        {
          "op": "unsupported",
          "source": "checkPermanentCounters(\"fungus\", 11, PhaseStep.END_TURN, playerA, douser, CounterType.FUNGUS, 1)"
        },
        {
          "op": "activateAbility",
          "turn": 12,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "ability": "{R}, Sacrifice"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Sporogenesis"
        },
        {
          "op": "unsupported",
          "source": "checkPermanentCounters(\"fungus\", 12, PhaseStep.END_TURN, playerA, warleader, CounterType.FUNGUS, 0)"
        },
        {
          "op": "unsupported",
          "source": "checkPermanentCounters(\"fungus\", 12, PhaseStep.END_TURN, playerA, vampire, CounterType.FUNGUS, 0)"
        },
        {
          "op": "unsupported",
          "source": "checkPermanentCounters(\"fungus\", 12, PhaseStep.END_TURN, playerA, douser, CounterType.FUNGUS, 0)"
        },
        {
          "op": "assertPowerToughness",
          "player": "boost",
          "name": 12,
          "power": "END_TURN",
          "toughness": 0
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 13,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Warpath Ghoul",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Sporogenesis",
          "count": 1
        }
      ]
    }
  ]
});
