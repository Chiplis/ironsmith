import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/mid/SigardasSplendorTest.java",
  "tests": [
    {
      "name": "sigardasSplendorTestBasic",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Sigarda's Splendor",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 4
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "assertHandCount",
          "player": 1,
          "name": "PRECOMBAT_MAIN",
          "count": 0
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Sigarda's Splendor"
        },
        {
          "op": "assertHandCount",
          "player": 1,
          "name": "END_TURN",
          "count": 0
        },
        {
          "op": "unsupported",
          "source": "checkLife(\"Initial life\", 1, PhaseStep.END_TURN, playerA, 20)"
        },
        {
          "op": "assertHandCount",
          "player": "Did not draw on 1st upkeep",
          "name": 3,
          "count": "PRECOMBAT_MAIN"
        },
        {
          "op": "castSpell",
          "turn": 3,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Sigarda's Splendor"
        },
        {
          "op": "assertHandCount",
          "player": "Did not draw on 1st upkeep (2)",
          "name": 3,
          "count": "END_TURN"
        },
        {
          "op": "unsupported",
          "source": "checkLife(\"Initial life\", 3, PhaseStep.END_TURN, playerA, 21)"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "At the beginning of your upkeep"
        },
        {
          "op": "setStopAt",
          "turn": 5,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Sigarda's Splendor",
          "count": 2
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "count": 0,
          "name": 5
        }
      ]
    },
    {
      "name": "sigardasSplendorTestDamaged",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Sigarda's Splendor",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Scorching Spear",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Mountain",
          "count": 1
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "assertHandCount",
          "player": 1,
          "name": "PRECOMBAT_MAIN",
          "count": 0
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Sigarda's Splendor"
        },
        {
          "op": "assertHandCount",
          "player": 1,
          "name": "END_TURN",
          "count": 0
        },
        {
          "op": "unsupported",
          "source": "checkLife(\"Initial life\", 1, PhaseStep.END_TURN, playerA, 20)"
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Scorching Spear",
          "target": 0
        },
        {
          "op": "unsupported",
          "source": "checkLife(\"Post-spear 1\", 2, PhaseStep.END_TURN, playerA, 19)"
        },
        {
          "op": "assertHandCount",
          "player": "Did not draw on 1st upkeep",
          "name": 3,
          "count": "PRECOMBAT_MAIN"
        },
        {
          "op": "castSpell",
          "turn": 3,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Sigarda's Splendor"
        },
        {
          "op": "assertHandCount",
          "player": "Did not draw on 1st upkeep (2)",
          "name": 3,
          "count": "END_TURN"
        },
        {
          "op": "unsupported",
          "source": "checkLife(\"Post-splendors\", 3, PhaseStep.END_TURN, playerA, 20)"
        },
        {
          "op": "castSpell",
          "turn": 4,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Scorching Spear",
          "target": 0
        },
        {
          "op": "unsupported",
          "source": "checkLife(\"Post-spear 2\", 4, PhaseStep.END_TURN, playerA, 19)"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "At the beginning of your upkeep"
        },
        {
          "op": "setStopAt",
          "turn": 5,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Sigarda's Splendor",
          "count": 2
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "count": 0,
          "name": 3
        }
      ]
    }
  ]
});
