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
          "op": "activateManaAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "source": "Hall of the Bandit Lord",
          "ability": "Pay 3 life",
          "count": 1
        },
        {
          "op": "assertManaPool",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "colors": "C",
          "amount": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Multani, Maro-Sorcerer"
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Multani, Maro-Sorcerer",
          "count": 1
        },
        {
          "op": "assertManaPool",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "colors": "C",
          "amount": 0,
          "holdStack": true
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "resolutions": 1
        },
        {
          "op": "assertAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Multani, Maro-Sorcerer",
          "ability": "Haste",
          "expected": true
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Multani, Maro-Sorcerer",
          "defender": 1,
          "expectedLegal": true
        },
        {
          "op": "activateManaAbility",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "source": "Hall of the Bandit Lord",
          "ability": "Pay 3 life",
          "count": 1
        },
        {
          "op": "assertManaPool",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "colors": "C",
          "amount": 1
        },
        {
          "op": "castSpell",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Pathbreaker Ibex"
        },
        {
          "op": "assertStackObject",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "source": "Pathbreaker Ibex",
          "count": 1
        },
        {
          "op": "assertManaPool",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "colors": "C",
          "amount": 0,
          "holdStack": true
        },
        {
          "op": "waitStackResolved",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "resolutions": 1
        },
        {
          "op": "assertAbility",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Pathbreaker Ibex",
          "ability": "Haste",
          "expected": true
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Multani, Maro-Sorcerer",
          "defender": 1,
          "expectedLegal": true
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Pathbreaker Ibex",
          "defender": 1,
          "expectedLegal": true
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
          "op": "assertTapped",
          "name": "Hall of the Bandit Lord",
          "tapped": true,
          "player": 0
        },
        {
          "op": "assertTapped",
          "name": "Multani, Maro-Sorcerer",
          "tapped": true,
          "player": 0
        },
        {
          "op": "assertTapped",
          "name": "Pathbreaker Ibex",
          "tapped": true,
          "player": 0
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
