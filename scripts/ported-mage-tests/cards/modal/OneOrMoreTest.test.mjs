import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/modal/OneOrMoreTest.java",
  "tests": [
    {
      "name": "test_ChooseModes_AsCardOrder",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 6
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Sublime Epiphany",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Sublime Epiphany"
        },
        {
          "op": "setModeChoice",
          "player": 0,
          "value": "3"
        },
        {
          "op": "setModeChoice",
          "player": 0,
          "value": "4"
        },
        {
          "op": "setModeChoice",
          "player": 0,
          "value": "5"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Silvercoat Lion"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Silvercoat Lion"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": 1
        },
        {
          "op": "setModeChoice",
          "player": 0,
          "value": "TestPlayer.MODE_SKIP"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
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
          "op": "assertHandCount",
          "player": 0,
          "count": 1,
          "name": 1
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Silvercoat Lion",
          "power": 2,
          "toughness": 2
        }
      ]
    },
    {
      "name": "test_ChooseModes_AsCustomOrder",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 6
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Sublime Epiphany",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Sublime Epiphany"
        },
        {
          "op": "setModeChoice",
          "player": 0,
          "value": "4"
        },
        {
          "op": "setModeChoice",
          "player": 0,
          "value": "5"
        },
        {
          "op": "setModeChoice",
          "player": 0,
          "value": "3"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Silvercoat Lion"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Silvercoat Lion"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": 1
        },
        {
          "op": "setModeChoice",
          "player": 0,
          "value": "TestPlayer.MODE_SKIP"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
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
          "op": "assertHandCount",
          "player": 0,
          "count": 1,
          "name": 1
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Silvercoat Lion",
          "power": 2,
          "toughness": 2
        }
      ]
    }
  ]
});
