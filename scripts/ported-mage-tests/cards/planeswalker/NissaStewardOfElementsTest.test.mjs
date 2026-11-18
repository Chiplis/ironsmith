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
          "value": "X=0"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "text": "Cast Nissa, Steward of Elements",
          "count": 1
        },
        {
          "op": "assertTappedCount",
          "player": 0,
          "name": "Tropical Island",
          "count": 2,
          "tapped": true
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
          "op": "sequence",
          "operations": [
            {
              "op": "assertGraveyardCount",
              "player": 0,
              "name": "Nissa, Steward of Elements",
              "count": 1
            }
          ]
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
          "value": "X=1"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "text": "Cast Nissa, Steward of Elements",
          "count": 1
        },
        {
          "op": "assertTappedCount",
          "player": 0,
          "name": "Tropical Island",
          "count": 3,
          "tapped": true
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
          "op": "sequence",
          "operations": [
            {
              "op": "assertCounterCount",
              "player": 0,
              "name": "Nissa, Steward of Elements",
              "counter": "LOYALTY",
              "count": 1
            }
          ]
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
          "value": "X=2"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "text": "Cast Nissa, Steward of Elements",
          "count": 1
        },
        {
          "op": "assertTappedCount",
          "player": 0,
          "name": "Tropical Island",
          "count": 4,
          "tapped": true
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
          "op": "sequence",
          "operations": [
            {
              "op": "assertCounterCount",
              "player": 0,
              "name": "Nissa, Steward of Elements",
              "counter": "LOYALTY",
              "count": 2
            }
          ]
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
          "value": "X=10"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "text": "Cast Nissa, Steward of Elements",
          "count": 1
        },
        {
          "op": "assertTappedCount",
          "player": 0,
          "name": "Tropical Island",
          "count": 12,
          "tapped": true
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
          "op": "sequence",
          "operations": [
            {
              "op": "assertCounterCount",
              "player": 0,
              "name": "Nissa, Steward of Elements",
              "counter": "LOYALTY",
              "count": 10
            }
          ]
        }
      ]
    }
  ]
});
