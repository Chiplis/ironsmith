import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/abilities/keywords/HideawayTest.java",
  "tests": [
    {
      "name": "testHideaway",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Shelldock Isle",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "skipInitShuffling"
        },
        {
          "op": "playLand",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Shelldock Isle"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Silvercoat Lion"
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
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Shelldock Isle",
          "count": 1
        },
        {
          "op": "assertExileCount",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "for (Card card : currentGame.getExile().getAllCards(currentGame)) { Assert.assertTrue(\"Exiled card is not face down\", card.isFaceDown(currentGame)); }"
        }
      ]
    },
    {
      "name": "testMosswortBridge",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Mosswort Bridge",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Ulamog, the Ceaseless Hunger",
          "count": 1
        },
        {
          "op": "skipInitShuffling"
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Dross Crocodile",
          "count": 2
        },
        {
          "op": "playLand",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Mosswort Bridge"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Ulamog, the Ceaseless Hunger"
        },
        {
          "op": "activateAbility",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{G},"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Ulamog, the Ceaseless Hunger"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Dross Crocodile^Dross Crocodile"
        },
        {
          "op": "setStopAt",
          "turn": 3,
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
          "op": "assertExileCount",
          "name": "Dross Crocodile",
          "count": 2
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Mosswort Bridge",
          "count": 1
        },
        {
          "op": "assertExileCount",
          "player": 0,
          "count": 2
        },
        {
          "op": "assertExileCount",
          "name": "Ulamog, the Ceaseless Hunger",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Ulamog, the Ceaseless Hunger",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "assertTapped(\"Mosswort Bridge\", true)"
        }
      ]
    },
    {
      "name": "testCannotPlayLandIfPlayedLand",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Windbrisk Heights",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Plains",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Ghost Quarter",
          "count": 1
        },
        {
          "op": "skipInitShuffling"
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
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Auriok Champion",
          "count": 3
        },
        {
          "op": "playLand",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Windbrisk Heights"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Ghost Quarter"
        },
        {
          "op": "playLand",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Plains"
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Auriok Champion",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Auriok Champion",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Auriok Champion",
          "defender": 1
        },
        {
          "op": "activateAbility",
          "turn": 3,
          "phase": "DECLARE_BLOCKERS",
          "player": 0,
          "ability": "{W},"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Ghost Quarter"
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "END_COMBAT"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Ghost Quarter",
          "count": 0
        },
        {
          "op": "unsupported",
          "source": "assertTapped(\"Windbrisk Heights\", true)"
        }
      ]
    },
    {
      "name": "testCannotPlayLandIfNotOwnTurn",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Mosswort Bridge",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Ghost Quarter",
          "count": 1
        },
        {
          "op": "skipInitShuffling"
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Dross Crocodile",
          "count": 2
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "playLand",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Mosswort Bridge"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Ghost Quarter"
        },
        {
          "op": "activateAbility",
          "turn": 4,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{G},"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Ghost Quarter"
        },
        {
          "op": "setStopAt",
          "turn": 4,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "unsupported",
          "source": "assertTapped(\"Mosswort Bridge\", true)"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Ghost Quarter",
          "count": 0
        }
      ]
    },
    {
      "name": "testCanPlayLandIfNotPlayedLand",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Windbrisk Heights",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Ghost Quarter",
          "count": 1
        },
        {
          "op": "skipInitShuffling"
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
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Auriok Champion",
          "count": 3
        },
        {
          "op": "playLand",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Windbrisk Heights"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Ghost Quarter"
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Auriok Champion",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Auriok Champion",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Auriok Champion",
          "defender": 1
        },
        {
          "op": "activateAbility",
          "turn": 3,
          "phase": "DECLARE_BLOCKERS",
          "player": 0,
          "ability": "{W},"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Ghost Quarter"
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "END_COMBAT"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Ghost Quarter",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "assertTapped(\"Windbrisk Heights\", true)"
        }
      ]
    },
    {
      "name": "testCanPlayMoreLandsIfAble",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Windbrisk Heights",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Ghost Quarter",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Plains",
          "count": 1
        },
        {
          "op": "skipInitShuffling"
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
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Auriok Champion",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Fastbond",
          "count": 1
        },
        {
          "op": "playLand",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Windbrisk Heights"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Ghost Quarter"
        },
        {
          "op": "playLand",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Plains"
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Auriok Champion",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Auriok Champion",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Auriok Champion",
          "defender": 1
        },
        {
          "op": "activateAbility",
          "turn": 3,
          "phase": "DECLARE_BLOCKERS",
          "player": 0,
          "ability": "{W},"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Ghost Quarter"
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "END_COMBAT"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Ghost Quarter",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "assertTapped(\"Windbrisk Heights\", true)"
        }
      ]
    },
    {
      "name": "testShelldockIsleHideawayConditionOwnLibrary",
      "operations": [
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
          "name": "Shelldock Isle",
          "count": 1
        },
        {
          "op": "clearZone",
          "player": 0,
          "zone": "library"
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Ulamog's Crusher",
          "count": 4
        },
        {
          "op": "skipInitShuffling"
        },
        {
          "op": "playLand",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Shelldock Isle"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Ulamog's Crusher"
        },
        {
          "op": "activateAbility",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{U}"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Ulamog's Crusher"
        },
        {
          "op": "setStopAt",
          "turn": 3,
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
          "op": "assertTappedCount",
          "name": "Island",
          "tapped": true,
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Shelldock Isle",
          "count": 1
        },
        {
          "op": "assertLibraryCount",
          "player": 0,
          "count": 2
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Ulamog's Crusher",
          "count": 1
        }
      ]
    },
    {
      "name": "testShelldockIsleHideawayConditionOpponentsLibrary",
      "operations": [
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
          "name": "Shelldock Isle",
          "count": 1
        },
        {
          "op": "clearZone",
          "player": 1,
          "zone": "library"
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Ulamog's Crusher",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 1,
          "name": "Bronze Sable",
          "count": 4
        },
        {
          "op": "skipInitShuffling"
        },
        {
          "op": "playLand",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Shelldock Isle"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Ulamog's Crusher"
        },
        {
          "op": "activateAbility",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{U}"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Ulamog's Crusher"
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
          "name": "Shelldock Isle",
          "count": 1
        },
        {
          "op": "assertLibraryCount",
          "player": 1,
          "count": 3
        },
        {
          "op": "assertTappedCount",
          "name": "Island",
          "tapped": true,
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Ulamog's Crusher",
          "count": 1
        }
      ]
    },
    {
      "name": "testMultipleHideawayTriggers",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Windbrisk Heights",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Llanowar Elves",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Auriok Champion",
          "count": 4
        },
        {
          "op": "skipInitShuffling"
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
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Auriok Glaivemaster",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Elesh Norn, Mother of Machines",
          "count": 1
        },
        {
          "op": "playLand",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Windbrisk Heights"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Hideaway 4"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Auriok Champion"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Llanowar Elves"
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Auriok Glaivemaster",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Auriok Glaivemaster",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Elesh Norn, Mother of Machines",
          "defender": 1
        },
        {
          "op": "activateAbility",
          "turn": 3,
          "phase": "DECLARE_BLOCKERS",
          "player": 0,
          "ability": "{W},"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Llanowar Elves^Auriok Champion"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Whenever"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "END_COMBAT"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Llanowar Elves",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Auriok Champion",
          "count": 1
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 22
        },
        {
          "op": "unsupported",
          "source": "assertTapped(\"Windbrisk Heights\", true)"
        }
      ]
    },
    {
      "name": "testMultipleHideawayTriggersPlayOneLand",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Windbrisk Heights",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Field of the Dead",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Ghost Quarter",
          "count": 4
        },
        {
          "op": "skipInitShuffling"
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
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Auriok Champion",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Elesh Norn, Mother of Machines",
          "count": 1
        },
        {
          "op": "playLand",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Windbrisk Heights"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Hideaway 4"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Ghost Quarter"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Field of the Dead"
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Auriok Champion",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Auriok Champion",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Auriok Champion",
          "defender": 1
        },
        {
          "op": "activateAbility",
          "turn": 3,
          "phase": "DECLARE_BLOCKERS",
          "player": 0,
          "ability": "{W},"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Ghost Quarter^Field of the Dead"
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "END_COMBAT"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Ghost Quarter",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "assertTapped(\"Windbrisk Heights\", true)"
        },
        {
          "op": "assertExileCount",
          "player": 0,
          "count": 1
        }
      ]
    },
    {
      "name": "testMultipleHideawayTriggersPlayMultipleLands",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Windbrisk Heights",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Ghost Quarter",
          "count": 5
        },
        {
          "op": "skipInitShuffling"
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
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Auriok Champion",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Elesh Norn, Mother of Machines",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Fastbond",
          "count": 1
        },
        {
          "op": "playLand",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Windbrisk Heights"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Hideaway 4"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Ghost Quarter"
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Auriok Champion",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Auriok Champion",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Auriok Champion",
          "defender": 1
        },
        {
          "op": "activateAbility",
          "turn": 3,
          "phase": "DECLARE_BLOCKERS",
          "player": 0,
          "ability": "{W},"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Ghost Quarter^Ghost Quarter"
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "END_COMBAT"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Ghost Quarter",
          "count": 2
        },
        {
          "op": "unsupported",
          "source": "assertTapped(\"Windbrisk Heights\", true)"
        }
      ]
    },
    {
      "name": "testWatcherForTomorrowLeft",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Watcher for Tomorrow",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Ephemerate",
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
          "player": 0,
          "name": "Plains",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 4
        },
        {
          "op": "skipInitShuffling"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Watcher for Tomorrow"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Silvercoat Lion"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Ephemerate",
          "target": "Watcher for Tomorrow"
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
          "name": "Watcher for Tomorrow",
          "count": 1
        },
        {
          "op": "assertExileCount",
          "player": 0,
          "name": "Ephemerate",
          "count": 1
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "assertExileCount",
          "player": 0,
          "count": 2
        },
        {
          "op": "unsupported",
          "source": "assertTapped(\"Watcher for Tomorrow\", true)"
        }
      ]
    }
  ]
});
