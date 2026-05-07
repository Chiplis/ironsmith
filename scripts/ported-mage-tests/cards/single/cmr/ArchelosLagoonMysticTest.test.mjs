import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/cmr/ArchelosLagoonMysticTest.java",
  "tests": [
    {
      "name": "test_Playable",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Archelos, Lagoon Mystic",
          "count": 1
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
          "zone": "HAND",
          "player": 0,
          "name": "Deranged Outcast",
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
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "unsupported",
          "source": "checkPermanentTapped(\"untapped\", 1, PhaseStep.PRECOMBAT_MAIN, playerA, \"Archelos, Lagoon Mystic\", false, 1)"
        },
        {
          "op": "unsupported",
          "source": "checkPermanentTapped(\"untapped\", 1, PhaseStep.PRECOMBAT_MAIN, playerA, \"Grizzly Bears\", false, 1)"
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Archelos, Lagoon Mystic",
          "defender": 1
        },
        {
          "op": "unsupported",
          "source": "checkPermanentTapped(\"tapped\", 1, PhaseStep.POSTCOMBAT_MAIN, playerA, \"Archelos, Lagoon Mystic\", true, 1)"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Deranged Outcast"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": null
        },
        {
          "op": "unsupported",
          "source": "checkPermanentTapped(\"tapped\", 1, PhaseStep.POSTCOMBAT_MAIN, playerA, \"Archelos, Lagoon Mystic\", true, 1)"
        },
        {
          "op": "unsupported",
          "source": "checkPermanentTapped(\"tapped\", 1, PhaseStep.POSTCOMBAT_MAIN, playerA, \"Deranged Outcast\", true, 1)"
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
        }
      ]
    }
  ]
});
