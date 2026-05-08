import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/triggers/DivineVisitationTest.java",
  "tests": [
    {
      "name": "testDivineVisitationDoesNotReplaceNoncreatureTokens",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Divine Visitation",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Smothering Tithe",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Ancestral Recall",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Ancestral Recall",
          "target": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Whenever an opponent draws a card"
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": false
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
          "player": 1,
          "count": 3
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Treasure Token",
          "count": 3
        },
        {
          "op": "unsupported",
          "source": "assertType(\"Treasure Token\", CardType.ARTIFACT, SubType.TREASURE)"
        },
        {
          "op": "unsupported",
          "source": "assertNotType(\"Treasure Token\", CardType.CREATURE)"
        },
        {
          "op": "unsupported",
          "source": "assertNotSubtype(\"Treasure Token\", SubType.ANGEL)"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Angel Token",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "count": 6
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "count": 1
        }
      ]
    },
    {
      "name": "testDivineVisitationReplacesCreatureTokens",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Divine Visitation",
          "count": 1
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
          "zone": "HAND",
          "player": 0,
          "name": "Dragon Fodder",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Dragon Fodder"
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
          "op": "assertGraveyardCount",
          "player": 0,
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "count": 5
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Goblin Token",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Angel Token",
          "count": 2
        },
        {
          "op": "unsupported",
          "source": "assertType(\"Angel Token\", CardType.CREATURE, SubType.ANGEL)"
        },
        {
          "op": "unsupported",
          "source": "assertColor(playerA, \"Angel Token\", ObjectColor.WHITE, true)"
        },
        {
          "op": "unsupported",
          "source": "assertColor(playerA, \"Angel Token\", ObjectColor.RED, false)"
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Angel Token",
          "power": 4,
          "toughness": 4
        },
        {
          "op": "unsupported",
          "source": "assertNotSubtype(\"Angel Token\", SubType.GOBLIN)"
        }
      ]
    },
    {
      "name": "testSacrificeEOT",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Divine Visitation",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Thatcher Revolt",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Thatcher Revolt"
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
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Human Token",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Angel Token",
          "count": 0
        }
      ]
    }
  ]
});
