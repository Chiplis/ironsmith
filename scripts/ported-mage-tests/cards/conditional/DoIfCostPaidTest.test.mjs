import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/conditional/DoIfCostPaidTest.java",
  "tests": [
    {
      "name": "test_NonOptional",
      "operations": [
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
          "name": "Shock",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Awaken the Sky Tyrant",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Shock",
          "target": 1
        },
        {
          "op": "setStrictChooseMode",
          "value": true
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
          "player": 1,
          "name": "Awaken the Sky Tyrant",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Dragon Token",
          "count": 1
        }
      ]
    },
    {
      "name": "test_Optional_ManaVault_1",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mana Vault",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 4
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": false
        },
        {
          "op": "unsupported",
          "source": "checkPermanentTapped(\"must be untapped on start\", 1, PhaseStep.PRECOMBAT_MAIN, playerA, \"Mana Vault\", false, 1)"
        },
        {
          "op": "unsupported",
          "source": "checkLife(\"no damage on untapped\", 1, PhaseStep.PRECOMBAT_MAIN, playerA, 20)"
        },
        {
          "op": "activateManaAbility",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{T}: Add {C}",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "checkPermanentTapped(\"must be tapped after usage\", 2, PhaseStep.PRECOMBAT_MAIN, playerA, \"Mana Vault\", true, 1)"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": false
        },
        {
          "op": "unsupported",
          "source": "checkPermanentTapped(\"must be tapped after dialog\", 3, PhaseStep.PRECOMBAT_MAIN, playerA, \"Mana Vault\", true, 1)"
        },
        {
          "op": "unsupported",
          "source": "checkLife(\"must do damage on tapped\", 3, PhaseStep.PRECOMBAT_MAIN, playerA, 20 - 1)"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "unsupported",
          "source": "checkPermanentTapped(\"must be untapped after dialog\", 5, PhaseStep.PRECOMBAT_MAIN, playerA, \"Mana Vault\", false, 1)"
        },
        {
          "op": "unsupported",
          "source": "checkLife(\"no damage on untapped\", 5, PhaseStep.PRECOMBAT_MAIN, playerA, 20 - 1)"
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
      "name": "test_Optional_ManaVault_2",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mana Vault",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mana Reflection",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mesmeric Orb",
          "count": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "unsupported",
          "source": "checkPermanentTapped(\"must be untapped\", 1, PhaseStep.PRECOMBAT_MAIN, playerA, \"Mana Vault\", false, 1)"
        },
        {
          "op": "unsupported",
          "source": "checkLife(\"no damage on untapped\", 1, PhaseStep.PRECOMBAT_MAIN, playerA, 20)"
        },
        {
          "op": "assertGraveyardCount",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Mountain",
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
        }
      ]
    },
    {
      "name": "testCannotPay",
      "operations": [
        {
          "op": "skipInitShuffling"
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Craw Wurm",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Runeclaw Bear",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Glory Seeker",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Thirst for Meaning",
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
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Thirst for Meaning"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Runeclaw Bear"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Glory Seeker"
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
          "count": 0,
          "name": 3
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Thirst for Meaning",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Runeclaw Bear",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Glory Seeker",
          "count": 1
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "count": 0,
          "name": 1
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "Craw Wurm",
          "count": 1
        }
      ]
    }
  ]
});
