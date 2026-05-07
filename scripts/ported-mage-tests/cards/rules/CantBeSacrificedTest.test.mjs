import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/rules/CantBeSacrificedTest.java",
  "tests": [
    {
      "name": "testAssaultSuitWithSacEffect",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Walking Corpse",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Assault Suit",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Barony Vampire",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Innocent Blood",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "Equip",
          "target": "Walking Corpse"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Innocent Blood"
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": "Barony Vampire"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Walking Corpse",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Barony Vampire",
          "count": 1
        }
      ]
    },
    {
      "name": "testAssaultSuitWithSacSourceCost",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Bile Urchin",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Assault Suit",
          "count": 1
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "UPKEEP",
          "player": 0,
          "label": "Sacrifice ",
          "expected": true
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "Equip",
          "target": "Bile Urchin"
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "BEGIN_COMBAT",
          "player": 0,
          "label": "Sacrifice ",
          "expected": false
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        },
        {
          "op": "unsupported",
          "source": "assertAttachedTo(playerA, assaultSuit, urchin, true)"
        }
      ]
    },
    {
      "name": "testAssaultSuitWithSacAnotherCost",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Walking Corpse",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Blood Bairn",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Assault Suit",
          "count": 1
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "UPKEEP",
          "player": 0,
          "label": "Sacrifice another",
          "expected": true
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "Equip",
          "target": "Walking Corpse"
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "BEGIN_COMBAT",
          "player": 0,
          "label": "Sacrifice another",
          "expected": false
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        },
        {
          "op": "unsupported",
          "source": "assertAttachedTo(playerA, assaultSuit, zombie, true)"
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Blood Bairn",
          "power": 2,
          "toughness": 2
        }
      ]
    },
    {
      "name": "testJonIrenicusWithSacSourceCost",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Bile Urchin",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Jon Irenicus, Shattered One",
          "count": 1
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "UPKEEP",
          "player": 0,
          "label": "Sacrifice ",
          "expected": true
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "UPKEEP",
          "player": 1,
          "label": "Sacrifice ",
          "expected": false
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": 1
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Bile Urchin"
        },
        {
          "op": "assertPlayableAbility",
          "turn": 2,
          "phase": "UPKEEP",
          "player": 0,
          "label": "Sacrifice ",
          "expected": false
        },
        {
          "op": "assertPlayableAbility",
          "turn": 2,
          "phase": "UPKEEP",
          "player": 1,
          "label": "Sacrifice ",
          "expected": false
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Bile Urchin",
          "count": 1
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Bile Urchin",
          "counter": "P1P1",
          "count": 2
        }
      ]
    }
  ]
});
