import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/asthough/LeylineOfAnticipationTest.java",
  "tests": [
    {
      "name": "testCastAsThoughHasFlashDuringCombat",
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
          "zone": "HAND",
          "player": 0,
          "name": "Leyline of Anticipation",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "useOpeningHandAction",
          "player": 0,
          "name": "Leyline of Anticipation"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Leyline of Anticipation",
          "count": 1
        },
        {
          "op": "assertPlayableAbility",
          "player": 0,
          "turn": 2,
          "phase": "DRAW",
          "expected": true,
          "label": "Cast Silvercoat Lion"
        },
        {
          "op": "castSpell",
          "name": "Silvercoat Lion",
          "player": 0,
          "turn": 2,
          "phase": "DRAW"
        },
        {
          "op": "assertStackObject",
          "source": "Silvercoat Lion",
          "count": 1,
          "turn": 2,
          "phase": "DRAW",
          "isSpell": true
        },
        {
          "op": "waitStackResolved",
          "turn": 2,
          "phase": "DRAW"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 0
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
        }
      ]
    },
    {
      "name": "testNoCastPossibleOnOpponentsTurn",
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
          "name": "Yawgmoth's Agenda",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Leyline of Anticipation",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "useOpeningHandAction",
          "player": 0,
          "name": "Leyline of Anticipation"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Leyline of Anticipation",
          "count": 1
        },
        {
          "op": "assertPlayableAbility",
          "player": 0,
          "turn": 2,
          "phase": "DRAW",
          "expected": true,
          "label": "Cast Silvercoat Lion"
        },
        {
          "op": "castSpell",
          "name": "Silvercoat Lion",
          "player": 0,
          "turn": 2,
          "phase": "DRAW"
        },
        {
          "op": "assertStackObject",
          "source": "Silvercoat Lion",
          "count": 1,
          "turn": 2,
          "phase": "DRAW",
          "isSpell": true
        },
        {
          "op": "waitStackResolved",
          "turn": 2,
          "phase": "DRAW"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 0
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
        }
      ]
    }
  ]
});
