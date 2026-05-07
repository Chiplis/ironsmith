import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/mh3/ArenaOfGloryTest.java",
  "tests": [
    {
      "name": "test_NormalManaNoHaste",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Dwarven Trader",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Arena of Glory",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Dwarven Trader"
        },
        {
          "op": "assertAbility",
          "player": "Dwarven Trader doesn't have haste",
          "name": 3,
          "ability": "PRECOMBAT_MAIN",
          "expected": 0
        },
        {
          "op": "setStopAt",
          "turn": 5,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "unsupported",
          "source": "assertTapped(arena, false)"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Dwarven Trader",
          "count": 1
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Dwarven Trader",
          "ability": "Haste",
          "expected": false
        }
      ]
    },
    {
      "name": "test_TwoCreatureHaste",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Dwarven Trader",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Mons's Goblin Raiders",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Arena of Glory",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 1
        },
        {
          "op": "activateManaAbility",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{R}",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Dwarven Trader"
        },
        {
          "op": "castSpell",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Mons's Goblin Raiders"
        },
        {
          "op": "assertAbility",
          "player": "Dwarven Trader has haste",
          "name": 3,
          "ability": "PRECOMBAT_MAIN",
          "expected": 0
        },
        {
          "op": "assertAbility",
          "player": "Mons's Goblin Raiders has haste",
          "name": 3,
          "ability": "PRECOMBAT_MAIN",
          "expected": 0
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Dwarven Trader",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Mons's Goblin Raiders",
          "defender": 1
        },
        {
          "op": "setStopAt",
          "turn": 5,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "unsupported",
          "source": "assertTapped(arena, true)"
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 18
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Dwarven Trader",
          "count": 1
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Dwarven Trader",
          "ability": "Haste",
          "expected": false
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Mons's Goblin Raiders",
          "count": 1
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Mons's Goblin Raiders",
          "ability": "Haste",
          "expected": false
        }
      ]
    }
  ]
});
