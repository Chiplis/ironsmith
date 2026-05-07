import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/abilities/enters/DearlyDepartedTest.java",
  "tests": [
    {
      "name": "testEnteringWithCounters",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 5
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Dearly Departed",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Thraben Doomsayer",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Thraben Doomsayer"
        },
        {
          "op": "activateAbility",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{T}: Create a 1/1 white Human creature token."
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 20
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 20
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Thraben Doomsayer",
          "power": 3,
          "toughness": 3
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Human Token",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Human Token",
          "power": 2,
          "toughness": 2
        }
      ]
    }
  ]
});
