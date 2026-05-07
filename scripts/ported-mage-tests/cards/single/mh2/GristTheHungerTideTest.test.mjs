import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/mh2/GristTheHungerTideTest.java",
  "tests": [
    {
      "name": "testGristInHandBattlefieldGraveLibrary",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Grist, the Hunger Tide",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Grist, the Hunger Tide",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Grist, the Hunger Tide",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Grist, the Hunger Tide",
          "count": 1
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
          "source": "for (Card card : currentGame.getCards()) { if (!card.getName().equals(grist)) { continue; } Zone zone = currentGame.getState().getZone(card.getId()); if (zone == Zone.BATTLEFIELD) { Assert.assertFalse(\"Not a creature on the battlefield\", card.isCreature(currentGame)); } else { Assert.assertTrue(\"Should be a creature when zone is \" + zone, card.isCreature(currentGame)); } }"
        }
      ]
    },
    {
      "name": "testGristInExile",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Grist, the Hunger Tide",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Putrid Imp",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Leyline of the Void",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "Discard"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Grist, the Hunger Tide"
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
          "source": "for (Card card : currentGame.getCards()) { if (!card.getName().equals(grist)) { continue; } Assert.assertEquals(\"\", Zone.EXILED, currentGame.getState().getZone(card.getId())); Assert.assertTrue(\"Should be a creature in exile\", card.isCreature(currentGame)); }"
        }
      ]
    },
    {
      "name": "testGristFromStackToBattlefield",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Bayou",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Primeval Bounty",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Grist, the Hunger Tide",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Grist, the Hunger Tide"
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
          "name": "Beast Token",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Grist, the Hunger Tide",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "assertType(grist, CardType.CREATURE, false)"
        }
      ]
    }
  ]
});
