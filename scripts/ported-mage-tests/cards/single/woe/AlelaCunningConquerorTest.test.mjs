import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/woe/AlelaCunningConquerorTest.java",
  "tests": [
    {
      "name": "attackSinglePlayer",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Alela, Cunning Conqueror",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Pestermite",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Grizzly Bears",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": "playerC",
          "name": "Ancient Carp",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": "playerD",
          "name": "Devoted Hero",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Alela, Cunning Conqueror",
          "defender": "playerC"
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Pestermite",
          "defender": "playerC"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Ancient Carp"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "END_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "unsupported",
          "source": "assertGoadedByPlayer(carp, playerA)"
        },
        {
          "op": "assertLife",
          "player": "playerC",
          "life": 36
        }
      ]
    },
    {
      "name": "attackTwoPlayers",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": false
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Alela, Cunning Conqueror",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Pestermite",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Grizzly Bears",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": "playerC",
          "name": "Ancient Carp",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": "playerD",
          "name": "Devoted Hero",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Alela, Cunning Conqueror",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Pestermite",
          "defender": "playerD"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "END_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "unsupported",
          "source": "assertGoadedByPlayer(bears, playerA)"
        },
        {
          "op": "unsupported",
          "source": "assertGoadedByPlayer(devoted, playerA)"
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 38
        },
        {
          "op": "assertLife",
          "player": "playerD",
          "life": 38
        }
      ]
    },
    {
      "name": "attackWithNotAFaerie",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Alela, Cunning Conqueror",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Storm Crow",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Grizzly Bears",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": "playerC",
          "name": "Ancient Carp",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": "playerD",
          "name": "Devoted Hero",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Alela, Cunning Conqueror",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Storm Crow",
          "defender": "playerD"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Grizzly Bears"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "END_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "unsupported",
          "source": "assertGoadedByPlayer(bears, playerA)"
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 38
        },
        {
          "op": "assertLife",
          "player": "playerD",
          "life": 39
        }
      ]
    }
  ]
});
