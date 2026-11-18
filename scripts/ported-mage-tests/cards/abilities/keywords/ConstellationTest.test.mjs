import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/abilities/keywords/ConstellationTest.java",
  "tests": [
    {
      "name": "test_DaxosGotBoostOnEnter",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Daxos's Torment",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 4
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Daxos's Torment"
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
          "op": "assertHandCount",
          "player": 0,
          "name": "Daxos's Torment",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Daxos's Torment",
          "count": 1
        },
        {
          "op": "sequence",
          "operations": [
            {
              "op": "assertPowerToughness",
              "player": 0,
              "name": "Daxos's Torment",
              "power": 5,
              "toughness": 5
            },
            {
              "op": "unsupported",
              "source": "assertType(daxosCard, CardType.CREATURE, SubType.DEMON)"
            },
            {
              "op": "assertAbility",
              "player": 0,
              "name": "Daxos's Torment",
              "ability": "Flying",
              "expected": true
            },
            {
              "op": "assertAbility",
              "player": 0,
              "name": "Daxos's Torment",
              "ability": "Haste",
              "expected": true
            }
          ]
        }
      ]
    },
    {
      "name": "test_DaxosLostBoostOnNextTurn",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Daxos's Torment",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 4
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Daxos's Torment"
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "Daxos's Torment",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Daxos's Torment",
          "count": 1
        },
        {
          "op": "sequence",
          "operations": [
            {
              "op": "assertPowerToughness",
              "player": 0,
              "name": "Daxos's Torment",
              "power": 0,
              "toughness": 0
            },
            {
              "op": "unsupported",
              "source": "assertNotSubtype(daxosCard, SubType.DEMON)"
            },
            {
              "op": "assertAbility",
              "player": 0,
              "name": "Daxos's Torment",
              "ability": "Flying",
              "expected": false
            },
            {
              "op": "assertAbility",
              "player": 0,
              "name": "Daxos's Torment",
              "ability": "Haste",
              "expected": false
            }
          ]
        }
      ]
    },
    {
      "name": "test_DaxosGotBoostOnOtherEnchantment",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Daxos's Torment",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Absolute Grace",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 2
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Daxos's Torment"
        },
        {
          "op": "castSpell",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Absolute Grace"
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
          "op": "assertHandCount",
          "player": 0,
          "name": "Absolute Grace",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Absolute Grace",
          "count": 1
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "Daxos's Torment",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Daxos's Torment",
          "count": 1
        },
        {
          "op": "sequence",
          "operations": [
            {
              "op": "assertPowerToughness",
              "player": 0,
              "name": "Daxos's Torment",
              "power": 5,
              "toughness": 5
            },
            {
              "op": "unsupported",
              "source": "assertType(daxosCard, CardType.CREATURE, SubType.DEMON)"
            },
            {
              "op": "assertAbility",
              "player": 0,
              "name": "Daxos's Torment",
              "ability": "Flying",
              "expected": true
            },
            {
              "op": "assertAbility",
              "player": 0,
              "name": "Daxos's Torment",
              "ability": "Haste",
              "expected": true
            }
          ]
        }
      ]
    },
    {
      "name": "test_DaxosGotBoostAndWithPTLose",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Daxos's Torment",
          "count": 1
        },
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
          "name": "Night of Souls' Betrayal",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Daxos's Torment"
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
          "op": "assertHandCount",
          "player": 0,
          "name": "Daxos's Torment",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Daxos's Torment",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Daxos's Torment",
          "power": 4,
          "toughness": 4
        },
        {
          "op": "unsupported",
          "source": "assertType(daxosCard, CardType.CREATURE, SubType.DEMON)"
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Daxos's Torment",
          "ability": "Flying",
          "expected": true
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Daxos's Torment",
          "ability": "Haste",
          "expected": true
        }
      ]
    },
    {
      "name": "test_DaxosGotBoostWithLoseFly",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Daxos's Torment",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Invert the Skies",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Forest",
          "count": 4
        },
        {
          "op": "assertHandCount",
          "player": 1,
          "name": "PRECOMBAT_MAIN",
          "count": 0
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Daxos's Torment"
        },
        {
          "op": "assertHandCount",
          "player": 1,
          "name": "POSTCOMBAT_MAIN",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Daxos's Torment",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Daxos's Torment",
          "power": 5,
          "toughness": 5
        },
        {
          "op": "assertAbility",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Daxos's Torment",
          "ability": "FlyingAbility.class",
          "expected": true
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 1,
          "name": "Invert the Skies"
        },
        {
          "op": "assertPowerToughness",
          "turn": 1,
          "phase": "END_TURN",
          "player": 0,
          "name": "Daxos's Torment",
          "power": 5,
          "toughness": 5
        },
        {
          "op": "assertAbility",
          "turn": 1,
          "phase": "END_TURN",
          "player": 0,
          "name": "Daxos's Torment",
          "ability": "FlyingAbility.class",
          "expected": false
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        }
      ]
    },
    {
      "name": "test_DaxosGotBoostWithLoseFlyAndGotItAgain",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Daxos's Torment",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Gravity Sphere",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 6
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Daxos's Torment"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Gravity Sphere"
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
          "op": "assertHandCount",
          "player": 0,
          "name": "Daxos's Torment",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Daxos's Torment",
          "count": 1
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "Gravity Sphere",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Gravity Sphere",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Daxos's Torment",
          "power": 5,
          "toughness": 5
        },
        {
          "op": "unsupported",
          "source": "assertType(daxosCard, CardType.CREATURE, SubType.DEMON)"
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Daxos's Torment",
          "ability": "Flying",
          "expected": true
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Daxos's Torment",
          "ability": "Haste",
          "expected": true
        }
      ]
    },
    {
      "name": "test_DaxosGotBoostAndSaveColor",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Daxos's Torment",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Chaoslace",
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
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Archetype of Courage",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 4
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Daxos's Torment"
        },
        {
          "op": "assertPermanentCount",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Daxos's Torment",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "checkColor(\"dax without color\", 1, PhaseStep.POSTCOMBAT_MAIN, playerA, daxosCard, \"R\", false)"
        },
        {
          "op": "castSpell",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Chaoslace",
          "target": "Daxos's Torment"
        },
        {
          "op": "assertPowerToughness",
          "turn": 3,
          "phase": "BEGIN_COMBAT",
          "player": 0,
          "name": "Daxos's Torment",
          "power": 0,
          "toughness": 0
        },
        {
          "op": "unsupported",
          "source": "checkColor(\"dax is red\", 3, PhaseStep.BEGIN_COMBAT, playerA, daxosCard, \"R\", true)"
        },
        {
          "op": "castSpell",
          "turn": 5,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Archetype of Courage"
        },
        {
          "op": "assertPowerToughness",
          "turn": 5,
          "phase": "BEGIN_COMBAT",
          "player": 0,
          "name": "Daxos's Torment",
          "power": 5,
          "toughness": 5
        },
        {
          "op": "assertAbility",
          "turn": 5,
          "phase": "BEGIN_COMBAT",
          "player": 0,
          "name": "Daxos's Torment",
          "ability": "FlyingAbility.class",
          "expected": true
        },
        {
          "op": "unsupported",
          "source": "checkColor(\"dax is red\", 5, PhaseStep.BEGIN_COMBAT, playerA, daxosCard, \"R\", true)"
        },
        {
          "op": "setStopAt",
          "turn": 5,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        }
      ]
    },
    {
      "name": "test_DaxosGotBoostAndNewTypeByDependencyEffects_RegularWay",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Daxos's Torment",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Mephidross Vampire",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 8
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Archetype of Courage",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 2
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Daxos's Torment"
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "count": 1,
          "source": "Daxos's Torment"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "resolutions": 1
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "count": 1,
          "source": "Daxos's Torment",
          "abilityKind": "Triggered"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "resolutions": 1
        },
        {
          "op": "assertPermanentCount",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Daxos's Torment",
          "count": 1,
          "seatIndependent": true
        },
        {
          "op": "assertType",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Daxos's Torment",
          "cardType": "CREATURE",
          "extra": "false",
          "seatIndependent": true
        },
        {
          "op": "assertAbility",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Daxos's Torment",
          "ability": "Flying",
          "expected": false,
          "seatIndependent": true
        },
        {
          "op": "assertManaPool",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "colors": "B",
          "amount": 0
        },
        {
          "op": "activateManaAbility",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "source": "Swamp",
          "ability": "{T}: Add {B}",
          "count": 6
        },
        {
          "op": "assertManaPool",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "colors": "B",
          "amount": 6
        },
        {
          "op": "castSpell",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Mephidross Vampire"
        },
        {
          "op": "assertStackObject",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "count": 1,
          "source": "Mephidross Vampire"
        },
        {
          "op": "waitStackResolved",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "resolutions": 1
        },
        {
          "op": "assertPermanentCount",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Mephidross Vampire",
          "count": 1,
          "seatIndependent": true
        },
        {
          "op": "assertTappedCount",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Plains",
          "tapped": true,
          "count": 0,
          "seatIndependent": true
        },
        {
          "op": "assertType",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Daxos's Torment",
          "cardType": "CREATURE",
          "extra": "false",
          "seatIndependent": true
        },
        {
          "op": "castSpell",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Archetype of Courage"
        },
        {
          "op": "assertStackObject",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "count": 1,
          "source": "Archetype of Courage"
        },
        {
          "op": "waitStackResolved",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "resolutions": 1
        },
        {
          "op": "assertStackObject",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "count": 1,
          "source": "Daxos's Torment",
          "abilityKind": "Triggered"
        },
        {
          "op": "waitStackResolved",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "resolutions": 1
        },
        {
          "op": "assertPermanentCount",
          "turn": 3,
          "phase": "BEGIN_COMBAT",
          "player": 0,
          "name": "Archetype of Courage",
          "count": 1,
          "seatIndependent": true
        },
        {
          "op": "assertPowerToughness",
          "turn": 3,
          "phase": "BEGIN_COMBAT",
          "player": 0,
          "name": "Daxos's Torment",
          "power": 5,
          "toughness": 5,
          "seatIndependent": true
        },
        {
          "op": "assertType",
          "turn": 3,
          "phase": "BEGIN_COMBAT",
          "player": 0,
          "name": "Daxos's Torment",
          "cardType": "ENCHANTMENT",
          "extra": "true",
          "seatIndependent": true
        },
        {
          "op": "assertType",
          "turn": 3,
          "phase": "BEGIN_COMBAT",
          "player": 0,
          "name": "Daxos's Torment",
          "cardType": "CREATURE",
          "extra": "true",
          "seatIndependent": true
        },
        {
          "op": "assertSubtype",
          "turn": 3,
          "phase": "BEGIN_COMBAT",
          "player": 0,
          "name": "Daxos's Torment",
          "subtype": "DEMON",
          "seatIndependent": true
        },
        {
          "op": "assertAbility",
          "turn": 3,
          "phase": "BEGIN_COMBAT",
          "player": 0,
          "name": "Daxos's Torment",
          "ability": "Flying",
          "expected": true,
          "seatIndependent": true
        },
        {
          "op": "assertAbility",
          "turn": 3,
          "phase": "BEGIN_COMBAT",
          "player": 0,
          "name": "Daxos's Torment",
          "ability": "Haste",
          "expected": true,
          "seatIndependent": true
        },
        {
          "op": "assertSubtype",
          "turn": 3,
          "phase": "BEGIN_COMBAT",
          "player": 0,
          "name": "Daxos's Torment",
          "subtype": "VAMPIRE",
          "seatIndependent": true
        }
      ]
    },
    {
      "name": "test_DaxosGotBoostAndNewTypeByDependencyEffects_DifferentWay",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Daxos's Torment",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Mephidross Vampire",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 8
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Archetype of Courage",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 2
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Daxos's Torment"
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "count": 1,
          "source": "Daxos's Torment"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "resolutions": 1
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "count": 1,
          "source": "Daxos's Torment",
          "abilityKind": "Triggered"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "resolutions": 1
        },
        {
          "op": "assertPermanentCount",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Daxos's Torment",
          "count": 1,
          "seatIndependent": true
        },
        {
          "op": "assertType",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Daxos's Torment",
          "cardType": "CREATURE",
          "extra": "false",
          "seatIndependent": true
        },
        {
          "op": "assertAbility",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Daxos's Torment",
          "ability": "Flying",
          "expected": false,
          "seatIndependent": true
        },
        {
          "op": "castSpell",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Archetype of Courage"
        },
        {
          "op": "assertStackObject",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "count": 1,
          "source": "Archetype of Courage"
        },
        {
          "op": "waitStackResolved",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "resolutions": 1
        },
        {
          "op": "assertStackObject",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "count": 1,
          "source": "Daxos's Torment",
          "abilityKind": "Triggered"
        },
        {
          "op": "waitStackResolved",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "resolutions": 1
        },
        {
          "op": "assertPowerToughness",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Daxos's Torment",
          "power": 5,
          "toughness": 5,
          "seatIndependent": true
        },
        {
          "op": "castSpell",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Mephidross Vampire"
        },
        {
          "op": "assertStackObject",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "count": 1,
          "source": "Mephidross Vampire"
        },
        {
          "op": "waitStackResolved",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "resolutions": 1
        },
        {
          "op": "assertPermanentCount",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Mephidross Vampire",
          "count": 1,
          "seatIndependent": true
        },
        {
          "op": "assertPermanentCount",
          "turn": 3,
          "phase": "BEGIN_COMBAT",
          "player": 0,
          "name": "Archetype of Courage",
          "count": 1,
          "seatIndependent": true
        },
        {
          "op": "assertPowerToughness",
          "turn": 3,
          "phase": "BEGIN_COMBAT",
          "player": 0,
          "name": "Daxos's Torment",
          "power": 5,
          "toughness": 5,
          "seatIndependent": true
        },
        {
          "op": "assertType",
          "turn": 3,
          "phase": "BEGIN_COMBAT",
          "player": 0,
          "name": "Daxos's Torment",
          "cardType": "ENCHANTMENT",
          "extra": "true",
          "seatIndependent": true
        },
        {
          "op": "assertType",
          "turn": 3,
          "phase": "BEGIN_COMBAT",
          "player": 0,
          "name": "Daxos's Torment",
          "cardType": "CREATURE",
          "extra": "true",
          "seatIndependent": true
        },
        {
          "op": "assertSubtype",
          "turn": 3,
          "phase": "BEGIN_COMBAT",
          "player": 0,
          "name": "Daxos's Torment",
          "subtype": "DEMON",
          "seatIndependent": true
        },
        {
          "op": "assertAbility",
          "turn": 3,
          "phase": "BEGIN_COMBAT",
          "player": 0,
          "name": "Daxos's Torment",
          "ability": "Flying",
          "expected": true,
          "seatIndependent": true
        },
        {
          "op": "assertAbility",
          "turn": 3,
          "phase": "BEGIN_COMBAT",
          "player": 0,
          "name": "Daxos's Torment",
          "ability": "Haste",
          "expected": true,
          "seatIndependent": true
        },
        {
          "op": "assertSubtype",
          "turn": 3,
          "phase": "BEGIN_COMBAT",
          "player": 0,
          "name": "Daxos's Torment",
          "subtype": "VAMPIRE",
          "seatIndependent": true
        }
      ]
    }
  ]
});
