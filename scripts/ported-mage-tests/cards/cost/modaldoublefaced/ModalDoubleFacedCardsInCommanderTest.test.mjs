import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/cost/modaldoublefaced/ModalDoubleFacedCardsInCommanderTest.java",
  "tests": [
    {
      "name": "test_Triggers_MustAddTriggersOneTimeOnly",
      "operations": [
        {
          "op": "clearZone",
          "player": 0,
          "zone": "hand"
        },
        {
          "op": "clearZone",
          "player": 0,
          "zone": "library"
        },
        {
          "op": "skipInitShuffling"
        },
        {
          "op": "addCard",
          "zone": "COMMAND",
          "player": 0,
          "name": "Esika, God of the Tree",
          "count": 1
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
          "name": "Island",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 1
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
          "zone": "LIBRARY",
          "player": 0,
          "name": "Forest",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Grizzly Bears",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Forest",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Grizzly Bears",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Forest",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Grizzly Bears",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Forest",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Grizzly Bears",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Forest",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Grizzly Bears",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Forest",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "The Prismatic Bridge"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "assertPermanentCount",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "The Prismatic Bridge",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "checkLibraryCount(\"prepare\", 1, PhaseStep.PRECOMBAT_MAIN, playerA, \"Grizzly Bears\", 5)"
        },
        {
          "op": "unsupported",
          "source": "checkLibraryCount(\"after upkeep 1\", 3, PhaseStep.PRECOMBAT_MAIN, playerA, \"Grizzly Bears\", 5 - 1)"
        },
        {
          "op": "assertPermanentCount",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Grizzly Bears",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "checkLibraryCount(\"after upkeep 2\", 5, PhaseStep.PRECOMBAT_MAIN, playerA, \"Grizzly Bears\", 5 - 2)"
        },
        {
          "op": "assertPermanentCount",
          "turn": 5,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Grizzly Bears",
          "count": 2
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
        }
      ]
    },
    {
      "name": "test_Triggers_MustWorkFromCommandZone",
      "operations": [
        {
          "op": "addCard",
          "zone": "COMMAND",
          "player": 0,
          "name": "Oloro, Ageless Ascetic",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "checkLife(\"after upkeep 1\", 1, PhaseStep.PRECOMBAT_MAIN, playerA, 40 + 2)"
        },
        {
          "op": "unsupported",
          "source": "checkLife(\"after upkeep 2\", 3, PhaseStep.PRECOMBAT_MAIN, playerA, 40 + 2 + 2)"
        },
        {
          "op": "unsupported",
          "source": "checkLife(\"after upkeep 3\", 5, PhaseStep.PRECOMBAT_MAIN, playerA, 40 + 2 + 2 + 2)"
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
        }
      ]
    }
  ]
});
