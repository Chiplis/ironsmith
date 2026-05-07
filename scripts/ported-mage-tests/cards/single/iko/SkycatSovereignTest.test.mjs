import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/iko/SkycatSovereignTest.java",
  "tests": [
    {
      "name": "test_BoostFromFlyers",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Skycat Sovereign",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Abbey Griffin",
          "count": 2
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
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Skycat Sovereign",
          "power": 3,
          "toughness": 3
        }
      ]
    },
    {
      "name": "test_NoBoostIfFlyingLost",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Skycat Sovereign",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Abbey Griffin",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Archetype of Imagination",
          "count": 1
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
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Skycat Sovereign",
          "power": 1,
          "toughness": 1
        }
      ]
    },
    {
      "name": "test_BoostFromToken",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Skycat Sovereign",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Abbey Griffin",
          "count": 2
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{2}{W}{U}: Create"
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
          "name": "Cat Bird Token",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "assertColor(playerA, \"Cat Bird Token\", ObjectColor.WHITE, true)"
        },
        {
          "op": "unsupported",
          "source": "assertColor(playerA, \"Cat Bird Token\", ObjectColor.BLUE, false)"
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Cat Bird Token",
          "ability": "Flying",
          "expected": true
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Skycat Sovereign",
          "power": 4,
          "toughness": 4
        }
      ]
    }
  ]
});
