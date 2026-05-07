import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/ltr/WitchKingOfAngmarTest.java",
  "tests": [
    {
      "name": "test_Sacrifice",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Witch-king of Angmar",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Swamp",
          "count": 5
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Watchwolf",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Simic Sky Swallower",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Nivix Cyclops",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 2,
          "player": 1,
          "attacker": "Watchwolf",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 2,
          "player": 1,
          "attacker": "Simic Sky Swallower",
          "defender": 1
        },
        {
          "op": "unsupported",
          "source": "checkStackObject(\"Sacrifice trigger check\", 2, PhaseStep.COMBAT_DAMAGE, playerB, \"Whenever one or more creatures deal combat damage to you\", 1)"
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": "Watchwolf"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 11
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "count": 1,
          "name": 2
        }
      ]
    },
    {
      "name": "testIndestructible",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Witch-king of Angmar",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Swamp",
          "count": 5
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "Discard a card:"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Witch-king of Angmar",
          "ability": "Indestructible",
          "expected": true
        },
        {
          "op": "unsupported",
          "source": "assertTapped(witchKing, true)"
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "count": 0,
          "name": 4
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "UNTAP"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Witch-king of Angmar",
          "ability": "Indestructible",
          "expected": false
        }
      ]
    }
  ]
});
