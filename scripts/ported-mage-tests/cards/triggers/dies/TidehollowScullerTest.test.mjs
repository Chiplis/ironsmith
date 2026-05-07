import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/triggers/dies/TidehollowScullerTest.java",
  "tests": [
    {
      "name": "test_CastOneCardFromHandWillBeExiled",
      "operations": [
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
          "name": "Island",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 3
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
          "zone": "HAND",
          "player": 0,
          "name": "Tidehollow Sculler",
          "count": 1
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
          "zone": "HAND",
          "player": 1,
          "name": "Bloodflow Connoisseur",
          "count": 1
        },
        {
          "op": "assertHandCount",
          "turn": 1,
          "phase": "UPKEEP",
          "player": 1,
          "name": "Bloodflow Connoisseur",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Tidehollow Sculler"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Bloodflow Connoisseur"
        },
        {
          "op": "assertHandCount",
          "turn": 1,
          "phase": "BEGIN_COMBAT",
          "player": 1,
          "name": "Bloodflow Connoisseur",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "turn": 1,
          "phase": "END_COMBAT",
          "player": 0,
          "name": "Tidehollow Sculler",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Lightning Bolt",
          "target": "Tidehollow Sculler"
        },
        {
          "op": "assertPermanentCount",
          "turn": 1,
          "phase": "END_TURN",
          "player": 0,
          "name": "Tidehollow Sculler",
          "count": 0
        },
        {
          "op": "assertHandCount",
          "turn": 1,
          "phase": "END_TURN",
          "player": 1,
          "name": "Bloodflow Connoisseur",
          "count": 1
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
          "player": 1,
          "name": "Bloodflow Connoisseur",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Tidehollow Sculler",
          "count": 0
        }
      ]
    },
    {
      "name": "test_CastTwoCardFromHandWillBeExiled",
      "operations": [
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
          "name": "Island",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Tidehollow Sculler@tide",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Lightning Bolt",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Bloodflow Connoisseur",
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
          "op": "assertHandCount",
          "turn": 1,
          "phase": "UPKEEP",
          "player": 1,
          "name": "Bloodflow Connoisseur",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Tidehollow Sculler"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Bloodflow Connoisseur"
        },
        {
          "op": "assertHandCount",
          "turn": 1,
          "phase": "BEGIN_COMBAT",
          "player": 1,
          "name": "Bloodflow Connoisseur",
          "count": 0
        },
        {
          "op": "assertHandCount",
          "turn": 1,
          "phase": "END_COMBAT",
          "player": 1,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Tidehollow Sculler"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Silvercoat Lion"
        },
        {
          "op": "assertHandCount",
          "turn": 1,
          "phase": "END_TURN",
          "player": 1,
          "name": "Silvercoat Lion",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "turn": 2,
          "phase": "UPKEEP",
          "player": 0,
          "name": "Tidehollow Sculler",
          "count": 2
        },
        {
          "op": "assertHandCount",
          "turn": 2,
          "phase": "UPKEEP",
          "player": 1,
          "name": "Bloodflow Connoisseur",
          "count": 0
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Lightning Bolt",
          "target": "@tide.1"
        },
        {
          "op": "assertPermanentCount",
          "turn": 2,
          "phase": "BEGIN_COMBAT",
          "player": 0,
          "name": "Tidehollow Sculler",
          "count": 1
        },
        {
          "op": "assertHandCount",
          "turn": 2,
          "phase": "BEGIN_COMBAT",
          "player": 1,
          "name": "Bloodflow Connoisseur",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "turn": 2,
          "phase": "END_COMBAT",
          "player": 0,
          "name": "Tidehollow Sculler",
          "count": 1
        },
        {
          "op": "assertHandCount",
          "turn": 2,
          "phase": "END_COMBAT",
          "player": 1,
          "name": "Silvercoat Lion",
          "count": 0
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Lightning Bolt",
          "target": "@tide.2"
        },
        {
          "op": "assertPermanentCount",
          "turn": 2,
          "phase": "END_TURN",
          "player": 0,
          "name": "Tidehollow Sculler",
          "count": 0
        },
        {
          "op": "assertHandCount",
          "turn": 2,
          "phase": "END_TURN",
          "player": 1,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        }
      ]
    },
    {
      "name": "test_MultipleRuns",
      "operations": [
        {
          "op": "unsupported",
          "source": "for (int i = 1; i <= 10; i++) { try { this.reset(); test_CastTwoCardFromHandWillBeExiled(); } catch (Exception e) { } }"
        }
      ]
    }
  ]
});
