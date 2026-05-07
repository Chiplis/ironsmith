import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/triggers/delayed/ReturnToHandTest.java",
  "tests": [
    {
      "name": "testExilesCreatureAtEndStep",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Honor Guard",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Undiscovered Paradise",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Honor Guard"
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "DRAW"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Honor Guard",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Undiscovered Paradise",
          "count": 0
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "Undiscovered Paradise",
          "count": 1
        }
      ]
    }
  ]
});
