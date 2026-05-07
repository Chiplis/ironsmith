import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/bfz/ConduitOfRuinTest.java",
  "tests": [
    {
      "name": "testCast",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Emrakul, the Aeons Torn",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Conduit of Ruin",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 13
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Conduit of Ruin"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Emrakul, the Aeons Torn"
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
          "op": "assertLibraryCount",
          "player": 0,
          "name": "Emrakul, the Aeons Torn",
          "count": 0
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "Emrakul, the Aeons Torn",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Emrakul, the Aeons Torn"
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
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Conduit of Ruin",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Emrakul, the Aeons Torn",
          "count": 1
        }
      ]
    }
  ]
});
