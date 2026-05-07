import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/bfz/UginsInsightTest.java",
  "tests": [
    {
      "name": "test_greatest_0",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Ugin's Insight",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 5
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Barbarian Horde",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Ugin's Insight"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
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
          "count": 0,
          "name": 3
        }
      ]
    },
    {
      "name": "test_greatest_4",
      "operations": [
        {
          "op": "skipInitShuffling"
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Ageless Guardian",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Barktooth Warbeard",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Catacomb Crocodile",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Devilthorn Fox",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Ugin's Insight",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 5
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Acolyte of Bahamut",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Barbarian Horde",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Sword of Fire and Ice",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Barbarian Horde",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Ugin's Insight"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Devilthorn Fox^Barktooth Warbeard"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Barktooth Warbeard"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Catacomb Crocodile"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
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
          "count": 0,
          "name": 3
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "Mountain",
          "count": 1
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "Catacomb Crocodile",
          "count": 1
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "Ageless Guardian",
          "count": 1
        }
      ]
    }
  ]
});
