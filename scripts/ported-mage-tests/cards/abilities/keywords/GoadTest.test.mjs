import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/abilities/keywords/GoadTest.java",
  "tests": [
    {
      "name": "testCantAttackGoadingPlayer",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Jeering Homunculus",
          "count": 1
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
          "player": "playerD",
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Silvercoat Lion"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Yes"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Jeering Homunculus"
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "DECLARE_BLOCKERS"
        },
        {
          "op": "execute"
        }
      ],
      "turnOrder": [
        0,
        3,
        2,
        1
      ],
      "startingLife": 40
    },
    {
      "name": "testCanOnlyAttackOnePlayer",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Jeering Homunculus",
          "count": 1
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
          "player": "playerD",
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Blazing Archon",
          "count": 1
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Silvercoat Lion"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Yes"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Jeering Homunculus"
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "DECLARE_BLOCKERS"
        },
        {
          "op": "execute"
        }
      ],
      "turnOrder": [
        0,
        3,
        2,
        1
      ],
      "startingLife": 40
    },
    {
      "name": "testMustAttackGoader",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Jeering Homunculus",
          "count": 1
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
          "player": "playerD",
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Blazing Archon",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": "playerC",
          "name": "Blazing Archon",
          "count": 1
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Silvercoat Lion"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Yes"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Jeering Homunculus"
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "DECLARE_BLOCKERS"
        },
        {
          "op": "execute"
        }
      ],
      "turnOrder": [
        0,
        3,
        2,
        1
      ],
      "startingLife": 40
    },
    {
      "name": "testMultipleGoad",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Jeering Homunculus",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 3,
          "name": "Jeering Homunculus",
          "count": 1
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
          "player": 3,
          "name": "Island",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 2,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Silvercoat Lion"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Yes"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Jeering Homunculus"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 2,
            "name": "Silvercoat Lion"
          },
          "count": 1,
          "checks": [
            {
              "property": "goaded",
              "expected": true
            }
          ]
        },
        {
          "op": "addTarget",
          "player": 3,
          "target": "Silvercoat Lion"
        },
        {
          "op": "setChoice",
          "player": 3,
          "value": "Yes"
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 3,
          "name": "Jeering Homunculus"
        },
        {
          "op": "waitStackResolved",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 2,
            "name": "Silvercoat Lion"
          },
          "count": 1,
          "checks": [
            {
              "property": "goaded",
              "expected": true
            }
          ]
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "DECLARE_BLOCKERS"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertAttacking",
          "player": 2,
          "name": "Silvercoat Lion",
          "defenders": [
            1
          ],
          "expected": true
        }
      ],
      "turnOrder": [
        0,
        3,
        2,
        1
      ],
      "startingLife": 40
    },
    {
      "name": "testMultipleGoadRestriction",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Jeering Homunculus",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 3,
          "name": "Jeering Homunculus",
          "count": 1
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
          "player": 3,
          "name": "Island",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Blazing Archon",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 2,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Silvercoat Lion"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Yes"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Jeering Homunculus"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 2,
            "name": "Silvercoat Lion"
          },
          "count": 1,
          "checks": [
            {
              "property": "goaded",
              "expected": true
            }
          ]
        },
        {
          "op": "addTarget",
          "player": 3,
          "target": "Silvercoat Lion"
        },
        {
          "op": "setChoice",
          "player": 3,
          "value": "Yes"
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 3,
          "name": "Jeering Homunculus"
        },
        {
          "op": "waitStackResolved",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 2,
            "name": "Silvercoat Lion"
          },
          "count": 1,
          "checks": [
            {
              "property": "goaded",
              "expected": true
            }
          ]
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "DECLARE_BLOCKERS"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertAttacking",
          "player": 2,
          "name": "Silvercoat Lion",
          "defenders": [
            0,
            3
          ],
          "expected": true
        }
      ],
      "turnOrder": [
        0,
        3,
        2,
        1
      ],
      "startingLife": 40
    },
    {
      "name": "testRegularCombatRequirement",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Berserkers of Blood Ridge",
          "count": 1
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "DECLARE_BLOCKERS"
        },
        {
          "op": "execute"
        }
      ],
      "turnOrder": [
        0,
        3,
        2,
        1
      ],
      "startingLife": 40
    },
    {
      "name": "goadAllCorrectAffect",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Geode Rager",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Swamp",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 3,
          "name": "Goblin Balloon Brigade",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 3,
          "name": "Mountain",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 3,
          "name": "Goblin Champion",
          "count": 1
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "playLand",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Swamp"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": 3
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 3,
          "name": "Goblin Champion"
        },
        {
          "op": "addTarget",
          "player": 3,
          "target": 2
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "DECLARE_BLOCKERS"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 3,
            "name": "Goblin Balloon Brigade"
          },
          "count": 1,
          "checks": [
            {
              "property": "goaded",
              "expected": true
            }
          ]
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 3,
            "name": "Goblin Champion"
          },
          "count": 1,
          "checks": [
            {
              "property": "goaded",
              "expected": false
            }
          ]
        }
      ],
      "turnOrder": [
        0,
        3,
        2,
        1
      ],
      "startingLife": 40
    }
  ]
});
