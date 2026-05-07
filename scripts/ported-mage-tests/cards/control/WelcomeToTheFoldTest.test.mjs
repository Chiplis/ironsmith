import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/control/WelcomeToTheFoldTest.java",
  "tests": [
    {
      "name": "regularCostVariousCreatures",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Welcome to the Fold",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 12
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Arashin Cleric",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Jace, Vryn's Prodigy",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Elite Vanguard",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Welcome to the Fold"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Arashin Cleric"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Welcome to the Fold"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Jace, Vryn's Prodigy"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Welcome to the Fold"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Elite Vanguard"
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
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Welcome to the Fold",
          "count": 3
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Arashin Cleric",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Jace, Vryn's Prodigy",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Elite Vanguard",
          "count": 1
        }
      ]
    }
  ]
});
