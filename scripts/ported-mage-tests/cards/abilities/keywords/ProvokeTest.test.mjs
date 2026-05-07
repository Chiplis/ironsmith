import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/abilities/keywords/ProvokeTest.java",
  "tests": [
    {
      "name": "testProvokeTappedCreature",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Brontotherium",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 2,
          "player": 1,
          "attacker": "Silvercoat Lion",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Brontotherium",
          "defender": 1
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 18
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 17
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Brontotherium",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Silvercoat Lion",
          "count": 1
        }
      ]
    },
    {
      "name": "testProvokeCreatureThatCantBlock",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Brontotherium",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Putrid Imp",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 1,
          "name": "Swamp",
          "count": 7
        },
        {
          "op": "attack",
          "turn": 2,
          "player": 1,
          "attacker": "Putrid Imp",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Brontotherium",
          "defender": 1
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Brontotherium",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Putrid Imp",
          "count": 1
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 18
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 15
        }
      ]
    }
  ]
});
