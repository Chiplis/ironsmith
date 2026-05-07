import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/nph/TriumphOfTheHordesTest.java",
  "tests": [
    {
      "name": "triumphOfTheHordesNonCreatureTest",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 10
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Chromatic Lantern",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Triumph of the Hordes",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Grizzly Bears",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Triumph of the Hordes"
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
          "op": "assertAbility",
          "player": 0,
          "name": "Chromatic Lantern",
          "ability": "Infect",
          "expected": false
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Grizzly Bears",
          "ability": "Infect",
          "expected": true
        }
      ]
    }
  ]
});
