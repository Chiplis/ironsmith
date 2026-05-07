import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/ulg/MultaniTest.java",
  "tests": [
    {
      "name": "pathbreakerTrampleShouldOnlyLastUntilEOT",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Multani, Maro-Sorcerer",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Pathbreaker Ibex",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Hall of the Bandit Lord",
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
          "name": "Multani, Maro-Sorcerer"
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Multani, Maro-Sorcerer",
          "defender": 1
        },
        {
          "op": "castSpell",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Pathbreaker Ibex"
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Multani, Maro-Sorcerer",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Pathbreaker Ibex",
          "defender": 1
        },
        {
          "op": "setStopAt",
          "turn": 4,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 14
        },
        {
          "op": "unsupported",
          "source": "assertTapped(hBandit, true)"
        },
        {
          "op": "unsupported",
          "source": "assertTapped(multani, true)"
        },
        {
          "op": "unsupported",
          "source": "assertTapped(pIbex, true)"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Pathbreaker Ibex",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Multani, Maro-Sorcerer",
          "count": 1
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Pathbreaker Ibex",
          "ability": "Haste",
          "expected": true
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Multani, Maro-Sorcerer",
          "ability": "Haste",
          "expected": true
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Multani, Maro-Sorcerer",
          "ability": "Shroud",
          "expected": true
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Multani, Maro-Sorcerer",
          "ability": "Trample",
          "expected": false
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Pathbreaker Ibex",
          "ability": "Trample",
          "expected": false
        }
      ]
    }
  ]
});
