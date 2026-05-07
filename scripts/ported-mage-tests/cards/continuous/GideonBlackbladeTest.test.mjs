import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/continuous/GideonBlackbladeTest.java",
  "tests": [
    {
      "name": "test_PreventDamageToGideonOnYourTurn",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Gideon Blackblade",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Lightning Bolt",
          "count": 2
        },
        {
          "op": "assertPowerToughness",
          "player": 1,
          "name": "PRECOMBAT_MAIN",
          "power": 0,
          "toughness": "Gideon Blackblade"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "BEGIN_COMBAT",
          "player": 0,
          "name": "Lightning Bolt",
          "target": "Gideon Blackblade"
        },
        {
          "op": "assertPowerToughness",
          "player": 1,
          "name": "POSTCOMBAT_MAIN",
          "power": 0,
          "toughness": "Gideon Blackblade"
        },
        {
          "op": "unsupported",
          "source": "checkPermanentCounters(\"turn 1 after\", 1, PhaseStep.POSTCOMBAT_MAIN, playerA, \"Gideon Blackblade\", CounterType.LOYALTY, 4)"
        },
        {
          "op": "assertPowerToughness",
          "player": "turn 2 before",
          "name": 2,
          "power": "PRECOMBAT_MAIN",
          "toughness": 0
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "BEGIN_COMBAT",
          "player": 0,
          "name": "Lightning Bolt",
          "target": "Gideon Blackblade"
        },
        {
          "op": "assertPowerToughness",
          "player": "turn 2 after",
          "name": 2,
          "power": "POSTCOMBAT_MAIN",
          "toughness": 0
        },
        {
          "op": "unsupported",
          "source": "checkPermanentCounters(\"turn 2 after\", 2, PhaseStep.POSTCOMBAT_MAIN, playerA, \"Gideon Blackblade\", CounterType.LOYALTY, 4 - 3)"
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
        }
      ]
    }
  ]
});
