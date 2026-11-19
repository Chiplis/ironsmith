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
          "op": "assertCounterCount",
          "player": 0,
          "name": "Cryptic Trilobite",
          "counter": "P1P1",
          "count": 5
        },
        {
          "op": "assertManaOptions",
          "player": 0,
          "exact": true,
          "options": [
            {
              "mana": "{C}{C}{C}{C}{C}{C}{C}{C}{C}{C}",
              "restricted": {
                "activateAbilities": 10
              }
            }
          ]
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
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Cryptic Trilobite",
          "count": 1
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "resolutions": 1
        },
        {
          "op": "assertCounterCount",
          "name": "Cryptic Trilobite",
          "count": 5,
          "player": 0,
          "counter": "P1P1",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "seatIndependent": true
        },
        {
          "op": "activateManaAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "source": "Cryptic Trilobite",
          "ability": "Remove a +1/+1 counter"
        },
        {
          "op": "assertManaPool",
          "player": 0,
          "colors": "C",
          "amount": 2,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{2}:",
          "source": "Deathknell Kami"
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Deathknell Kami",
          "count": 1,
          "abilityKind": "Activated"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "resolutions": 1
        },
        {
          "op": "assertManaPool",
          "player": 0,
          "colors": "C",
          "amount": 0,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "activateManaAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "source": "Cryptic Trilobite",
          "ability": "Remove a +1/+1 counter"
        },
        {
          "op": "assertManaPool",
          "player": 0,
          "colors": "C",
          "amount": 2,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{2}:",
          "source": "Deathknell Kami"
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Deathknell Kami",
          "count": 1,
          "abilityKind": "Activated"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "resolutions": 1
        },
        {
          "op": "assertManaPool",
          "player": 0,
          "colors": "C",
          "amount": 0,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
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
          "count": 3,
          "seatIndependent": true
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Deathknell Kami",
          "power": 2,
          "toughness": 3
        },
        {
          "op": "assertManaOptions",
          "player": 0,
          "options": [
            {
              "mana": "{C}{C}{C}{C}{C}{C}",
              "restricted": {
                "activateAbilities": 6
              }
            }
          ],
          "exact": true
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
          "phase": "PRECOMBAT_MAIN"
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
