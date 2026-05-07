import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/hou/TormentOfHailfireTest.java",
  "tests": [
    {
      "name": "test_Normal",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Torment of Hailfire",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 12
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Silvercoat Lion",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Plains",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": "playerC",
          "name": "Silvercoat Lion",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": "playerD",
          "name": "Silvercoat Lion",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": "playerD",
          "name": "Plains",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Torment of Hailfire"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "X=10"
        },
        {
          "op": "setChoice",
          "player": "playerD",
          "value": true
        },
        {
          "op": "setChoice",
          "player": "playerD",
          "value": "Silvercoat Lion"
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": true
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": "Silvercoat Lion"
        },
        {
          "op": "setChoice",
          "player": "playerD",
          "value": true
        },
        {
          "op": "setChoice",
          "player": "playerD",
          "value": "Silvercoat Lion"
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": true
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": "Silvercoat Lion"
        },
        {
          "op": "setChoice",
          "player": "playerD",
          "value": false
        },
        {
          "op": "setChoice",
          "player": "playerD",
          "value": true
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": true
        },
        {
          "op": "setChoice",
          "player": "playerD",
          "value": true
        },
        {
          "op": "setChoice",
          "player": "playerD",
          "value": "Silvercoat Lion"
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
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Torment of Hailfire",
          "count": 1
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 20
        },
        {
          "op": "assertLife",
          "player": "playerC",
          "life": 20
        },
        {
          "op": "assertLife",
          "player": "playerD",
          "life": 2
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": -1
        }
      ]
    }
  ]
});
