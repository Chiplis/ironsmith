import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/conditional/LifeCompareConditionTest.java",
  "tests": [
    {
      "name": "test10OrLess",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 7
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Vampire Lacerator",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Ruthless Cullblade",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Sorin's Vengeance",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Vampire Lacerator"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Vampire Lacerator"
        },
        {
          "op": "assertPowerToughness",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Ruthless Cullblade",
          "power": 2,
          "toughness": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "At the beginning"
        },
        {
          "op": "assertPowerToughness",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Ruthless Cullblade",
          "power": 2,
          "toughness": 1
        },
        {
          "op": "unsupported",
          "source": "checkLife(\"2 life lost\", 3, PhaseStep.PRECOMBAT_MAIN, playerA, 18)"
        },
        {
          "op": "unsupported",
          "source": "checkLife(\"no damage dealt\", 3, PhaseStep.PRECOMBAT_MAIN, playerB, 20)"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "At the beginning"
        },
        {
          "op": "castSpell",
          "turn": 5,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Sorin's Vengeance",
          "target": 1
        },
        {
          "op": "assertPowerToughness",
          "turn": 5,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Ruthless Cullblade",
          "power": 4,
          "toughness": 2
        },
        {
          "op": "unsupported",
          "source": "checkLife(\"4 life lost, 10 gained\", 5, PhaseStep.POSTCOMBAT_MAIN, playerA, 26)"
        },
        {
          "op": "unsupported",
          "source": "checkLife(\"10 damage dealt\", 5, PhaseStep.POSTCOMBAT_MAIN, playerB, 10)"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "At the beginning"
        },
        {
          "op": "setStopAt",
          "turn": 7,
          "phase": "END_TURN"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "execute"
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 26
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 10
        }
      ]
    },
    {
      "name": "test25orMore",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Lone Missionary",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Angel of Vitality",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Angel of Vitality",
          "power": 2,
          "toughness": 2
        },
        {
          "op": "castSpell",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Lone Missionary"
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "END_TURN"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "execute"
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 25
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 20
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Angel of Vitality",
          "power": 4,
          "toughness": 4
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Lone Missionary",
          "power": 2,
          "toughness": 1
        }
      ]
    }
  ]
});
