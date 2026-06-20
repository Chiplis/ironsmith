import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/copy/TokenCopyTest.java",
  "tests": [
    {
      "name": "testCopyDFC",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Kessig Prowler",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Rite of Replication",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Rite of Replication",
          "target": "Kessig Prowler"
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
          "name": "Rite of Replication",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Kessig Prowler",
          "count": 2
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Sinuous Predator",
          "count": 0
        },
        {
          "op": "unsupported",
          "source": "for (Permanent permanent : currentGame.getBattlefield().getAllActivePermanents()) { switch (permanent.getName()) { case prowler: Assert.assertEquals(\"Power of \" + prowler + \" should be 2\", 2, permanent.getPower().getValue()); Assert.assertEquals(\"Toughness of \" + prowler + \" should be 1\", 1, permanent.getToughness().getValue()); Assert.assertEquals(prowler + \" should be green\", ObjectColor.GREEN, permanent.getColor(currentGame)); Assert.assertTrue(prowler + \" should be a Werewolf\", permanent.hasSubtype(SubType.WEREWOLF, currentGame)); Assert.assertTrue(prowler + \" should be a Horror\", permanent.hasSubtype(SubType.HORROR, currentGame)); Assert.assertFalse(prowler + \" should not be an Eldrazi\", permanent.hasSubtype(SubType.ELDRAZI, currentGame)); Assert.assertEquals(prowler + \" should have mana value 1\", 1, permanent.getManaValue()); Assert.assertFalse(prowler + \" should not be transformed\", permanent.isTransformed()); break; case predator: Assert.assertEquals(\"Power of \" + predator + \" should be 4\", 4, permanent.getPower().getValue()); Assert.assertEquals(\"Toughness of \" + predator + \" should be 4\", 4, permanent.getToughness().getValue()); Assert.assertTrue(predator + \" should be colorless\", permanent.getColor(currentGame).isColorless()); Assert.assertTrue(predator + \" should be an Eldrazi\", permanent.hasSubtype(SubType.ELDRAZI, currentGame)); Assert.assertTrue(predator + \" should be a Werewolf\", permanent.hasSubtype(SubType.WEREWOLF, currentGame)); Assert.assertFalse(predator + \" should not be a Horror\", permanent.hasSubtype(SubType.HORROR, currentGame)); Assert.assertEquals(predator + \" should have mana value 1\", 1, permanent.getManaValue()); Assert.assertTrue(prowler + \" should be transformed\", permanent.isTransformed()); break; } }"
        }
      ]
    },
    {
      "name": "testCopyDFCAndTransform",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Tropical Island",
          "count": 14
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Kessig Prowler",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Rite of Replication",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Rite of Replication",
          "target": "Kessig Prowler"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{4}{G}"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{4}{G}"
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
          "name": "Rite of Replication",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Kessig Prowler",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Sinuous Predator",
          "count": 2
        },
        {
          "op": "unsupported",
          "source": "for (Permanent permanent : currentGame.getBattlefield().getAllActivePermanents()) { switch (permanent.getName()) { case prowler: Assert.assertEquals(\"Power of \" + prowler + \" should be 2\", 2, permanent.getPower().getValue()); Assert.assertEquals(\"Toughness of \" + prowler + \" should be 1\", 1, permanent.getToughness().getValue()); Assert.assertEquals(prowler + \" should be green\", ObjectColor.GREEN, permanent.getColor(currentGame)); Assert.assertTrue(prowler + \" should be a Werewolf\", permanent.hasSubtype(SubType.WEREWOLF, currentGame)); Assert.assertTrue(prowler + \" should be a Horror\", permanent.hasSubtype(SubType.HORROR, currentGame)); Assert.assertFalse(prowler + \" should not be an Eldrazi\", permanent.hasSubtype(SubType.ELDRAZI, currentGame)); Assert.assertEquals(prowler + \" should have mana value 1\", 1, permanent.getManaValue()); Assert.assertFalse(prowler + \" should not be transformed\", permanent.isTransformed()); break; case predator: Assert.assertEquals(\"Power of \" + predator + \" should be 4\", 4, permanent.getPower().getValue()); Assert.assertEquals(\"Toughness of \" + predator + \" should be 4\", 4, permanent.getToughness().getValue()); Assert.assertTrue(predator + \" should be colorless\", permanent.getColor(currentGame).isColorless()); Assert.assertTrue(predator + \" should be an Eldrazi\", permanent.hasSubtype(SubType.ELDRAZI, currentGame)); Assert.assertTrue(predator + \" should be a Werewolf\", permanent.hasSubtype(SubType.WEREWOLF, currentGame)); Assert.assertFalse(predator + \" should not be a Horror\", permanent.hasSubtype(SubType.HORROR, currentGame)); Assert.assertEquals(predator + \" should have mana value 1\", 1, permanent.getManaValue()); Assert.assertTrue(prowler + \" should be transformed\", permanent.isTransformed()); break; } }"
        }
      ]
    },
    {
      "name": "testCopyTransformedDFC",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Tropical Island",
          "count": 9
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Kessig Prowler",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Rite of Replication",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{4}{G}"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Rite of Replication",
          "target": "Sinuous Predator"
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
          "name": "Rite of Replication",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Kessig Prowler",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Sinuous Predator",
          "count": 2
        },
        {
          "op": "unsupported",
          "source": "for (Permanent permanent : currentGame.getBattlefield().getAllActivePermanents()) { switch (permanent.getName()) { case prowler: Assert.assertEquals(\"Power of \" + prowler + \" should be 2\", 2, permanent.getPower().getValue()); Assert.assertEquals(\"Toughness of \" + prowler + \" should be 1\", 1, permanent.getToughness().getValue()); Assert.assertEquals(prowler + \" should be green\", ObjectColor.GREEN, permanent.getColor(currentGame)); Assert.assertTrue(prowler + \" should be a Werewolf\", permanent.hasSubtype(SubType.WEREWOLF, currentGame)); Assert.assertTrue(prowler + \" should be a Horror\", permanent.hasSubtype(SubType.HORROR, currentGame)); Assert.assertFalse(prowler + \" should not be an Eldrazi\", permanent.hasSubtype(SubType.ELDRAZI, currentGame)); Assert.assertEquals(prowler + \" should have mana value 1\", 1, permanent.getManaValue()); Assert.assertFalse(prowler + \" should not be transformed\", permanent.isTransformed()); break; case predator: Assert.assertEquals(\"Power of \" + predator + \" should be 4\", 4, permanent.getPower().getValue()); Assert.assertEquals(\"Toughness of \" + predator + \" should be 4\", 4, permanent.getToughness().getValue()); Assert.assertTrue(predator + \" should be colorless\", permanent.getColor(currentGame).isColorless()); Assert.assertTrue(predator + \" should be an Eldrazi\", permanent.hasSubtype(SubType.ELDRAZI, currentGame)); Assert.assertTrue(predator + \" should be a Werewolf\", permanent.hasSubtype(SubType.WEREWOLF, currentGame)); Assert.assertFalse(predator + \" should not be a Horror\", permanent.hasSubtype(SubType.HORROR, currentGame)); Assert.assertEquals(predator + \" should have mana value 1\", 1, permanent.getManaValue()); Assert.assertTrue(prowler + \" should be transformed\", permanent.isTransformed()); break; } }"
        }
      ]
    },
    {
      "name": "testBackFromTheBrink",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Tropical Island",
          "count": 7
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Back from the Brink",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Kessig Prowler",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "Exile"
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
          "op": "assertExileCount",
          "player": 0,
          "name": "Kessig Prowler",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Kessig Prowler",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Sinuous Predator",
          "count": 0
        },
        {
          "op": "unsupported",
          "source": "for (Permanent permanent : currentGame.getBattlefield().getAllActivePermanents()) { switch (permanent.getName()) { case prowler: Assert.assertEquals(\"Power of \" + prowler + \" should be 2\", 2, permanent.getPower().getValue()); Assert.assertEquals(\"Toughness of \" + prowler + \" should be 1\", 1, permanent.getToughness().getValue()); Assert.assertEquals(prowler + \" should be green\", ObjectColor.GREEN, permanent.getColor(currentGame)); Assert.assertTrue(prowler + \" should be a Werewolf\", permanent.hasSubtype(SubType.WEREWOLF, currentGame)); Assert.assertTrue(prowler + \" should be a Horror\", permanent.hasSubtype(SubType.HORROR, currentGame)); Assert.assertFalse(prowler + \" should not be an Eldrazi\", permanent.hasSubtype(SubType.ELDRAZI, currentGame)); Assert.assertEquals(prowler + \" should have mana value 1\", 1, permanent.getManaValue()); Assert.assertFalse(prowler + \" should not be transformed\", permanent.isTransformed()); break; case predator: Assert.assertEquals(\"Power of \" + predator + \" should be 4\", 4, permanent.getPower().getValue()); Assert.assertEquals(\"Toughness of \" + predator + \" should be 4\", 4, permanent.getToughness().getValue()); Assert.assertTrue(predator + \" should be colorless\", permanent.getColor(currentGame).isColorless()); Assert.assertTrue(predator + \" should be an Eldrazi\", permanent.hasSubtype(SubType.ELDRAZI, currentGame)); Assert.assertTrue(predator + \" should be a Werewolf\", permanent.hasSubtype(SubType.WEREWOLF, currentGame)); Assert.assertFalse(predator + \" should not be a Horror\", permanent.hasSubtype(SubType.HORROR, currentGame)); Assert.assertEquals(predator + \" should have mana value 1\", 1, permanent.getManaValue()); Assert.assertTrue(prowler + \" should be transformed\", permanent.isTransformed()); break; } }"
        }
      ]
    },
    {
      "name": "testBackFromTheBrinkTransform",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Tropical Island",
          "count": 12
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Back from the Brink",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Kessig Prowler",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "Exile"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "ability": "{4}{G}"
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
          "op": "assertExileCount",
          "player": 0,
          "name": "Kessig Prowler",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Kessig Prowler",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Sinuous Predator",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "for (Permanent permanent : currentGame.getBattlefield().getAllActivePermanents()) { switch (permanent.getName()) { case prowler: Assert.assertEquals(\"Power of \" + prowler + \" should be 2\", 2, permanent.getPower().getValue()); Assert.assertEquals(\"Toughness of \" + prowler + \" should be 1\", 1, permanent.getToughness().getValue()); Assert.assertEquals(prowler + \" should be green\", ObjectColor.GREEN, permanent.getColor(currentGame)); Assert.assertTrue(prowler + \" should be a Werewolf\", permanent.hasSubtype(SubType.WEREWOLF, currentGame)); Assert.assertTrue(prowler + \" should be a Horror\", permanent.hasSubtype(SubType.HORROR, currentGame)); Assert.assertFalse(prowler + \" should not be an Eldrazi\", permanent.hasSubtype(SubType.ELDRAZI, currentGame)); Assert.assertEquals(prowler + \" should have mana value 1\", 1, permanent.getManaValue()); Assert.assertFalse(prowler + \" should not be transformed\", permanent.isTransformed()); break; case predator: Assert.assertEquals(\"Power of \" + predator + \" should be 4\", 4, permanent.getPower().getValue()); Assert.assertEquals(\"Toughness of \" + predator + \" should be 4\", 4, permanent.getToughness().getValue()); Assert.assertTrue(predator + \" should be colorless\", permanent.getColor(currentGame).isColorless()); Assert.assertTrue(predator + \" should be an Eldrazi\", permanent.hasSubtype(SubType.ELDRAZI, currentGame)); Assert.assertTrue(predator + \" should be a Werewolf\", permanent.hasSubtype(SubType.WEREWOLF, currentGame)); Assert.assertFalse(predator + \" should not be a Horror\", permanent.hasSubtype(SubType.HORROR, currentGame)); Assert.assertEquals(predator + \" should have mana value 1\", 1, permanent.getManaValue()); Assert.assertTrue(prowler + \" should be transformed\", permanent.isTransformed()); break; } }"
        }
      ]
    }
  ]
});
