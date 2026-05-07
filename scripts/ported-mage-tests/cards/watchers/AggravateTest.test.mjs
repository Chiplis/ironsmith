import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/watchers/AggravateTest.java",
  "tests": [
    {
      "name": "testDamagedCreaturesAttack",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 5
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Craw Wurm",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Goblin Roughrider",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Aggravate",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Aggravate",
          "target": 1
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
          "source": "assertAttacking(\"Craw Wurm\", true)"
        },
        {
          "op": "unsupported",
          "source": "assertAttacking(\"Goblin Roughrider\", true)"
        }
      ]
    },
    {
      "name": "testUndamagedCreaturesDontAttack",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 5
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Mountain",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Craw Wurm",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Goblin Roughrider",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Aggravate",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Raging Goblin",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "UPKEEP",
          "player": 0,
          "name": "Aggravate",
          "target": 1
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Raging Goblin"
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
          "source": "assertAttacking(\"Craw Wurm\", true)"
        },
        {
          "op": "unsupported",
          "source": "assertAttacking(\"Goblin Roughrider\", true)"
        },
        {
          "op": "unsupported",
          "source": "assertAttacking(\"Raging Goblin\", false)"
        }
      ]
    }
  ]
});
