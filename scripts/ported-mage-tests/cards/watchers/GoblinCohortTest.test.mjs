import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/watchers/GoblinCohortTest.java",
  "tests": [
    {
      "name": "testCanAttack",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Mountain",
          "count": 5
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Goblin Cohort",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Goblin Roughrider",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Goblin Roughrider"
        },
        {
          "op": "attack",
          "turn": 2,
          "player": 1,
          "attacker": "Goblin Cohort",
          "defender": 1
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
          "source": "assertAttacking(\"Goblin Cohort\", true)"
        }
      ]
    },
    {
      "name": "testCannotAttack",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Mountain",
          "count": 5
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Goblin Cohort",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Goblin Roughrider",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 2,
          "player": 1,
          "attacker": "Goblin Cohort",
          "defender": 1
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "DECLARE_BLOCKERS"
        },
        {
          "op": "unsupported",
          "source": "try { execute(); } catch (Throwable e) { if (!e.getMessage().contains(\"Player PlayerB must have 0 actions but found 1\")) { Assert.fail(\"must not have throw error about cannot have action, but got:\\n\" + e.getMessage()); } } assertAttacking(\"Goblin Cohort\", false)"
        }
      ]
    }
  ]
});
