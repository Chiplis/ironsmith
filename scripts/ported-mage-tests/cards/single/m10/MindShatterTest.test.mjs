import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/m10/MindShatterTest.java",
  "tests": [
    {
      "name": "testMindShatterXZeroOneCardInOpponentHand",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Mind Shatter",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Wooded Foothills",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Mind Shatter"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "X=0"
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
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Mind Shatter",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Wooded Foothills",
          "count": 0
        },
        {
          "op": "assertHandCount",
          "player": 1,
          "name": "Wooded Foothills",
          "count": 1
        }
      ]
    },
    {
      "name": "testMindShatterXZeroTwoCardsInOpponentHand",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Mind Shatter",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Wooded Foothills",
          "count": 2
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Mind Shatter"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "X=0"
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
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Mind Shatter",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Wooded Foothills",
          "count": 0
        },
        {
          "op": "assertHandCount",
          "player": 1,
          "name": "Wooded Foothills",
          "count": 2
        }
      ]
    }
  ]
});
