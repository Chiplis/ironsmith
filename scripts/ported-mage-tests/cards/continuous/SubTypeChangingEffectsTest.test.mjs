import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/continuous/SubTypeChangingEffectsTest.java",
  "tests": [
    {
      "name": "testConspiracyGiveType",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Conspiracy",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 5
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Silvercoat Lion",
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
          "name": "Silvercoat Lion",
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
          "name": "Conspiracy"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Orc"
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
          "name": "Conspiracy",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "for (Card card : playerA.getLibrary().getCards(currentGame)) { if (card.isCreature(currentGame)) { Assert.assertTrue(card.getName() + \" should have ORC type\", card.hasSubtype(SubType.ORC, currentGame)); Assert.assertFalse(card.getName() + \" should not have CAT type\", card.hasSubtype(SubType.CAT, currentGame)); } } for (Card card : playerB.getLibrary().getCards(currentGame)) { if (card.isCreature(currentGame)) { Assert.assertFalse(card.getName() + \" should not have ORC type\", card.hasSubtype(SubType.ORC, currentGame)); Assert.assertTrue(card.getName() + \" should have CAT type\", card.hasSubtype(SubType.CAT, currentGame)); } } for (Card card : playerA.getHand().getCards(currentGame)) { if (card.isCreature(currentGame)) { Assert.assertTrue(card.getName() + \" should have ORC type\", card.hasSubtype(SubType.ORC, currentGame)); } } for (Card card : playerB.getHand().getCards(currentGame)) { if (card.isCreature(currentGame)) { Assert.assertFalse(card.getName() + \" should not have ORC type\", card.hasSubtype(SubType.ORC, currentGame)); } } for (Card card : playerA.getGraveyard().getCards(currentGame)) { if (card.isCreature(currentGame)) { Assert.assertTrue(card.getName() + \" should have ORC type\", card.hasSubtype(SubType.ORC, currentGame)); } } for (Card card : playerB.getGraveyard().getCards(currentGame)) { if (card.isCreature(currentGame)) { Assert.assertFalse(card.getName() + \" should not have ORC type\", card.hasSubtype(SubType.ORC, currentGame)); } }"
        }
      ]
    },
    {
      "name": "testConspiracyIsRestCorrectly",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Conspiracy",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 5
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Silvercoat Lion",
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
          "name": "Disenchant",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Plains",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Silvercoat Lion",
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
          "name": "Conspiracy"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Orc"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 1,
          "name": "Disenchant",
          "target": "Conspiracy"
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
          "name": "Conspiracy",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Disenchant",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "for (Card card : playerA.getLibrary().getCards(currentGame)) { if (card.isCreature(currentGame)) { Assert.assertFalse(card.getName() + \" should not have ORC type\", card.hasSubtype(SubType.ORC, currentGame)); Assert.assertTrue(card.getName() + \" should have CAT type\", card.hasSubtype(SubType.CAT, currentGame)); } } for (Card card : playerA.getHand().getCards(currentGame)) { if (card.isCreature(currentGame)) { Assert.assertFalse(card.getName() + \" should not have ORC type\", card.hasSubtype(SubType.ORC, currentGame)); Assert.assertTrue(card.getName() + \" should have CAT type\", card.hasSubtype(SubType.CAT, currentGame)); } } for (Card card : playerA.getGraveyard().getCards(currentGame)) { if (card.isCreature(currentGame)) { Assert.assertFalse(card.getName() + \" should not have ORC type\", card.hasSubtype(SubType.ORC, currentGame)); Assert.assertTrue(card.getName() + \" should have CAT type\", card.hasSubtype(SubType.CAT, currentGame)); } }"
        }
      ]
    },
    {
      "name": "testArcaneAdaptationGiveType",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Arcane Adaptation",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Silvercoat Lion",
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
          "name": "Silvercoat Lion",
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
          "name": "Arcane Adaptation"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Orc"
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
          "name": "Arcane Adaptation",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "for (Card card : playerA.getLibrary().getCards(currentGame)) { if (card.isCreature(currentGame)) { Assert.assertEquals(card.getName() + \" should have ORC type\", true, card.hasSubtype(SubType.ORC, currentGame)); Assert.assertEquals(card.getName() + \" should have CAT type\", true, card.hasSubtype(SubType.CAT, currentGame)); } } for (Card card : playerB.getLibrary().getCards(currentGame)) { if (card.isCreature(currentGame)) { Assert.assertEquals(card.getName() + \" should not have ORC type\", false, card.hasSubtype(SubType.ORC, currentGame)); Assert.assertEquals(card.getName() + \" should have CAT type\", true, card.hasSubtype(SubType.CAT, currentGame)); } } for (Card card : playerA.getHand().getCards(currentGame)) { if (card.isCreature(currentGame)) { Assert.assertEquals(card.getName() + \" should have ORC type\", true, card.hasSubtype(SubType.ORC, currentGame)); Assert.assertEquals(card.getName() + \" should have CAT type\", true, card.hasSubtype(SubType.CAT, currentGame)); } } for (Card card : playerB.getHand().getCards(currentGame)) { if (card.isCreature(currentGame)) { Assert.assertEquals(card.getName() + \" should not have ORC type\", false, card.hasSubtype(SubType.ORC, currentGame)); Assert.assertEquals(card.getName() + \" should have CAT type\", true, card.hasSubtype(SubType.CAT, currentGame)); } } for (Card card : playerA.getGraveyard().getCards(currentGame)) { if (card.isCreature(currentGame)) { Assert.assertEquals(card.getName() + \" should have ORC type\", true, card.hasSubtype(SubType.ORC, currentGame)); Assert.assertEquals(card.getName() + \" should have CAT type\", true, card.hasSubtype(SubType.CAT, currentGame)); } } for (Card card : playerB.getGraveyard().getCards(currentGame)) { if (card.isCreature(currentGame)) { Assert.assertEquals(card.getName() + \" should not have ORC type\", false, card.hasSubtype(SubType.ORC, currentGame)); Assert.assertEquals(card.getName() + \" should have CAT type\", true, card.hasSubtype(SubType.CAT, currentGame)); } }"
        }
      ]
    },
    {
      "name": "testArcaneAdaptationIsRestCorrectly",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Arcane Adaptation",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 8
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Advent of the Wurm",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Beast Attack",
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
          "name": "Disenchant",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Plains",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Silvercoat Lion",
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
          "phase": "UPKEEP",
          "player": 0,
          "name": "Advent of the Wurm"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Arcane Adaptation"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Orc"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "BEGIN_COMBAT",
          "player": 0,
          "name": "Beast Attack"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 1,
          "name": "Disenchant",
          "target": "Arcane Adaptation"
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
          "name": "Arcane Adaptation",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Advent of the Wurm",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Beast Attack",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Disenchant",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "for (Card card : playerA.getLibrary().getCards(currentGame)) { if (card.isCreature(currentGame)) { Assert.assertEquals(card.getName() + \" should not have ORC type\", false, card.hasSubtype(SubType.ORC, currentGame)); Assert.assertEquals(card.getName() + \" should have CAT type\", true, card.hasSubtype(SubType.CAT, currentGame)); } } for (Card card : playerA.getHand().getCards(currentGame)) { if (card.isCreature(currentGame)) { Assert.assertEquals(card.getName() + \" should not have ORC type\", false, card.hasSubtype(SubType.ORC, currentGame)); Assert.assertEquals(card.getName() + \" should have CAT type\", true, card.hasSubtype(SubType.CAT, currentGame)); } } for (Card card : playerA.getGraveyard().getCards(currentGame)) { if (card.isCreature(currentGame)) { Assert.assertEquals(card.getName() + \" should not have ORC type\", false, card.hasSubtype(SubType.ORC, currentGame)); Assert.assertEquals(card.getName() + \" should have CAT type\", true, card.hasSubtype(SubType.CAT, currentGame)); } }"
        }
      ]
    },
    {
      "name": "testKeepOtherTypes",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Volcanic Island",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Gingerbrute",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Dragonshift",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Dragonshift",
          "target": "Gingerbrute"
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
          "op": "unsupported",
          "source": "assertType(\"Gingerbrute\", CardType.ARTIFACT, SubType.FOOD)"
        },
        {
          "op": "unsupported",
          "source": "assertType(\"Gingerbrute\", CardType.CREATURE, SubType.DRAGON)"
        },
        {
          "op": "unsupported",
          "source": "assertNotSubtype(\"Gingerbrute\", SubType.GOLEM)"
        }
      ]
    },
    {
      "name": "testKeepOtherTypes2",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Volcanic Island",
          "count": 7
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Gingerbrute",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Dragonshift",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Dragonshift with overload"
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
          "op": "unsupported",
          "source": "assertType(\"Gingerbrute\", CardType.ARTIFACT, SubType.FOOD)"
        },
        {
          "op": "unsupported",
          "source": "assertType(\"Gingerbrute\", CardType.CREATURE, SubType.DRAGON)"
        },
        {
          "op": "unsupported",
          "source": "assertNotSubtype(\"Gingerbrute\", SubType.GOLEM)"
        }
      ]
    },
    {
      "name": "testMaskwoodNexus",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Maskwood Nexus",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Silvercoat Lion",
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
          "name": "Silvercoat Lion",
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
          "name": "Maskwood Nexus"
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
          "name": "Maskwood Nexus",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "for (Card card : playerA.getLibrary().getCards(currentGame)) { if (card.isCreature(currentGame)) { Assert.assertTrue(card.getName() + \" should have ORC type\", card.hasSubtype(SubType.ORC, currentGame)); Assert.assertTrue(card.getName() + \" should have CAT type\", card.hasSubtype(SubType.CAT, currentGame)); } } for (Card card : playerB.getLibrary().getCards(currentGame)) { if (card.isCreature(currentGame)) { Assert.assertFalse(card.getName() + \" should not have ORC type\", card.hasSubtype(SubType.ORC, currentGame)); Assert.assertTrue(card.getName() + \" should have CAT type\", card.hasSubtype(SubType.CAT, currentGame)); } } for (Card card : playerA.getHand().getCards(currentGame)) { if (card.isCreature(currentGame)) { Assert.assertTrue(card.getName() + \" should have ORC type\", card.hasSubtype(SubType.ORC, currentGame)); } } for (Card card : playerB.getHand().getCards(currentGame)) { if (card.isCreature(currentGame)) { Assert.assertFalse(card.getName() + \" should not have ORC type\", card.hasSubtype(SubType.ORC, currentGame)); } } for (Card card : playerA.getGraveyard().getCards(currentGame)) { if (card.isCreature(currentGame)) { Assert.assertTrue(card.getName() + \" should have ORC type\", card.hasSubtype(SubType.ORC, currentGame)); } } for (Card card : playerB.getGraveyard().getCards(currentGame)) { if (card.isCreature(currentGame)) { Assert.assertFalse(card.getName() + \" should not have ORC type\", card.hasSubtype(SubType.ORC, currentGame)); } }"
        }
      ]
    },
    {
      "name": "testMaskwoodNexus2",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Maskwood Nexus",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Shatter",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 6
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Silvercoat Lion",
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
          "name": "Silvercoat Lion",
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
          "name": "Maskwood Nexus"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "BEGIN_COMBAT",
          "player": 0,
          "name": "Shatter",
          "target": "Maskwood Nexus"
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
          "name": "Maskwood Nexus",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "for (Card card : playerA.getLibrary().getCards(currentGame)) { if (card.isCreature(currentGame)) { Assert.assertFalse(card.getName() + \" should not have ORC type\", card.hasSubtype(SubType.ORC, currentGame)); Assert.assertTrue(card.getName() + \" should have CAT type\", card.hasSubtype(SubType.CAT, currentGame)); } } for (Card card : playerB.getLibrary().getCards(currentGame)) { if (card.isCreature(currentGame)) { Assert.assertFalse(card.getName() + \" should not have ORC type\", card.hasSubtype(SubType.ORC, currentGame)); Assert.assertTrue(card.getName() + \" should have CAT type\", card.hasSubtype(SubType.CAT, currentGame)); } } for (Card card : playerA.getHand().getCards(currentGame)) { if (card.isCreature(currentGame)) { Assert.assertFalse(card.getName() + \" should not have ORC type\", card.hasSubtype(SubType.ORC, currentGame)); } } for (Card card : playerB.getHand().getCards(currentGame)) { if (card.isCreature(currentGame)) { Assert.assertFalse(card.getName() + \" should not have ORC type\", card.hasSubtype(SubType.ORC, currentGame)); } } for (Card card : playerA.getGraveyard().getCards(currentGame)) { if (card.isCreature(currentGame)) { Assert.assertFalse(card.getName() + \" should not have ORC type\", card.hasSubtype(SubType.ORC, currentGame)); } } for (Card card : playerB.getGraveyard().getCards(currentGame)) { if (card.isCreature(currentGame)) { Assert.assertFalse(card.getName() + \" should not have ORC type\", card.hasSubtype(SubType.ORC, currentGame)); } }"
        }
      ]
    },
    {
      "name": "testMaskwoodNexus3",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Sarkhan the Masterless",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Silvercoat Lion",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Maskwood Nexus",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Bonebreaker Giant",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Bonebreaker Giant",
          "defender": 1
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
          "op": "unsupported",
          "source": "assertDamageReceived(playerA, \"Bonebreaker Giant\", 3)"
        }
      ]
    }
  ]
});
