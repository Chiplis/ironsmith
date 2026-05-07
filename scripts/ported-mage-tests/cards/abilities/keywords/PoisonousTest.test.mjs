import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/abilities/keywords/PoisonousTest.java",
  "tests": [
    {
      "name": "testNormalCombatDamageIsDealt",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Virulent Sliver",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Virulent Sliver",
          "defender": 1
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "END_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 20
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 19
        }
      ]
    },
    {
      "name": "testCombatDamageAddsPoisonCounters",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Virulent Sliver",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Virulent Sliver",
          "defender": 1
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "END_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": 0,
          "counter": "POISON",
          "count": 0
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": 1,
          "counter": "POISON",
          "count": 1
        }
      ]
    },
    {
      "name": "testMultipleInstancesAddMultipleCounters",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Virulent Sliver",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Manaweft Sliver",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Manaweft Sliver",
          "defender": 1
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "END_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "unsupported",
          "source": "assertMultipleInstancesOfAbility(playerA, \"Manaweft Sliver\", new PoisonousAbility(1), 2)"
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": 1,
          "counter": "POISON",
          "count": 2
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 19
        }
      ]
    },
    {
      "name": "testNumberOfCountersNotTiedToDamageDealt",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Virulent Sliver",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Battle Sliver",
          "count": 2
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Virulent Sliver",
          "defender": 1
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "END_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": 1,
          "counter": "POISON",
          "count": 1
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 15
        }
      ]
    },
    {
      "name": "testBlockedDamageDoesntAddPoisonCounters",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Virulent Sliver",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Wall of Frost",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Virulent Sliver",
          "defender": 1
        },
        {
          "op": "block",
          "turn": 3,
          "player": 1,
          "blocker": "Wall of Frost",
          "attacker": "Virulent Sliver"
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "END_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": 1,
          "counter": "POISON",
          "count": 0
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 20
        }
      ]
    },
    {
      "name": "testPreventedCombatDamageDoesntAddPoisonCounters",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Virulent Sliver",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Fog",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Forest",
          "count": 10
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Virulent Sliver",
          "defender": 1
        },
        {
          "op": "castSpell",
          "turn": 3,
          "phase": "DECLARE_BLOCKERS",
          "player": 1,
          "name": "Fog"
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "END_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": 1,
          "counter": "POISON",
          "count": 0
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 20
        }
      ]
    },
    {
      "name": "testZeroCombatDamageDoesntAddPoisonCounters",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Virulent Sliver",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Hydrosurge",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Island",
          "count": 10
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Virulent Sliver",
          "defender": 1
        },
        {
          "op": "castSpell",
          "turn": 3,
          "phase": "DECLARE_BLOCKERS",
          "player": 1,
          "name": "Hydrosurge",
          "target": "Virulent Sliver"
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "END_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Virulent Sliver",
          "power": -4,
          "toughness": 1
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": 1,
          "counter": "POISON",
          "count": 0
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 20
        }
      ]
    }
  ]
});
