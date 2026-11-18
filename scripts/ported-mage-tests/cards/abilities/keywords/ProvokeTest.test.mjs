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
          "defender": 0
        },
        {
          "op": "assertTapped",
          "name": "Silvercoat Lion",
          "player": 1,
          "tapped": true,
          "turn": 2,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": {
            "groups": [
              [
                "Silvercoat Lion"
              ]
            ]
          }
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Brontotherium",
          "defender": 1
        },
        {
          "op": "assertStackObject",
          "turn": 3,
          "phase": "DECLARE_ATTACKERS",
          "source": "Brontotherium",
          "count": 1,
          "abilityKind": "Triggered",
          "objectTarget": "Silvercoat Lion"
        },
        {
          "op": "waitStackResolved",
          "turn": 3,
          "phase": "DECLARE_ATTACKERS",
          "resolutions": 1
        },
        {
          "op": "block",
          "turn": 3,
          "player": 1,
          "attacker": "Brontotherium",
          "blocker": "Silvercoat Lion"
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "setOptionChoices",
          "player": 0,
          "kind": "distribution",
          "description": "Assign 5 combat damage from Brontotherium (assign exactly 5 total)",
          "choices": [
            "Silvercoat Lion",
            "Silvercoat Lion",
            "Bob",
            "Bob",
            "Bob"
          ]
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
          "defender": 0
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
