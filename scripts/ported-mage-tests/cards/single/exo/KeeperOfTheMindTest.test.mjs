import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/exo/KeeperOfTheMindTest.java",
  "tests": [
    {
      "name": "simpleActivate",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Keeper of the Mind",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Island",
          "count": 2
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{U}, {T}: Choose target opponent "
        },
        {
          "op": "setStrictChooseMode",
          "value": false
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
          "count": 0,
          "name": 1
        }
      ]
    },
    {
      "name": "activateOtherPlayerTurn",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Keeper of the Mind",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Island",
          "count": 2
        },
        {
          "op": "activateAbility",
          "turn": 2,
          "phase": "UPKEEP",
          "player": 0,
          "ability": "{U}, {T}: Choose target opponent "
        },
        {
          "op": "setStrictChooseMode",
          "value": false
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "DRAW"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "count": 0,
          "name": 1
        }
      ]
    },
    {
      "name": "cantActivateIfCardDifferenceTooLow",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Keeper of the Mind",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Island",
          "count": 1
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "label": "{U}, {T}: Choose target opponent ",
          "expected": false
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
          "count": 0,
          "name": 0
        }
      ]
    },
    {
      "name": "doubleActivate",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Keeper of the Mind",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Island",
          "count": 2
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{U}, {T}: Choose target opponent "
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{U}, {T}: Choose target opponent "
        },
        {
          "op": "setStrictChooseMode",
          "value": false
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
          "count": 0,
          "name": 2
        }
      ]
    }
  ]
});
