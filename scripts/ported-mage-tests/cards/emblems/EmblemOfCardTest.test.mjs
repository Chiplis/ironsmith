import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/emblems/EmblemOfCardTest.java",
  "tests": [
    {
      "name": "testEmblemOfGriselbrand",
      "operations": [
        {
          "op": "unsupported",
          "source": "addEmblem(playerA, new EmblemOfCard( CardRepository.instance.findCard(\"Griselbrand\", true).createMockCard() ))"
        },
        {
          "op": "setLife",
          "player": 0,
          "life": 20
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "count": 0
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "Pay 7 life: Draw"
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
          "op": "assertHandCount",
          "player": 0,
          "count": 7
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 13
        },
        {
          "op": "unsupported",
          "source": "assertEmblemCount(playerA, 1)"
        }
      ]
    },
    {
      "name": "testEmblemOfYurlok",
      "operations": [
        {
          "op": "unsupported",
          "source": "addEmblem(playerA, new EmblemOfCard( CardRepository.instance.findCard(\"Yurlok of Scorch Thrash\", true).createMockCard() ))"
        },
        {
          "op": "setLife",
          "player": 0,
          "life": 20
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 1
        },
        {
          "op": "activateManaAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{T}: Add {R}",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "checkManaPool(\"after tapping Mountain\", 1, PhaseStep.PRECOMBAT_MAIN, playerA, \"R\", 1)"
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "label": "{1}, {T}:",
          "expected": false
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "unsupported",
          "source": "checkLife(\"takes 1 point of mana burn\", 1, PhaseStep.BEGIN_COMBAT, playerA, 19)"
        },
        {
          "op": "execute"
        },
        {
          "op": "unsupported",
          "source": "assertEmblemCount(playerA, 1)"
        }
      ]
    },
    {
      "name": "testEmblemOfOmniscience",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "unsupported",
          "source": "addEmblem(playerA, new EmblemOfCard( CardRepository.instance.findCard(\"Omniscience\", true).createMockCard() ))"
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Colossal Dreadmaw",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Colossal Dreadmaw"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Cast without paying its mana cost (source: Omniscience"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Colossal Dreadmaw",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "assertEmblemCount(playerA, 1)"
        }
      ]
    },
    {
      "name": "testEmblemOfParadoxEngine",
      "operations": [
        {
          "op": "unsupported",
          "source": "addEmblem(playerA, new EmblemOfCard( CardRepository.instance.findCard(\"Paradox Engine\", true).createMockCard() ))"
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mox Emerald",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Sol Ring",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Basalt Monolith",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Book of Rass",
          "count": 1
        },
        {
          "op": "setLife",
          "player": 0,
          "life": 20
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Sol Ring"
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
          "name": "Basalt Monolith"
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
          "name": "Book of Rass"
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
          "ability": "{2}, Pay"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{2}, Pay"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{2}, Pay"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
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
          "life": 14
        },
        {
          "op": "unsupported",
          "source": "assertEmblemCount(playerA, 1)"
        }
      ]
    },
    {
      "name": "testEmblemOfDoublingSeason",
      "operations": [
        {
          "op": "unsupported",
          "source": "addEmblem(playerA, new EmblemOfCard( CardRepository.instance.findCard(\"Doubling Season\", true).createMockCard() ))"
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 6
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Elspeth, Sun's Champion",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Elspeth, Sun's Champion"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "unsupported",
          "source": "checkPermanentCounters( \"Elspeth's loyalty is doubled\", 1, PhaseStep.PRECOMBAT_MAIN, playerA, \"Elspeth, Sun's Champion\", CounterType.LOYALTY, 8 )"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "+1: Create"
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "label": "Pay 7 life:",
          "expected": false
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "unsupported",
          "source": "checkPermanentCounters( \"+1 is not doubled\", 1, PhaseStep.PRECOMBAT_MAIN, playerA, \"Elspeth, Sun's Champion\", CounterType.LOYALTY, 9 )"
        },
        {
          "op": "assertPermanentCount",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Soldier Token",
          "count": 6
        },
        {
          "op": "execute"
        },
        {
          "op": "unsupported",
          "source": "assertEmblemCount(playerA, 1)"
        }
      ]
    },
    {
      "name": "testEmblemOfMaelstromNexus",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "unsupported",
          "source": "addEmblem(playerA, new EmblemOfCard( CardRepository.instance.findCard(\"Maelstrom Nexus\", true).createMockCard() ))"
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Grizzly Bears",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 2
        },
        {
          "op": "skipInitShuffling"
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Elite Vanguard",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Grizzly Bears"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Elite Vanguard",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Grizzly Bears",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "assertEmblemCount(playerA, 1)"
        }
      ]
    }
  ]
});
