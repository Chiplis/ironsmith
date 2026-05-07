import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/continuous/WardenOfTheFirstTreeTest.java",
  "tests": [
    {
      "name": "testFirstAbility",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 7
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Warden of the First Tree",
          "count": 2
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Warden of the First Tree"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{1}{W/B}:"
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
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Warden of the First Tree",
          "power": 3,
          "toughness": 3
        },
        {
          "op": "unsupported",
          "source": "assertType(\"Warden of the First Tree\", CardType.CREATURE, SubType.HUMAN)"
        },
        {
          "op": "unsupported",
          "source": "assertType(\"Warden of the First Tree\", CardType.CREATURE, SubType.WARRIOR)"
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Warden of the First Tree",
          "ability": "Trample",
          "expected": false
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Warden of the First Tree",
          "ability": "Lifelink",
          "expected": false
        }
      ]
    },
    {
      "name": "testSecondAbility",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 7
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Warden of the First Tree",
          "count": 2
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Warden of the First Tree"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{1}{W/B}:"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "ability": "{2}{W/B}{W/B}:"
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
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Warden of the First Tree",
          "power": 3,
          "toughness": 3
        },
        {
          "op": "unsupported",
          "source": "assertType(\"Warden of the First Tree\", CardType.CREATURE, SubType.HUMAN)"
        },
        {
          "op": "unsupported",
          "source": "assertType(\"Warden of the First Tree\", CardType.CREATURE, SubType.SPIRIT)"
        },
        {
          "op": "unsupported",
          "source": "assertType(\"Warden of the First Tree\", CardType.CREATURE, SubType.WARRIOR)"
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Warden of the First Tree",
          "ability": "Trample",
          "expected": true
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Warden of the First Tree",
          "ability": "Lifelink",
          "expected": true
        }
      ]
    },
    {
      "name": "testThirdAbility",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 7
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Warden of the First Tree",
          "count": 2
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Warden of the First Tree"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{1}{W/B}:"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "ability": "{2}{W/B}{W/B}:"
        },
        {
          "op": "activateAbility",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{3}{W/B}{W/B}{W/B}:"
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Warden of the First Tree",
          "power": 8,
          "toughness": 8
        },
        {
          "op": "unsupported",
          "source": "assertType(\"Warden of the First Tree\", CardType.CREATURE, SubType.HUMAN)"
        },
        {
          "op": "unsupported",
          "source": "assertType(\"Warden of the First Tree\", CardType.CREATURE, SubType.SPIRIT)"
        },
        {
          "op": "unsupported",
          "source": "assertType(\"Warden of the First Tree\", CardType.CREATURE, SubType.WARRIOR)"
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Warden of the First Tree",
          "ability": "Trample",
          "expected": true
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Warden of the First Tree",
          "ability": "Lifelink",
          "expected": true
        }
      ]
    },
    {
      "name": "testTwoWarden",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 7
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Warden of the First Tree",
          "count": 2
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Warden of the First Tree"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{1}{W/B}:"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "ability": "{2}{W/B}{W/B}:"
        },
        {
          "op": "castSpell",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Warden of the First Tree"
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Warden of the First Tree",
          "power": 1,
          "toughness": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Warden of the First Tree",
          "power": 3,
          "toughness": 3
        }
      ]
    }
  ]
});
