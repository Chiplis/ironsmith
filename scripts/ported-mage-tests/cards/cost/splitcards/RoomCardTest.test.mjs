import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/cost/splitcards/RoomCardTest.java",
  "tests": [
    {
      "name": "testCardViewLeftHalf",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "\"Bottomless Pool \n    private static final String bottomlessPool = \"Bottomless Pool\"",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "bottomlessPool"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "TestPlayer.TARGET_SKIP"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "execute"
        }
      ]
    },
    {
      "name": "testCardViewRightHalf",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "\"Bottomless Pool \n    private static final String bottomlessPool = \"Bottomless Pool\"",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 5
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Locker Room"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "execute"
        }
      ]
    },
    {
      "name": "testCardViewBothHalves",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "\"Bottomless Pool \n    private static final String bottomlessPool = \"Bottomless Pool\"",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 6
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "bottomlessPool"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "TestPlayer.TARGET_SKIP"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{4}{U}: Unlock the right half."
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "execute"
        }
      ]
    },
    {
      "name": "testCardViewFullyLocked",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "\"Bottomless Pool \n    private static final String bottomlessPool = \"Bottomless Pool\"",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 6
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "execute"
        }
      ]
    },
    {
      "name": "testBottomlessPoolETB",
      "operations": [
        {
          "op": "skipInitShuffling"
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "\"Bottomless Pool \n    private static final String bottomlessPool = \"Bottomless Pool\"",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Grizzly Bears",
          "count": 2
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "label": "Cast Bottomless Pool",
          "expected": true
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "bottomlessPool"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Grizzly Bears"
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
          "player": 1,
          "name": "Grizzly Bears",
          "count": 1
        },
        {
          "op": "assertHandCount",
          "player": 1,
          "name": "Grizzly Bears",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "bottomlessPool",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "assertType(bottomlessPool, CardType.ENCHANTMENT, true)"
        },
        {
          "op": "unsupported",
          "source": "assertSubtype(bottomlessPool, SubType.ROOM)"
        }
      ]
    },
    {
      "name": "testLockerRoomCombatDamageTrigger",
      "operations": [
        {
          "op": "skipInitShuffling"
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "\"Bottomless Pool \n    private static final String bottomlessPool = \"Bottomless Pool\"",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 5
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Plains",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Memnite",
          "count": 2
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Locker Room"
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Memnite",
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
          "op": "assertStackSize",
          "turn": 1,
          "phase": "COMBAT_DAMAGE",
          "player": 0,
          "count": 2
        },
        {
          "op": "unsupported",
          "source": "checkStackObject(\"Locker Room trigger must be correct\", 1, PhaseStep.COMBAT_DAMAGE, playerA, \"Whenever a creature you control deals combat damage to an opponent, draw a card.\", 2)"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "END_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Locker Room",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "assertType(lockerRoom, CardType.ENCHANTMENT, true)"
        },
        {
          "op": "unsupported",
          "source": "assertSubtype(lockerRoom, SubType.ROOM)"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Memnite",
          "count": 2
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "execute"
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "Plains",
          "count": 2
        }
      ]
    },
    {
      "name": "testBottomlessPoolUnlock",
      "operations": [
        {
          "op": "skipInitShuffling"
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "\"Bottomless Pool \n    private static final String bottomlessPool = \"Bottomless Pool\"",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 6
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Memnite",
          "count": 2
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Locker Room"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "label": "{U}: Unlock the left half.",
          "expected": true
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{U}: Unlock the left half."
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Memnite"
        },
        {
          "op": "setStopAt",
          "turn": 1,
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
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Memnite",
          "count": 1
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "Memnite",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "\"Bottomless Pool \n    private static final String bottomlessPool = \"Bottomless Pool\"",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "assertType(bottomlessPoolLockerRoom, CardType.ENCHANTMENT, true)"
        },
        {
          "op": "unsupported",
          "source": "assertSubtype(bottomlessPoolLockerRoom, SubType.ROOM)"
        }
      ]
    },
    {
      "name": "testFlickerNameAndManaCost",
      "operations": [
        {
          "op": "skipInitShuffling"
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "\"Bottomless Pool \n    private static final String bottomlessPool = \"Bottomless Pool\"",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Felidar Guardian",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
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
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Memnite",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "bottomlessPool"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Memnite"
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
          "name": "Felidar Guardian"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Yes"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "bottomlessPool"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "setStopAt",
          "turn": 1,
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
          "op": "assertHandCount",
          "player": 0,
          "name": "Memnite",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "EmptyNames.FULLY_LOCKED_ROOM.getTestCommand()",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Felidar Guardian",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "assertType(EmptyNames.FULLY_LOCKED_ROOM.getTestCommand(), CardType.ENCHANTMENT, true)"
        },
        {
          "op": "unsupported",
          "source": "assertSubtype(EmptyNames.FULLY_LOCKED_ROOM.getTestCommand(), SubType.ROOM)"
        }
      ]
    },
    {
      "name": "testFlickerCanBeUnlockedAgain",
      "operations": [
        {
          "op": "skipInitShuffling"
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "\"Bottomless Pool \n    private static final String bottomlessPool = \"Bottomless Pool\"",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Felidar Guardian",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 5
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
          "player": 0,
          "name": "Memnite",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Black Knight",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "bottomlessPool"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Memnite"
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
          "name": "Felidar Guardian"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Yes"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "bottomlessPool"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "label": "{U}: Unlock the left half.",
          "expected": true
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{U}: Unlock the left half."
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Black Knight"
        },
        {
          "op": "setStopAt",
          "turn": 1,
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
          "op": "assertHandCount",
          "player": 0,
          "name": "Memnite",
          "count": 1
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "Black Knight",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "bottomlessPool",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Felidar Guardian",
          "count": 1
        }
      ]
    },
    {
      "name": "testEerie",
      "operations": [
        {
          "op": "skipInitShuffling"
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "\"Bottomless Pool \n    private static final String bottomlessPool = \"Bottomless Pool\"",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 6
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Erratic Apparition",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "bottomlessPool"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "When you unlock"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "TestPlayer.TARGET_SKIP"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{4}{U}: Unlock the right half."
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "setStopAt",
          "turn": 1,
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
          "op": "assertPermanentCount",
          "player": 0,
          "name": "\"Bottomless Pool \n    private static final String bottomlessPool = \"Bottomless Pool\"",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Erratic Apparition",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Erratic Apparition",
          "power": 3,
          "toughness": 5
        }
      ]
    },
    {
      "name": "testCopyOnStack",
      "operations": [
        {
          "op": "skipInitShuffling"
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "\"Bottomless Pool \n    private static final String bottomlessPool = \"Bottomless Pool\"",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "See Double",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 5
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
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Ornithopter",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "bottomlessPool"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "See Double"
        },
        {
          "op": "setModeChoice",
          "player": 0,
          "value": "1"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "bottomlessPool"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 3
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Memnite"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Ornithopter"
        },
        {
          "op": "setStopAt",
          "turn": 1,
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
          "op": "assertHandCount",
          "player": 0,
          "name": "Memnite",
          "count": 1
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "Ornithopter",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "bottomlessPool",
          "count": 2
        }
      ]
    },
    {
      "name": "testCopyOnBattlefield",
      "operations": [
        {
          "op": "skipInitShuffling"
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "\"Bottomless Pool \n    private static final String bottomlessPool = \"Bottomless Pool\"",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Clever Impersonator",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 6
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
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Ornithopter",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "bottomlessPool"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Memnite"
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
          "name": "Clever Impersonator"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Yes"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "bottomlessPool"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{U}: Unlock the left half."
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Ornithopter"
        },
        {
          "op": "setStopAt",
          "turn": 1,
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
          "op": "assertHandCount",
          "player": 0,
          "name": "Memnite",
          "count": 1
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "Ornithopter",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "bottomlessPool",
          "count": 2
        }
      ]
    },
    {
      "name": "testNameMatchOnStack",
      "operations": [
        {
          "op": "skipInitShuffling"
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "\"Bottomless Pool \n    private static final String bottomlessPool = \"Bottomless Pool\"",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Twiddle",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mindreaver",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 6
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "\"Bottomless Pool \n    private static final String bottomlessPool = \"Bottomless Pool\"",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Plains",
          "count": 2
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Twiddle"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Mindreaver"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "No"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": 0
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
          "name": "bottomlessPool"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{U}{U}, Sacrifice {this}:"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "bottomlessPool"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "END_TURN"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "execute"
        }
      ]
    },
    {
      "name": "testNameMatchOnFieldFromLocked",
      "operations": [
        {
          "op": "skipInitShuffling"
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "\"Bottomless Pool \n    private static final String bottomlessPool = \"Bottomless Pool\"",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Cackling Counterpart",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Bile Blight",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Underground Sea",
          "count": 17
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Glorious Anthem",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Opalescence",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "bottomlessPool"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "TestPlayer.TARGET_SKIP"
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
          "name": "Locker Room"
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
          "name": "bottomlessPool"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "TestPlayer.TARGET_SKIP"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{4}{U}: Unlock the right half."
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
          "name": "Cackling Counterpart"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "\"Bottomless Pool \n    private static final String bottomlessPool = \"Bottomless Pool\""
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
          "name": "Bile Blight"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "EmptyNames.FULLY_LOCKED_ROOM.getTestCommand()"
        },
        {
          "op": "setStopAt",
          "turn": 1,
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
          "op": "assertPermanentCount",
          "player": 0,
          "name": "EmptyNames.FULLY_LOCKED_ROOM.getTestCommand()",
          "count": 0
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "\"Bottomless Pool \n    private static final String bottomlessPool = \"Bottomless Pool\"",
          "count": 0
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "bottomlessPool",
          "power": 2,
          "toughness": 2
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Locker Room",
          "power": 6,
          "toughness": 6
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "\"Bottomless Pool \n    private static final String bottomlessPool = \"Bottomless Pool\"",
          "power": 7,
          "toughness": 7
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "bottomlessPool",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Locker Room",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "\"Bottomless Pool \n    private static final String bottomlessPool = \"Bottomless Pool\"",
          "count": 1
        }
      ]
    },
    {
      "name": "testNameMatchOnFieldFromHalf",
      "operations": [
        {
          "op": "skipInitShuffling"
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "\"Bottomless Pool \n    private static final String bottomlessPool = \"Bottomless Pool\"",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Cackling Counterpart",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Bile Blight",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Underground Sea",
          "count": 17
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Glorious Anthem",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Opalescence",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "bottomlessPool"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "TestPlayer.TARGET_SKIP"
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
          "name": "Locker Room"
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
          "name": "bottomlessPool"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "TestPlayer.TARGET_SKIP"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{4}{U}: Unlock the right half."
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
          "name": "Cackling Counterpart"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "\"Bottomless Pool \n    private static final String bottomlessPool = \"Bottomless Pool\""
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
          "name": "Bile Blight"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Locker Room"
        },
        {
          "op": "setStopAt",
          "turn": 1,
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
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Locker Room",
          "power": 3,
          "toughness": 3
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "\"Bottomless Pool \n    private static final String bottomlessPool = \"Bottomless Pool\"",
          "power": 4,
          "toughness": 4
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "bottomlessPool",
          "power": 2,
          "toughness": 2
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "EmptyNames.FULLY_LOCKED_ROOM.getTestCommand()",
          "power": 1,
          "toughness": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "bottomlessPool",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Locker Room",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "\"Bottomless Pool \n    private static final String bottomlessPool = \"Bottomless Pool\"",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "EmptyNames.FULLY_LOCKED_ROOM.getTestCommand()",
          "count": 1
        }
      ]
    },
    {
      "name": "testNameMatchOnFieldFromUnlocked",
      "operations": [
        {
          "op": "skipInitShuffling"
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "\"Bottomless Pool \n    private static final String bottomlessPool = \"Bottomless Pool\"",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Cackling Counterpart",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Bile Blight",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Underground Sea",
          "count": 17
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Glorious Anthem",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Opalescence",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "bottomlessPool"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "TestPlayer.TARGET_SKIP"
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
          "name": "Locker Room"
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
          "name": "bottomlessPool"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "TestPlayer.TARGET_SKIP"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{4}{U}: Unlock the right half."
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
          "name": "Cackling Counterpart"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "\"Bottomless Pool \n    private static final String bottomlessPool = \"Bottomless Pool\""
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
          "name": "Bile Blight"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "\"Bottomless Pool \n    private static final String bottomlessPool = \"Bottomless Pool\""
        },
        {
          "op": "setStopAt",
          "turn": 1,
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
          "op": "assertPermanentCount",
          "player": 0,
          "name": "bottomlessPool",
          "count": 0
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "\"Bottomless Pool \n    private static final String bottomlessPool = \"Bottomless Pool\"",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Locker Room",
          "power": 3,
          "toughness": 3
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "\"Bottomless Pool \n    private static final String bottomlessPool = \"Bottomless Pool\"",
          "power": 4,
          "toughness": 4
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "EmptyNames.FULLY_LOCKED_ROOM.getTestCommand()",
          "power": 1,
          "toughness": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Locker Room",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "\"Bottomless Pool \n    private static final String bottomlessPool = \"Bottomless Pool\"",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "EmptyNames.FULLY_LOCKED_ROOM.getTestCommand()",
          "count": 1
        }
      ]
    },
    {
      "name": "testCounterspellThenReanimate",
      "operations": [
        {
          "op": "skipInitShuffling"
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "\"Bottomless Pool \n    private static final String bottomlessPool = \"Bottomless Pool\"",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Counterspell",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Campus Renovation",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Grizzly Bears",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "bottomlessPool"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Counterspell"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "bottomlessPool"
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
          "name": "Campus Renovation"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "\"Bottomless Pool \n    private static final String bottomlessPool = \"Bottomless Pool\""
        },
        {
          "op": "setStopAt",
          "turn": 1,
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
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Grizzly Bears",
          "count": 1
        },
        {
          "op": "assertHandCount",
          "player": 1,
          "name": "Grizzly Bears",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "EmptyNames.FULLY_LOCKED_ROOM.getTestCommand()",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "assertType(EmptyNames.FULLY_LOCKED_ROOM.getTestCommand(), CardType.ENCHANTMENT, true)"
        },
        {
          "op": "unsupported",
          "source": "assertSubtype(EmptyNames.FULLY_LOCKED_ROOM.getTestCommand(), SubType.ROOM)"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Campus Renovation",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Counterspell",
          "count": 1
        }
      ]
    },
    {
      "name": "testPithingNeedleActivatedAbility",
      "operations": [
        {
          "op": "skipInitShuffling"
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "\"Bottomless Pool \n    private static final String bottomlessPool = \"Bottomless Pool\"",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Pithing Needle",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Opalescence",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Diviner's Wand",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 20
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "bottomlessPool"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "TestPlayer.TARGET_SKIP"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "Equip {3}"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "bottomlessPool"
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
          "name": "Pithing Needle"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Locker Room"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "label": "{4}: Draw a card.",
          "expected": true
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{4}{U}: Unlock the right half."
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "label": "{4}: Draw a card.",
          "expected": false
        },
        {
          "op": "setStopAt",
          "turn": 1,
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
          "op": "assertPermanentCount",
          "player": 0,
          "name": "\"Bottomless Pool \n    private static final String bottomlessPool = \"Bottomless Pool\"",
          "count": 1
        }
      ]
    },
    {
      "name": "testUnlockingPermanentMakeCopyOfOtherRoom",
      "operations": [
        {
          "op": "skipInitShuffling"
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "\"Bottomless Pool \n    private static final String bottomlessPool = \"Bottomless Pool\"",
          "count": 1
        }
      ]
    },
    {
      "name": "testSakashimaCopiesRoomCard",
      "operations": [
        {
          "op": "skipInitShuffling"
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "\"Bottomless Pool \n    private static final String bottomlessPool = \"Bottomless Pool\"",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 10
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Opalescence",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Sakashima the Impostor",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "bottomlessPool"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "TestPlayer.TARGET_SKIP"
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
          "name": "Sakashima the Impostor"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Yes"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "bottomlessPool"
        },
        {
          "op": "setStopAt",
          "turn": 1,
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
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Sakashima the Impostor",
          "count": 0
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Sakashima the Impostor",
          "count": 1
        }
      ]
    }
  ]
});
