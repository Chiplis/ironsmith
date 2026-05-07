import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/iko/TheOzolithTest.java",
  "tests": [
    {
      "name": "testTheOzolithGetCounters",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Bayou",
          "count": 6
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "The Ozolith",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Fertilid",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Fully Grown",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Murder",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Fully Grown",
          "target": "Fertilid"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Murder",
          "target": "Fertilid"
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
          "name": "Fertilid",
          "count": 0
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "The Ozolith",
          "counter": "P1P1",
          "count": 2
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "The Ozolith",
          "counter": "TRAMPLE",
          "count": 1
        }
      ]
    },
    {
      "name": "testTheOzolithGiveCounters",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Bayou",
          "count": 6
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "The Ozolith",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Fertilid",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Walking Corpse",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Fully Grown",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Murder",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Fully Grown",
          "target": "Fertilid"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Murder",
          "target": "Fertilid"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
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
          "name": "Fertilid",
          "count": 0
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Walking Corpse",
          "counter": "P1P1",
          "count": 2
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Walking Corpse",
          "counter": "TRAMPLE",
          "count": 1
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "The Ozolith",
          "counter": "P1P1",
          "count": 0
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "The Ozolith",
          "counter": "TRAMPLE",
          "count": 0
        }
      ]
    },
    {
      "name": "testTheOzolithCantGiveAnyCounters",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "The Ozolith",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Fertilid",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Tatterkite",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Murder",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Murder",
          "target": "Fertilid"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
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
          "name": "Fertilid",
          "count": 0
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Tatterkite",
          "counter": "P1P1",
          "count": 0
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "The Ozolith",
          "counter": "P1P1",
          "count": 2
        }
      ]
    },
    {
      "name": "testTheOzolithCantGiveSomeCounters",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Bayou",
          "count": 6
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "The Ozolith",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Fertilid",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Walking Corpse",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Fully Grown",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Murder",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Blightbeetle",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Fully Grown",
          "target": "Fertilid"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Murder",
          "target": "Fertilid"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
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
          "name": "Fertilid",
          "count": 0
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Walking Corpse",
          "counter": "P1P1",
          "count": 0
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Walking Corpse",
          "counter": "TRAMPLE",
          "count": 1
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "The Ozolith",
          "counter": "P1P1",
          "count": 2
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "The Ozolith",
          "counter": "TRAMPLE",
          "count": 0
        }
      ]
    },
    {
      "name": "testTheOzolithMultiples",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Bayou",
          "count": 6
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "The Ozolith",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mirror Gallery",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Fertilid",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Walking Corpse",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Fully Grown",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Murder",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Fully Grown",
          "target": "Fertilid"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Murder",
          "target": "Fertilid"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
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
          "name": "Fertilid",
          "count": 0
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Walking Corpse",
          "counter": "P1P1",
          "count": 4
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Walking Corpse",
          "counter": "TRAMPLE",
          "count": 2
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "The Ozolith",
          "counter": "P1P1",
          "count": 0
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "The Ozolith",
          "counter": "TRAMPLE",
          "count": 0
        }
      ]
    },
    {
      "name": "testTheOzolithMinusPlus",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Taiga",
          "count": 9
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "The Ozolith",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "March of the Machines",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Fertilid",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Branching Evolution",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Puncture Blast",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Fertilid"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Branching Evolution"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Puncture Blast",
          "target": "Fertilid"
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
          "name": "Fertilid",
          "count": 0
        },
        {
          "op": "unsupported",
          "source": "assertType(THE_OZOLITH, CardType.CREATURE, true)"
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "The Ozolith",
          "counter": "P1P1",
          "count": 1
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "The Ozolith",
          "counter": "M1M1",
          "count": 0
        }
      ]
    }
  ]
});
