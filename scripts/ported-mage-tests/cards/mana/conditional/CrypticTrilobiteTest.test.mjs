import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/mana/conditional/CrypticTrilobiteTest.java",
  "tests": [
    {
      "name": "testAvailableManaCalculation",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Cryptic Trilobite",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 10
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Cryptic Trilobite"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "X=5"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Cryptic Trilobite",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "assertManaOptions(\"{C}{C}{C}{C}{C}{C}{C}{C}{C}{C}[{ActivatedAbilityManaCondition}]\", manaOptions)"
        }
      ]
    },
    {
      "name": "testUse",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Cryptic Trilobite",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Deathknell Kami",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 10
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Cryptic Trilobite"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "X=5"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{2}:"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{2}:"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Cryptic Trilobite",
          "count": 1
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Cryptic Trilobite",
          "counter": "P1P1",
          "count": 3
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Deathknell Kami",
          "power": 2,
          "toughness": 3
        },
        {
          "op": "unsupported",
          "source": "assertManaOptions(\"{C}{C}{C}{C}{C}{C}[{ActivatedAbilityManaCondition}]\", manaOptions)"
        }
      ]
    },
    {
      "name": "testCantUse",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Cryptic Trilobite",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Aegis Automaton",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 10
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Cryptic Trilobite"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "X=5"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "label": "Cast Aegis Automaton",
          "expected": false
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Cryptic Trilobite",
          "count": 1
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Cryptic Trilobite",
          "counter": "P1P1",
          "count": 5
        }
      ]
    }
  ]
});
