import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/abilities/keywords/ManifestTest.java",
  "tests": [
    {
      "name": "test_Simple_ManifestFromOwnLibrary",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Soul Summons",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 2
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Soul Summons"
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
          "name": "EmptyNames.FACE_DOWN_CREATURE.getTestCommand()",
          "count": 1
        }
      ]
    },
    {
      "name": "test_Simple_ManifestFromHand",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Scroll of Fate",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Basking Rootwalla",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{T}: Manifest"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Basking Rootwalla"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
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
          "name": "EmptyNames.FACE_DOWN_CREATURE.getTestCommand()",
          "count": 1
        }
      ]
    },
    {
      "name": "test_Simple_ManifestTargetPlayer",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Reality Shift",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Grizzly Bears",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Reality Shift"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Grizzly Bears"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
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
          "name": "EmptyNames.FACE_DOWN_CREATURE.getTestCommand()",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "EmptyNames.FACE_DOWN_CREATURE.getTestCommand()",
          "count": 1
        }
      ]
    },
    {
      "name": "test_Simple_ManifestFromOpponentLibrary",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Thieving Amalgam",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "EmptyNames.FACE_DOWN_CREATURE.getTestCommand()",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "EmptyNames.FACE_DOWN_CREATURE.getTestCommand()",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "EmptyNames.FACE_DOWN_CREATURE.getTestCommand()",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "EmptyNames.FACE_DOWN_CREATURE.getTestCommand()",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "EmptyNames.FACE_DOWN_CREATURE.getTestCommand()",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "EmptyNames.FACE_DOWN_CREATURE.getTestCommand()",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "turn": 4,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "EmptyNames.FACE_DOWN_CREATURE.getTestCommand()",
          "count": 2
        },
        {
          "op": "assertPermanentCount",
          "turn": 4,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "EmptyNames.FACE_DOWN_CREATURE.getTestCommand()",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "turn": 5,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "EmptyNames.FACE_DOWN_CREATURE.getTestCommand()",
          "count": 2
        },
        {
          "op": "assertPermanentCount",
          "turn": 5,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "EmptyNames.FACE_DOWN_CREATURE.getTestCommand()",
          "count": 0
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 5,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "EmptyNames.FACE_DOWN_CREATURE.getTestCommand()",
          "count": 2
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "EmptyNames.FACE_DOWN_CREATURE.getTestCommand()",
          "count": 0
        }
      ]
    },
    {
      "name": "test_ManifestThenBlink_Creature",
      "operations": []
    },
    {
      "name": "test_ManifestThenBlink_Instant",
      "operations": []
    },
    {
      "name": "test_ManifestThenBlink_MDFC_Creature",
      "operations": []
    },
    {
      "name": "test_ManifestThenBlink_MDFC_LandOnMainSide",
      "operations": []
    },
    {
      "name": "test_ManifestThenBlink_MDFC_LandOnSecondSide",
      "operations": []
    },
    {
      "name": "test_ManifestThenBlink_Split_Normal",
      "operations": []
    },
    {
      "name": "test_ManifestThenBlink_Split_Fused",
      "operations": []
    },
    {
      "name": "test_ManifestThenBlink_Split_Aftermath",
      "operations": []
    },
    {
      "name": "test_ManifestThenBlink_Meld",
      "operations": []
    },
    {
      "name": "test_ManifestThenBlink_Adventure",
      "operations": []
    },
    {
      "name": "testETBTriggeredAbilities",
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
          "name": "Soul Summons",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Tranquil Cove",
          "count": 1
        },
        {
          "op": "skipInitShuffling"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Soul Summons"
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
          "op": "assertLife",
          "player": 0,
          "life": 20
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 20
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "EmptyNames.FACE_DOWN_CREATURE.getTestCommand()",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "EmptyNames.FACE_DOWN_CREATURE.getTestCommand()",
          "power": 2,
          "toughness": 2
        },
        {
          "op": "unsupported",
          "source": "assertTapped(EmptyNames.FACE_DOWN_CREATURE.getTestCommand(), false)"
        }
      ]
    },
    {
      "name": "testETBTriggeredAbilities2",
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
          "name": "Soul Summons",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Doomwake Giant",
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
          "op": "skipInitShuffling"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Soul Summons"
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
          "op": "assertLife",
          "player": 0,
          "life": 20
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 20
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "EmptyNames.FACE_DOWN_CREATURE.getTestCommand()",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "EmptyNames.FACE_DOWN_CREATURE.getTestCommand()",
          "power": 2,
          "toughness": 2
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 1,
          "name": "Silvercoat Lion",
          "power": 2,
          "toughness": 2
        }
      ]
    },
    {
      "name": "testETBTriggeredAbilities3",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Island",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Reality Shift",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Doomwake Giant",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Pillarfield Ox",
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
          "op": "skipInitShuffling"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Reality Shift",
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
          "op": "assertLife",
          "player": 0,
          "life": 20
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 20
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Reality Shift",
          "count": 1
        },
        {
          "op": "assertExileCount",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "EmptyNames.FACE_DOWN_CREATURE.getTestCommand()",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "EmptyNames.FACE_DOWN_CREATURE.getTestCommand()",
          "power": 2,
          "toughness": 2
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Pillarfield Ox",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 1,
          "name": "Pillarfield Ox",
          "power": 2,
          "toughness": 4
        }
      ]
    },
    {
      "name": "testNylea",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Island",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Reality Shift",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Nylea, God of the Hunt",
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
          "op": "skipInitShuffling"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Reality Shift",
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
          "op": "assertLife",
          "player": 0,
          "life": 20
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 20
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Reality Shift",
          "count": 1
        },
        {
          "op": "assertExileCount",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "EmptyNames.FACE_DOWN_CREATURE.getTestCommand()",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "EmptyNames.FACE_DOWN_CREATURE.getTestCommand()",
          "power": 2,
          "toughness": 2
        }
      ]
    },
    {
      "name": "testColorOfManifestedCardDoesNotCount",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Island",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Reality Shift",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Gore Swine",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Foundry Street Denizen",
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
          "op": "skipInitShuffling"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Reality Shift",
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
          "op": "assertLife",
          "player": 0,
          "life": 20
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 20
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Reality Shift",
          "count": 1
        },
        {
          "op": "assertExileCount",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "EmptyNames.FACE_DOWN_CREATURE.getTestCommand()",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "EmptyNames.FACE_DOWN_CREATURE.getTestCommand()",
          "power": 2,
          "toughness": 2
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Foundry Street Denizen",
          "power": 1,
          "toughness": 1
        }
      ]
    },
    {
      "name": "testCardGetsExiledFaceUp",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Island",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Swamp",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Reality Shift",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Silence the Believers",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Gore Swine",
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
          "op": "skipInitShuffling"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Reality Shift",
          "target": "Silvercoat Lion"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 1,
          "name": "Silence the Believers",
          "target": "EmptyNames.FACE_DOWN_CREATURE.getTestCommand()"
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
          "life": 20
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 20
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Reality Shift",
          "count": 1
        },
        {
          "op": "assertExileCount",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "assertExileCount",
          "player": 0,
          "name": "Gore Swine",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "EmptyNames.FACE_DOWN_CREATURE.getTestCommand()",
          "count": 0
        },
        {
          "op": "unsupported",
          "source": "for (Card card : currentGame.getExile().getAllCards(currentGame)) { if (card.getName().equals(\"Gore Swine\")) { Assert.assertTrue(\"Gore Swine may not be face down in exile\", !card.isFaceDown(currentGame)); } }"
        }
      ]
    },
    {
      "name": "testManifestAura",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Swamp",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Qarsi High Priest",
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
          "zone": "LIBRARY",
          "player": 1,
          "name": "Illusory Gains",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 1,
          "name": "Mountain",
          "count": 1
        },
        {
          "op": "skipInitShuffling"
        },
        {
          "op": "activateAbility",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "ability": "{1}{B}, {T}, Sacrifice another creature"
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": "Silvercoat Lion"
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
          "op": "assertLife",
          "player": 0,
          "life": 20
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 20
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Illusory Gains",
          "count": 0
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "EmptyNames.FACE_DOWN_CREATURE.getTestCommand()",
          "count": 1
        }
      ]
    },
    {
      "name": "testManifestMegamorph_TurnUpByMegamorphCost",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Swamp",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Forest",
          "count": 6
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Qarsi High Priest",
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
          "zone": "LIBRARY",
          "player": 1,
          "name": "Aerie Bowmasters",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 1,
          "name": "Mountain",
          "count": 1
        },
        {
          "op": "skipInitShuffling"
        },
        {
          "op": "activateAbility",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "ability": "{1}{B}, {T}, Sacrifice another creature"
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": "Silvercoat Lion"
        },
        {
          "op": "activateAbility",
          "turn": 2,
          "phase": "POSTCOMBAT_MAIN",
          "player": 1,
          "ability": "{5}{G}: Turn"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
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
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "EmptyNames.FACE_DOWN_CREATURE.getTestCommand()",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Aerie Bowmasters",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 1,
          "name": "Aerie Bowmasters",
          "power": 4,
          "toughness": 5
        }
      ]
    },
    {
      "name": "testManifestMegamorph_TurnUpBySimpleCost",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Swamp",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Forest",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Qarsi High Priest",
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
          "zone": "LIBRARY",
          "player": 1,
          "name": "Aerie Bowmasters",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 1,
          "name": "Mountain",
          "count": 1
        },
        {
          "op": "skipInitShuffling"
        },
        {
          "op": "activateAbility",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "ability": "{1}{B}, {T}, Sacrifice another creature"
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": "Silvercoat Lion"
        },
        {
          "op": "activateAbility",
          "turn": 2,
          "phase": "POSTCOMBAT_MAIN",
          "player": 1,
          "ability": "{2}{G}{G}: Turn"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
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
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "EmptyNames.FACE_DOWN_CREATURE.getTestCommand()",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Aerie Bowmasters",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 1,
          "name": "Aerie Bowmasters",
          "power": 3,
          "toughness": 4
        }
      ]
    },
    {
      "name": "testManifestForest",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Swamp",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Courser of Kruphix",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Qarsi High Priest",
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
          "zone": "LIBRARY",
          "player": 1,
          "name": "Forest",
          "count": 1
        },
        {
          "op": "skipInitShuffling"
        },
        {
          "op": "activateAbility",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "ability": "{1}{B}, {T}, Sacrifice another creature"
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": "Silvercoat Lion"
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
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
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "EmptyNames.FACE_DOWN_CREATURE.getTestCommand()",
          "count": 1
        }
      ]
    },
    {
      "name": "testWhisperwoodElemental",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Seismic Rupture",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Whisperwood Elemental",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Silvercoat Lion",
          "count": 2
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "ability": "Sacrifice"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Seismic Rupture"
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": "When {this} dies"
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
          "life": 20
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 20
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Seismic Rupture",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Whisperwood Elemental",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Silvercoat Lion",
          "count": 2
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "EmptyNames.FACE_DOWN_CREATURE.getTestCommand()",
          "count": 2
        }
      ]
    },
    {
      "name": "testDiesTriggeredAbilitiesOfManifestedCreatures",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Swamp",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Nantuko Husk",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Qarsi High Priest",
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
          "zone": "LIBRARY",
          "player": 1,
          "name": "Mountain",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 1,
          "name": "Smothering Abomination",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 1,
          "name": "Mountain",
          "count": 1
        },
        {
          "op": "skipInitShuffling"
        },
        {
          "op": "activateAbility",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "ability": "{1}{B}, {T}, Sacrifice another creature"
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": "Silvercoat Lion"
        },
        {
          "op": "activateAbility",
          "turn": 2,
          "phase": "POSTCOMBAT_MAIN",
          "player": 1,
          "ability": "Sacrifice a creature"
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": "EmptyNames.FACE_DOWN_CREATURE.getTestCommand()"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
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
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Qarsi High Priest",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Nantuko Husk",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Smothering Abomination",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 1,
          "name": "Nantuko Husk",
          "power": 4,
          "toughness": 4
        },
        {
          "op": "assertHandCount",
          "player": 1,
          "name": "Mountain",
          "count": 1
        }
      ]
    },
    {
      "name": "test_ManifestSorceryAndBlinkIt",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Swamp",
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
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Qarsi High Priest",
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
          "zone": "HAND",
          "player": 1,
          "name": "Cloudshift",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 1,
          "name": "Mountain",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 1,
          "name": "Lightning Bolt",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 1,
          "name": "Mountain",
          "count": 1
        },
        {
          "op": "skipInitShuffling"
        },
        {
          "op": "activateAbility",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "ability": "{1}{B}, {T}, Sacrifice another creature"
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": "Silvercoat Lion"
        },
        {
          "op": "waitStackResolved",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Cloudshift",
          "target": "EmptyNames.FACE_DOWN_CREATURE.getTestCommand()"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
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
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Qarsi High Priest",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Cloudshift",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Lightning Bolt",
          "count": 0
        },
        {
          "op": "assertExileCount",
          "player": 1,
          "name": "Lightning Bolt",
          "count": 1
        },
        {
          "op": "assertHandCount",
          "player": 1,
          "name": "Mountain",
          "count": 1
        }
      ]
    },
    {
      "name": "test_ManifestNonPermanentWithCounters",
      "operations": [
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
          "name": "Fierce Invocation",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Lightning Bolt",
          "count": 1
        },
        {
          "op": "skipInitShuffling"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Fierce Invocation"
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
          "op": "assertPowerToughness",
          "player": 0,
          "name": "EmptyNames.FACE_DOWN_CREATURE.getTestCommand()",
          "power": 4,
          "toughness": 4
        }
      ]
    },
    {
      "name": "test_FaceDownCardsMustBeVisibleOnGameEnd",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Reality Shift",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
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
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Reality Shift",
          "target": "Silvercoat Lion"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
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
          "source": "assertFaceDownManifest(\"end game: must show for opponent\", permanent, \"Mountain\", true)"
        },
        {
          "op": "unsupported",
          "source": "assertFaceDownManifest(\"end game: must show for yourself\", permanent, \"Mountain\", true)"
        }
      ]
    },
    {
      "name": "testJeskaiInfiltrator",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Tundra",
          "count": 8
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Jeskai Infiltrator",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Lone Missionary",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Excommunicate",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Excommunicate",
          "target": "Lone Missionary"
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Jeskai Infiltrator",
          "defender": 1
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "label": "{1}{W}: Turn ",
          "expected": true
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "label": "{2}{U}: Turn ",
          "expected": true
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
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Excommunicate",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "EmptyNames.FACE_DOWN_CREATURE.getTestCommand()",
          "count": 2
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "EmptyNames.FACE_DOWN_CREATURE.getTestCommand()",
          "power": 2,
          "toughness": 2
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 20
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 18
        }
      ]
    }
  ]
});
