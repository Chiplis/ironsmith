import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/blc/RapidAugmenterTest.java",
  "tests": [
    {
      "name": "test_Cast21",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Rapid Augmenter",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Air Marshal",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Alpine Watchdog",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Air Marshal"
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Rapid Augmenter",
          "defender": 1
        },
        {
          "op": "block",
          "turn": 1,
          "player": 1,
          "blocker": "Alpine Watchdog",
          "attacker": "Rapid Augmenter"
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
          "op": "assertAbility",
          "player": 0,
          "name": "Air Marshal",
          "ability": "Haste",
          "expected": false
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Rapid Augmenter",
          "power": 1,
          "toughness": 3
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Rapid Augmenter",
          "counter": "P1P1",
          "count": 0
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": "currentGame.getStartingLife()"
        }
      ]
    },
    {
      "name": "test_Cast11",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Rapid Augmenter",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Memnite",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Alpine Watchdog",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Memnite"
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Rapid Augmenter",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Memnite",
          "defender": 1
        },
        {
          "op": "block",
          "turn": 1,
          "player": 1,
          "blocker": "Alpine Watchdog",
          "attacker": "Rapid Augmenter"
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
          "op": "assertAbility",
          "player": 0,
          "name": "Memnite",
          "ability": "Haste",
          "expected": true
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Rapid Augmenter",
          "power": 1,
          "toughness": 3
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Rapid Augmenter",
          "counter": "P1P1",
          "count": 0
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": "currentGame.getStartingLife() - 1"
        }
      ]
    },
    {
      "name": "test_Bounce21",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Rapid Augmenter",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Air Marshal",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Ephemerate",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Alpine Watchdog",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Ephemerate"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Air Marshal"
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Rapid Augmenter",
          "defender": 1
        },
        {
          "op": "block",
          "turn": 1,
          "player": 1,
          "blocker": "Alpine Watchdog",
          "attacker": "Rapid Augmenter"
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
          "op": "assertAbility",
          "player": 0,
          "name": "Air Marshal",
          "ability": "Haste",
          "expected": false
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Rapid Augmenter",
          "power": 2,
          "toughness": 4
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Rapid Augmenter",
          "counter": "P1P1",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Alpine Watchdog",
          "count": 1
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": "currentGame.getStartingLife() - 2"
        }
      ]
    },
    {
      "name": "test_Bounce11",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Rapid Augmenter",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Memnite",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Ephemerate",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Alpine Watchdog",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Ephemerate"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Memnite"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Whenever another creature you control enters"
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Rapid Augmenter",
          "defender": 1
        },
        {
          "op": "block",
          "turn": 1,
          "player": 1,
          "blocker": "Alpine Watchdog",
          "attacker": "Rapid Augmenter"
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
          "op": "assertAbility",
          "player": 0,
          "name": "Memnite",
          "ability": "Haste",
          "expected": true
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Rapid Augmenter",
          "power": 2,
          "toughness": 4
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Rapid Augmenter",
          "counter": "P1P1",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Alpine Watchdog",
          "count": 1
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": "currentGame.getStartingLife() - 2"
        }
      ]
    }
  ]
});
