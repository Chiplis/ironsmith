import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/continuous/PaintersServantTest.java",
  "tests": [
    {
      "name": "testColorSet",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Painter's Servant",
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
          "player": 0,
          "name": "Lightning Bolt",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Silvercoat Lion",
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
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Lightning Bolt",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 1,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Painter's Servant"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Blue"
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
          "name": "Painter's Servant",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "for (Card card : playerA.getLibrary().getCards(currentGame)) { Assert.assertEquals(card.getName() + \" should be blue\", true, card.getColor(currentGame).isBlue()); } for (Card card : playerB.getLibrary().getCards(currentGame)) { Assert.assertEquals(card.getName() + \" should be blue\", true, card.getColor(currentGame).isBlue()); } for (Card card : playerA.getHand().getCards(currentGame)) { Assert.assertEquals(true, card.getColor(currentGame).isRed()); Assert.assertEquals(true, card.getColor(currentGame).isBlue()); } for (Card card : playerB.getHand().getCards(currentGame)) { Assert.assertEquals(true, card.getColor(currentGame).isRed()); Assert.assertEquals(true, card.getColor(currentGame).isBlue()); } for (Card card : playerA.getGraveyard().getCards(currentGame)) { Assert.assertEquals(true, card.getColor(currentGame).isWhite()); Assert.assertEquals(true, card.getColor(currentGame).isBlue()); } for (Card card : playerB.getGraveyard().getCards(currentGame)) { Assert.assertEquals(true, card.getColor(currentGame).isWhite()); Assert.assertEquals(true, card.getColor(currentGame).isBlue()); }"
        }
      ]
    },
    {
      "name": "testColorReset",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Painter's Servant",
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
          "player": 0,
          "name": "Lightning Bolt",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Silvercoat Lion",
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
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Lightning Bolt",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 1,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Mountain",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Painter's Servant"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Blue"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 1,
          "name": "Lightning Bolt",
          "target": "Painter's Servant"
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
          "name": "Painter's Servant",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "for (Card card : playerA.getLibrary().getCards(currentGame)) { Assert.assertEquals(card.getName() + \" should not be blue\", false, card.getColor(currentGame).isBlue()); } for (Card card : playerB.getLibrary().getCards(currentGame)) { Assert.assertEquals(card.getName() + \" should not be blue\", false, card.getColor(currentGame).isBlue()); } for (Card card : playerA.getHand().getCards(currentGame)) { Assert.assertEquals(true, card.getColor(currentGame).isRed()); Assert.assertEquals(false, card.getColor(currentGame).isBlue()); } for (Card card : playerB.getHand().getCards(currentGame)) { Assert.assertEquals(true, card.getColor(currentGame).isRed()); Assert.assertEquals(false, card.getColor(currentGame).isBlue()); } for (Card card : playerA.getGraveyard().getCards(currentGame)) { if (card.getName().equals(\"Silvercoat Lion\")) { Assert.assertEquals(true, card.getColor(currentGame).isWhite()); Assert.assertEquals(false, card.getColor(currentGame).isBlue()); } } for (Card card : playerB.getGraveyard().getCards(currentGame)) { if (card.getName().equals(\"Silvercoat Lion\")) { Assert.assertEquals(true, card.getColor(currentGame).isWhite()); Assert.assertEquals(false, card.getColor(currentGame).isBlue()); } }"
        }
      ]
    },
    {
      "name": "testColorOverwrite",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Painter's Servant",
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
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Cerulean Wisps",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Island",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Painter's Servant"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Red"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Cerulean Wisps",
          "target": "Silvercoat Lion"
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
          "name": "Painter's Servant",
          "count": 1
        }
      ]
    },
    {
      "name": "testColorSpell",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Painter's Servant",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Divination",
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
          "name": "Dragon's Claw",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Cerulean Wisps",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Island",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Painter's Servant"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Red"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Cerulean Wisps",
          "target": "Silvercoat Lion"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Divination",
          "target": "TestPlayer.NO_TARGET"
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
          "name": "Painter's Servant",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Divination",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Cerulean Wisps",
          "count": 1
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 22
        }
      ]
    },
    {
      "name": "testColorSpellEnds",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Painter's Servant",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 5
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Pyroblast",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Green Sun's Zenith",
          "count": 1
        },
        {
          "op": "addTarget",
          "player": 1,
          "target": "Ambush Viper"
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Altar's Light",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Forest",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Plains",
          "count": 6
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 1,
          "name": "Ambush Viper",
          "count": 2
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Painter's Servant"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Blue"
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Green Sun's Zenith"
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": "X=2"
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Pyroblast",
          "target": "Green Sun's Zenith"
        },
        {
          "op": "setModeChoice",
          "player": 0,
          "value": "1"
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Altar's Light",
          "target": "Painter's Servant"
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Pyroblast",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Altar's Light",
          "count": 1
        },
        {
          "op": "assertLibraryCount",
          "player": 1,
          "name": "Green Sun's Zenith",
          "count": 1
        },
        {
          "op": "assertExileCount",
          "player": 0,
          "name": "Painter's Servant",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Ambush Viper",
          "count": 1
        }
      ]
    }
  ]
});
