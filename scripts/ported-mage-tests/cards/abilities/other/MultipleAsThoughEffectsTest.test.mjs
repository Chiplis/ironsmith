import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/abilities/other/MultipleAsThoughEffectsTest.java",
  "tests": [
    {
      "name": "testChoosingAlternateCastingMethod",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "skipInitShuffling"
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Vivien, Monsters' Advocate",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Bolas's Citadel",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Abzan Beastmaster",
          "count": 2
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
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 5
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Vivien, Monsters' Advocate"
        },
        {
          "op": "castSpell",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Abzan Beastmaster"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Vivien"
        },
        {
          "op": "unsupported",
          "source": "checkLife(\"Vivien not making pay life\", 1, PhaseStep.PRECOMBAT_MAIN, playerA, 20)"
        },
        {
          "op": "castSpell",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Abzan Beastmaster"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Bolas's"
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
          "name": "Abzan Beastmaster",
          "count": 2
        },
        {
          "op": "assertTappedCount",
          "name": "Forest",
          "tapped": true,
          "count": 3
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 17
        }
      ]
    },
    {
      "name": "testRisenExecutioner",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Risen Executioner",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Grizzly Bears",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Gisa and Geralf",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 9
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Risen Executioner"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Gisa"
        },
        {
          "op": "unsupported",
          "source": "checkPermanentTapped(\"Swamp tapped after cast with Gisa and Geralf\", 1, PhaseStep.PRECOMBAT_MAIN, playerA, \"Swamp\", true, 4)"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Risen Executioner"
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
          "name": "Risen Executioner",
          "count": 2
        },
        {
          "op": "assertTappedCount",
          "name": "Swamp",
          "tapped": true,
          "count": 9
        }
      ]
    }
  ]
});
