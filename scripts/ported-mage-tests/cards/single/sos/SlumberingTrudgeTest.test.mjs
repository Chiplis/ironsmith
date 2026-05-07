import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/sos/SlumberingTrudgeTest.java",
  "tests": [
    {
      "name": "test_X_0",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Slumbering Trudge",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 4
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Slumbering Trudge"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "X=0"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Slumbering Trudge",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "assertTapped(trudge, true)"
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Slumbering Trudge",
          "counter": "STUN",
          "count": 3
        }
      ]
    },
    {
      "name": "test_X_1",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Slumbering Trudge",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 4
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Slumbering Trudge"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "X=1"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Slumbering Trudge",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "assertTapped(trudge, true)"
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Slumbering Trudge",
          "counter": "STUN",
          "count": 2
        }
      ]
    },
    {
      "name": "test_X_2",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Slumbering Trudge",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 4
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Slumbering Trudge"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "X=2"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Slumbering Trudge",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "assertTapped(trudge, true)"
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Slumbering Trudge",
          "counter": "STUN",
          "count": 1
        }
      ]
    },
    {
      "name": "test_X_3",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Slumbering Trudge",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 4
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Slumbering Trudge"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "X=3"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Slumbering Trudge",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "assertTapped(trudge, false)"
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Slumbering Trudge",
          "counter": "STUN",
          "count": 0
        }
      ]
    }
  ]
});
