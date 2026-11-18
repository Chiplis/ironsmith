import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/jou/DeicideTest.java",
  "tests": [
    {
      "name": "targetNonCreatureeGod",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Deicide",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Heliod, Sun-Crowned",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Crusade",
          "count": 3,
          "fixtureSource": true,
          "typeLine": "Enchantment",
          "oracleText": "White creatures get +1/+1.",
          "manaCost": "{W}{W}"
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 2
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 1,
            "name": "Heliod, Sun-Crowned"
          },
          "checks": [
            {
              "property": "cardType",
              "value": "creature",
              "expected": true
            }
          ],
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Deicide",
          "target": "Heliod, Sun-Crowned"
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
          "op": "assertEachObject",
          "selector": {
            "zone": "exile",
            "player": 1,
            "name": "Heliod, Sun-Crowned"
          },
          "checks": [
            {
              "property": "subtype",
              "value": "God",
              "expected": true
            }
          ],
          "count": 1
        }
      ]
    }
  ]
});
