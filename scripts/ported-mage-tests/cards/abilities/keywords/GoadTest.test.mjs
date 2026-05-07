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
        },
        {
          "op": "unsupported",
          "source": "assertGoaded(lion, playerA)"
        },
        {
          "op": "unsupported",
          "source": "assertAttacking(lion, playerB, playerC)"
        }
      ]
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
        },
        {
          "op": "unsupported",
          "source": "assertGoaded(lion, playerA)"
        },
        {
          "op": "unsupported",
          "source": "assertAttacking(lion, playerC)"
        }
      ]
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
        },
        {
          "op": "unsupported",
          "source": "assertGoaded(lion, playerA)"
        },
        {
          "op": "unsupported",
          "source": "assertAttacking(lion, playerA)"
        }
      ]
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
          "player": "playerD",
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
          "name": "Island",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": "playerC",
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
          "op": "addTarget",
          "player": "playerD",
          "target": "Silvercoat Lion"
        },
        {
          "op": "setChoice",
          "player": "playerD",
          "value": "Yes"
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": "playerD",
          "name": "Jeering Homunculus"
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
          "op": "unsupported",
          "source": "assertGoaded(lion, playerA, playerD)"
        },
        {
          "op": "unsupported",
          "source": "assertAttacking(lion, playerB)"
        }
      ]
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
          "player": "playerD",
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
          "player": "playerC",
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
          "op": "addTarget",
          "player": "playerD",
          "target": "Silvercoat Lion"
        },
        {
          "op": "setChoice",
          "player": "playerD",
          "value": "Yes"
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": "playerD",
          "name": "Jeering Homunculus"
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
          "op": "unsupported",
          "source": "assertGoaded(lion, playerA, playerD)"
        },
        {
          "op": "unsupported",
          "source": "assertAttacking(lion, playerA, playerD)"
        }
      ]
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
        },
        {
          "op": "unsupported",
          "source": "assertAttacking(\"Berserkers of Blood Ridge\", playerB, playerC, playerD)"
        }
      ]
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
          "player": "playerD",
          "name": "Goblin Balloon Brigade",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": "playerD",
          "name": "Mountain",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": "playerD",
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
          "target": "playerD"
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": "playerD",
          "name": "Goblin Champion"
        },
        {
          "op": "addTarget",
          "player": "playerD",
          "target": "playerC"
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
          "op": "unsupported",
          "source": "assertGoaded(\"Goblin Balloon Brigade\", playerA)"
        },
        {
          "op": "unsupported",
          "source": "assertNotGoaded(\"Goblin Champion\", playerA)"
        }
      ]
    }
  ]
});
