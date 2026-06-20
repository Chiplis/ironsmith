import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/planeswalker/NissaStewardOfElementsTest.java",
  "tests": [
    {
      "name": "test0Counters",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Tropical Island",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Nissa, Steward of Elements",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Nissa, Steward of Elements"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "\"X=\" + 0"
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
          "op": "unsupported",
          "source": "if (0 == 0) { assertGraveyardCount(playerA, nissa, 1); } else { assertCounterCount(playerA, nissa, CounterType.LOYALTY, 0); }"
        }
      ]
    },
    {
      "name": "test1Counter",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Tropical Island",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Nissa, Steward of Elements",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Nissa, Steward of Elements"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "\"X=\" + 1"
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
          "op": "unsupported",
          "source": "if (1 == 0) { assertGraveyardCount(playerA, nissa, 1); } else { assertCounterCount(playerA, nissa, CounterType.LOYALTY, 1); }"
        }
      ]
    },
    {
      "name": "test2Counters",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Tropical Island",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Nissa, Steward of Elements",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Nissa, Steward of Elements"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "\"X=\" + 2"
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
          "op": "unsupported",
          "source": "if (2 == 0) { assertGraveyardCount(playerA, nissa, 1); } else { assertCounterCount(playerA, nissa, CounterType.LOYALTY, 2); }"
        }
      ]
    },
    {
      "name": "test10Counters",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Tropical Island",
          "count": 12
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Nissa, Steward of Elements",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Nissa, Steward of Elements"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "\"X=\" + 10"
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
          "op": "unsupported",
          "source": "if (10 == 0) { assertGraveyardCount(playerA, nissa, 1); } else { assertCounterCount(playerA, nissa, CounterType.LOYALTY, 10); }"
        }
      ]
    }
  ]
});
