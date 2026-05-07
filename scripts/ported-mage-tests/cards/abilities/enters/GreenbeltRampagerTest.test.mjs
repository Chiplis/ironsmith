import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/abilities/enters/GreenbeltRampagerTest.java",
  "tests": [
    {
      "name": "testFirstCast",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Greenbelt Rampager",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Greenbelt Rampager"
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
          "op": "assertHandCount",
          "player": 0,
          "name": "Greenbelt Rampager",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Greenbelt Rampager",
          "count": 0
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": 0,
          "counter": "ENERGY",
          "count": 1
        }
      ]
    },
    {
      "name": "testScondCast",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Greenbelt Rampager",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 2
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Greenbelt Rampager"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Greenbelt Rampager"
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
          "op": "assertHandCount",
          "player": 0,
          "name": "Greenbelt Rampager",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Greenbelt Rampager",
          "count": 0
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": 0,
          "counter": "ENERGY",
          "count": 2
        }
      ]
    },
    {
      "name": "testThirdCast",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Greenbelt Rampager",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 3
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Greenbelt Rampager"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Greenbelt Rampager"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Greenbelt Rampager"
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
          "op": "assertHandCount",
          "player": 0,
          "name": "Greenbelt Rampager",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Greenbelt Rampager",
          "count": 1
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": 0,
          "counter": "ENERGY",
          "count": 0
        }
      ]
    },
    {
      "name": "testCastNotOwned",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 10
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Gonti, Lord of Luxury",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 1,
          "name": "Greenbelt Rampager",
          "count": 1
        },
        {
          "op": "skipInitShuffling"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Gonti, Lord of Luxury"
        },
        {
          "op": "setStrictChooseMode",
          "value": false
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Greenbelt Rampager"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Greenbelt Rampager"
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
          "op": "assertCounterCount",
          "player": 0,
          "name": 0,
          "counter": "ENERGY",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Greenbelt Rampager",
          "count": 0
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "Greenbelt Rampager",
          "count": 0
        },
        {
          "op": "assertHandCount",
          "player": 1,
          "name": "Greenbelt Rampager",
          "count": 1
        }
      ]
    }
  ]
});
