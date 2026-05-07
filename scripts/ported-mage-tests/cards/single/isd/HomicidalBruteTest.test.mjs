import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/isd/HomicidalBruteTest.java",
  "tests": [
    {
      "name": "testCard",
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
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Civilized Scholar",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Sejiri Merfolk",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{T}: Draw a card, then discard a card."
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
          "name": "Civilized Scholar",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Homicidal Brute",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "assertTapped(\"Homicidal Brute\", false)"
        }
      ]
    },
    {
      "name": "testCardNegative",
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
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Civilized Scholar",
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
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{T}: Draw a card, then discard a card."
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
          "name": "Civilized Scholar",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "assertTapped(\"Civilized Scholar\", true)"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Homicidal Brute",
          "count": 0
        }
      ]
    },
    {
      "name": "testCardTransform",
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
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Civilized Scholar",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Sejiri Merfolk",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{T}: Draw a card, then discard a card."
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "UPKEEP"
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
          "name": "Civilized Scholar",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "assertTapped(\"Civilized Scholar\", true)"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Homicidal Brute",
          "count": 0
        }
      ]
    },
    {
      "name": "testCardNotTransform",
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
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Civilized Scholar",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Sejiri Merfolk",
          "count": 2
        },
        {
          "op": "activateAbility",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{T}: Draw a card, then discard a card."
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Homicidal Brute",
          "defender": 1
        },
        {
          "op": "setStopAt",
          "turn": 4,
          "phase": "UPKEEP"
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
          "life": 15
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Civilized Scholar",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Homicidal Brute",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "assertTapped(\"Homicidal Brute\", true)"
        }
      ]
    },
    {
      "name": "testCardBlinkNotTransform",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Civilized Scholar",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Sejiri Merfolk",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Moonmist",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Cloudshift",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Savannah",
          "count": 3
        },
        {
          "op": "activateAbility",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{T}: Draw a card, then discard a card."
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Sejiri Merfolk"
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Homicidal Brute",
          "defender": 1
        },
        {
          "op": "castSpell",
          "turn": 3,
          "phase": "COMBAT_DAMAGE",
          "player": 0,
          "name": "Cloudshift",
          "target": "Homicidal Brute"
        },
        {
          "op": "castSpell",
          "turn": 3,
          "phase": "END_COMBAT",
          "player": 0,
          "name": "Moonmist"
        },
        {
          "op": "unsupported",
          "source": "checkPermanentTapped(\"after transform\", 3, PhaseStep.POSTCOMBAT_MAIN, playerA, \"Homicidal Brute\", false, 1)"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 4,
          "phase": "UPKEEP"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Cloudshift",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Moonmist",
          "count": 1
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 20
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 15
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Civilized Scholar",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Homicidal Brute",
          "count": 0
        },
        {
          "op": "unsupported",
          "source": "assertTapped(\"Civilized Scholar\", true)"
        }
      ]
    }
  ]
});
