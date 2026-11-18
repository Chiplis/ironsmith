import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/dmc/TillerEngineTest.java",
  "tests": [
    {
      "name": "testFirstMode",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Tiller Engine",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Centaur Courser",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Bloodfell Caves",
          "count": 1
        },
        {
          "op": "playLand",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Bloodfell Caves"
        },
        {
          "op": "setOptionChoices",
          "player": 0,
          "kind": "options",
          "choices": [
            "Untap that land."
          ],
          "description": "Choose a mode"
        },
        {
          "op": "setStrictChooseMode",
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
          "op": "assertLife",
          "player": 0,
          "life": 21
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 20
        },
        {
          "op": "assertTapped",
          "name": "Bloodfell Caves",
          "tapped": false,
          "player": 0
        },
        {
          "op": "assertTapped",
          "name": "Centaur Courser",
          "tapped": false,
          "player": 1
        }
      ]
    },
    {
      "name": "testSecondMode",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Tiller Engine",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Centaur Courser",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Bloodfell Caves",
          "count": 1
        },
        {
          "op": "playLand",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Bloodfell Caves"
        },
        {
          "op": "setOptionChoices",
          "player": 0,
          "kind": "options",
          "choices": [
            "Tap target nonland permanent an opponent controls."
          ],
          "description": "Choose a mode"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Centaur Courser"
        },
        {
          "op": "setStrictChooseMode",
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
          "op": "assertLife",
          "player": 0,
          "life": 21
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 20
        },
        {
          "op": "assertTapped",
          "name": "Bloodfell Caves",
          "tapped": true,
          "player": 0
        },
        {
          "op": "assertTapped",
          "name": "Centaur Courser",
          "tapped": true,
          "player": 1
        }
      ]
    }
  ]
});
