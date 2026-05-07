import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/eld/RobberOfTheRichTest.java",
  "tests": [
    {
      "name": "testRobber",
      "operations": [
        {
          "op": "clearZone",
          "player": 1,
          "zone": "library"
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Robber of the Rich",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 1,
          "name": "Whitesun's Passage",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Mountain",
          "count": 3
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Robber of the Rich",
          "defender": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Whitesun's Passage"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
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
          "op": "assertLife",
          "player": 0,
          "life": 25
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Whitesun's Passage",
          "count": 1
        }
      ]
    },
    {
      "name": "testRobberWithOtherRogue",
      "operations": [
        {
          "op": "clearZone",
          "player": 1,
          "zone": "library"
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Robber of the Rich",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Bane Alley Blackguard",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 1,
          "name": "Whitesun's Passage",
          "count": 5
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Mountain",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Robber of the Rich",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Bane Alley Blackguard",
          "defender": 1
        },
        {
          "op": "castSpell",
          "turn": 3,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Whitesun's Passage"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 25
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Whitesun's Passage",
          "count": 1
        }
      ]
    }
  ]
});
