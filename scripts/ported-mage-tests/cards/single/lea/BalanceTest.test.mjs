import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/lea/BalanceTest.java",
  "tests": [
    {
      "name": "testBalance",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Runeclaw Bear",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Plains",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Balance",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Swamp",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Runeclaw Bear",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Swamp",
          "count": 2
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Balance"
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
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Plains",
          "count": 2
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Runeclaw Bear",
          "count": 2
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "Plains",
          "count": 1
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "Balance",
          "count": 0
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Runeclaw Bear",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Balance",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Swamp",
          "count": 2
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Runeclaw Bear",
          "count": 2
        },
        {
          "op": "assertHandCount",
          "player": 1,
          "name": "Swamp",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Swamp",
          "count": 2
        }
      ]
    }
  ]
});
