import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/mh3/SorinOfHouseMarkovTest.java",
  "tests": [
    {
      "name": "test_Gain2Life_NoTrigger",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Sorin of House Markov",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Courier Griffin",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Courier Griffin"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": false
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
          "name": "Courier Griffin",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Sorin of House Markov",
          "count": 1
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 22
        }
      ]
    },
    {
      "name": "test_Gain3Life_Trigger_ThenMinus1",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Sorin of House Markov",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 5
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Courier Griffin",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Courier Griffin"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "assertPermanentCount",
          "turn": 1,
          "phase": "BEGIN_COMBAT",
          "player": 0,
          "name": "Sorin of House Markov",
          "count": 1
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "ability": "-1",
          "target": 1
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
          "name": "Courier Griffin",
          "count": 1
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 23
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 16
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Sorin, Ravenous Neonate",
          "count": 1
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Sorin, Ravenous Neonate",
          "counter": "LOYALTY",
          "count": 2
        }
      ]
    },
    {
      "name": "test_Plus2_Plus2_Minus1",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Sorin, Ravenous Neonate",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 4
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "+2"
        },
        {
          "op": "activateAbility",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "+2"
        },
        {
          "op": "activateAbility",
          "turn": 5,
          "phase": "UPKEEP",
          "player": 0,
          "ability": "{2}, {T}, Sacrifice"
        },
        {
          "op": "activateAbility",
          "turn": 5,
          "phase": "UPKEEP",
          "player": 0,
          "ability": "{2}, {T}, Sacrifice"
        },
        {
          "op": "activateAbility",
          "turn": 5,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "-1"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": 1
        },
        {
          "op": "setStopAt",
          "turn": 5,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 26
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 14
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Sorin, Ravenous Neonate",
          "count": 1
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Sorin, Ravenous Neonate",
          "counter": "LOYALTY",
          "count": 6
        }
      ]
    },
    {
      "name": "test_Minus6_NoOtherWhite",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Sorin, Ravenous Neonate",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Grizzly Bears",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Elite Vanguard",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "addCounters(1, PhaseStep.PRECOMBAT_MAIN, playerA, sorinPW, CounterType.LOYALTY, 4)"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "-6"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Elite Vanguard"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Elite Vanguard",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "assertSubtype(\"Elite Vanguard\", SubType.HUMAN)"
        },
        {
          "op": "unsupported",
          "source": "assertSubtype(\"Elite Vanguard\", SubType.VAMPIRE)"
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Elite Vanguard",
          "counter": "LIFELINK",
          "count": 0
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Sorin, Ravenous Neonate",
          "counter": "LOYALTY",
          "count": 1
        }
      ]
    },
    {
      "name": "test_Minus6_OtherWhite",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Sorin, Ravenous Neonate",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Baneslayer Angel",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Elite Vanguard",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "addCounters(1, PhaseStep.PRECOMBAT_MAIN, playerA, sorinPW, CounterType.LOYALTY, 4)"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "-6"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Elite Vanguard"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Elite Vanguard",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "assertSubtype(\"Elite Vanguard\", SubType.HUMAN)"
        },
        {
          "op": "unsupported",
          "source": "assertSubtype(\"Elite Vanguard\", SubType.VAMPIRE)"
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Elite Vanguard",
          "counter": "LIFELINK",
          "count": 1
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Sorin, Ravenous Neonate",
          "counter": "LOYALTY",
          "count": 1
        }
      ]
    }
  ]
});
