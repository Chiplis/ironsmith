import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/ltc/GandalfOfTheSecretFireTest.java",
  "tests": [
    {
      "name": "test_yourturn",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Gandalf of the Secret Fire",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Lightning Bolt",
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
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Lightning Bolt",
          "target": 1
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 17
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Lightning Bolt",
          "count": 1
        }
      ]
    },
    {
      "name": "test_oppturn_suspend",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Gandalf of the Secret Fire",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Lightning Bolt",
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
          "op": "castSpell",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Lightning Bolt",
          "target": 1
        },
        {
          "op": "assertExileCount",
          "turn": 2,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Lightning Bolt",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "checkCardCounters(\"1: bolt has 3 time counters\", 2, PhaseStep.POSTCOMBAT_MAIN, playerA, \"Lightning Bolt\", CounterType.TIME, 3)"
        },
        {
          "op": "unsupported",
          "source": "checkCardCounters(\"2: bolt has 2 time counters\", 3, PhaseStep.PRECOMBAT_MAIN, playerA, \"Lightning Bolt\", CounterType.TIME, 2)"
        },
        {
          "op": "unsupported",
          "source": "checkCardCounters(\"3: bolt has 1 time counters\", 5, PhaseStep.PRECOMBAT_MAIN, playerA, \"Lightning Bolt\", CounterType.TIME, 1)"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": 1
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 7,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 14
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Lightning Bolt",
          "count": 1
        }
      ]
    },
    {
      "name": "test_split_suspend",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Gandalf of the Secret Fire",
          "count": 1
        }
      ]
    },
    {
      "name": "test_oppturn_counterspell_suspend",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Gandalf of the Secret Fire",
          "count": 1
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
          "name": "Island",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Lightning Bolt",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Mountain",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Lightning Bolt",
          "target": 1
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Counterspell",
          "target": "Lightning Bolt"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 20
        },
        {
          "op": "assertExileCount",
          "player": 0,
          "name": "Counterspell",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Lightning Bolt",
          "count": 1
        }
      ]
    },
    {
      "name": "test_oppturn_countered_nosuspend",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Gandalf of the Secret Fire",
          "count": 1
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
          "name": "Island",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Lightning Bolt",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Mountain",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Lightning Bolt",
          "target": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Counterspell",
          "target": "Lightning Bolt"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 20
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Counterspell",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Lightning Bolt",
          "count": 1
        }
      ]
    },
    {
      "name": "test_adventure_nosuspend",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Gandalf of the Secret Fire",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Bonecrusher Giant",
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
          "op": "castSpell",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Stomp",
          "target": 1
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 18
        },
        {
          "op": "assertExileCount",
          "player": 0,
          "name": "Bonecrusher Giant",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "assertCounterOnExiledCardCount(\"Bonecrusher Giant\", CounterType.TIME, 0)"
        }
      ]
    }
  ]
});
