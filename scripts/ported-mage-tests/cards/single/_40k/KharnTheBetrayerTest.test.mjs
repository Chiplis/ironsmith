import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/_40k/KharnTheBetrayerTest.java",
  "tests": [
    {
      "name": "testEffect",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Khârn the Betrayer@kharn",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Mountain",
          "count": 5
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 1,
          "name": "Mountain",
          "count": 5
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": "playerC",
          "name": "Mountain",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": "playerC",
          "name": "Lightning Bolt",
          "count": 2
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 2,
          "name": "Lightning Bolt",
          "target": "@kharn",
          "holdStack": true
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Bob"
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Lightning Bolt",
          "isSpell": true,
          "count": 1,
          "objectTarget": "Khârn the Betrayer"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "@kharn",
          "count": 1
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "count": 3
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 2,
          "name": "Lightning Bolt",
          "target": "@kharn",
          "holdStack": true
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": "Charlie"
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Lightning Bolt",
          "isSpell": true,
          "count": 1,
          "objectTarget": "Khârn the Betrayer"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertPermanentCount",
          "player": 2,
          "name": "@kharn",
          "count": 1
        },
        {
          "op": "assertHandCount",
          "player": 1,
          "count": 2
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "execute"
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "count": 3
        },
        {
          "op": "assertHandCount",
          "player": 1,
          "count": 2
        },
        {
          "op": "assertHandCount",
          "player": "playerC",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "name": "@kharn",
          "count": 1,
          "player": 2
        }
      ],
      "turnOrder": [
        0,
        3,
        2,
        1
      ]
    },
    {
      "name": "testLostControl",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Khârn the Betrayer@kharn",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Harmless Offering",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Mountain",
          "count": 5
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Harmless Offering",
          "holdStack": true,
          "target": {
            "groups": [
              [
                1
              ],
              [
                "@kharn"
              ]
            ]
          }
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Harmless Offering",
          "isSpell": true,
          "count": 1,
          "objectTarget": "Khârn the Betrayer"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "@kharn",
          "count": 1
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
          "op": "assertPermanentCount",
          "player": 1,
          "name": "@kharn",
          "count": 1
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "count": 3
        }
      ],
      "turnOrder": [
        0,
        3,
        2,
        1
      ]
    }
  ]
});
