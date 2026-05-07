import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/xln/TreasureCoveTest.java",
  "tests": [
    {
      "name": "test_CanDrawAfterSacrifice",
      "operations": [
        {
          "op": "clearZone",
          "player": 0,
          "zone": "hand"
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Treasure Cove",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forging the Tyrite Sword",
          "count": 1
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "assertHandCount",
          "player": 1,
          "name": "PRECOMBAT_MAIN",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Treasure Token",
          "count": 2
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{T}, Sacrifice"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Treasure Token"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "assertHandCount",
          "player": 1,
          "name": "PRECOMBAT_MAIN",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Treasure Token",
          "count": 1
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
        }
      ]
    }
  ]
});
