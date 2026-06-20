import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/abilities/keywords/ImpendingTest.java",
  "tests": [
    {
      "name": "testCastRegular",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 5
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Overlord of the Hauntwoods",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Overlord of the Hauntwoods"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Cast with no"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Overlord of the Hauntwoods",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "assertType(hauntwoods, CardType.ENCHANTMENT, true)"
        },
        {
          "op": "unsupported",
          "source": "assertType(hauntwoods, CardType.CREATURE, true)"
        },
        {
          "op": "unsupported",
          "source": "assertSubtype(hauntwoods, SubType.AVATAR)"
        },
        {
          "op": "unsupported",
          "source": "assertSubtype(hauntwoods, SubType.HORROR)"
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Overlord of the Hauntwoods",
          "counter": "TIME",
          "count": 0
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Overlord of the Hauntwoods",
          "power": 6,
          "toughness": 5
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Everywhere",
          "count": 1
        }
      ]
    },
    {
      "name": "testCastImpending",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Overlord of the Hauntwoods",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Overlord of the Hauntwoods"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Cast with Impending"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Overlord of the Hauntwoods",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "assertType(hauntwoods, CardType.ENCHANTMENT, true)"
        },
        {
          "op": "unsupported",
          "source": "assertType(hauntwoods, CardType.CREATURE, false)"
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Overlord of the Hauntwoods",
          "counter": "TIME",
          "count": 4
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Everywhere",
          "count": 1
        }
      ]
    },
    {
      "name": "testImpendingRemoveCounter",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Overlord of the Hauntwoods",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Overlord of the Hauntwoods"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Cast with Impending"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Overlord of the Hauntwoods",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "assertType(hauntwoods, CardType.ENCHANTMENT, true)"
        },
        {
          "op": "unsupported",
          "source": "assertType(hauntwoods, CardType.CREATURE, false)"
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Overlord of the Hauntwoods",
          "counter": "TIME",
          "count": 3
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Everywhere",
          "count": 1
        }
      ]
    },
    {
      "name": "testCastImpendingRemoveAllCounters",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Overlord of the Hauntwoods",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Overlord of the Hauntwoods"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Cast with Impending"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 8,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Overlord of the Hauntwoods",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "assertType(hauntwoods, CardType.ENCHANTMENT, true)"
        },
        {
          "op": "unsupported",
          "source": "assertType(hauntwoods, CardType.CREATURE, true)"
        },
        {
          "op": "unsupported",
          "source": "assertSubtype(hauntwoods, SubType.AVATAR)"
        },
        {
          "op": "unsupported",
          "source": "assertSubtype(hauntwoods, SubType.HORROR)"
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Overlord of the Hauntwoods",
          "counter": "TIME",
          "count": 0
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Overlord of the Hauntwoods",
          "power": 6,
          "toughness": 5
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Everywhere",
          "count": 1
        }
      ]
    },
    {
      "name": "testCastImpendingHexmage",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Vampire Hexmage",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Overlord of the Hauntwoods",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Overlord of the Hauntwoods"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Cast with Impending"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "BEGIN_COMBAT",
          "player": 0,
          "ability": "Sacrifice",
          "target": "Overlord of the Hauntwoods"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Overlord of the Hauntwoods",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "assertType(hauntwoods, CardType.ENCHANTMENT, true)"
        },
        {
          "op": "unsupported",
          "source": "assertType(hauntwoods, CardType.CREATURE, true)"
        },
        {
          "op": "unsupported",
          "source": "assertSubtype(hauntwoods, SubType.AVATAR)"
        },
        {
          "op": "unsupported",
          "source": "assertSubtype(hauntwoods, SubType.HORROR)"
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Overlord of the Hauntwoods",
          "counter": "TIME",
          "count": 0
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Overlord of the Hauntwoods",
          "power": 6,
          "toughness": 5
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Everywhere",
          "count": 1
        }
      ]
    },
    {
      "name": "testCastImpendingHexmageNextTurn",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Vampire Hexmage",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Overlord of the Hauntwoods",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Overlord of the Hauntwoods"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Cast with Impending"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "BEGIN_COMBAT",
          "player": 0,
          "ability": "Sacrifice",
          "target": "Overlord of the Hauntwoods"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Overlord of the Hauntwoods",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "assertType(hauntwoods, CardType.ENCHANTMENT, true)"
        },
        {
          "op": "unsupported",
          "source": "assertType(hauntwoods, CardType.CREATURE, true)"
        },
        {
          "op": "unsupported",
          "source": "assertSubtype(hauntwoods, SubType.AVATAR)"
        },
        {
          "op": "unsupported",
          "source": "assertSubtype(hauntwoods, SubType.HORROR)"
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Overlord of the Hauntwoods",
          "counter": "TIME",
          "count": 0
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Overlord of the Hauntwoods",
          "power": 6,
          "toughness": 5
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Everywhere",
          "count": 1
        }
      ]
    },
    {
      "name": "testCastImpendingSolemnity",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Solemnity",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Overlord of the Hauntwoods",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Overlord of the Hauntwoods"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Cast with Impending"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Overlord of the Hauntwoods",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "assertType(hauntwoods, CardType.ENCHANTMENT, true)"
        },
        {
          "op": "unsupported",
          "source": "assertType(hauntwoods, CardType.CREATURE, true)"
        },
        {
          "op": "unsupported",
          "source": "assertSubtype(hauntwoods, SubType.AVATAR)"
        },
        {
          "op": "unsupported",
          "source": "assertSubtype(hauntwoods, SubType.HORROR)"
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Overlord of the Hauntwoods",
          "counter": "TIME",
          "count": 0
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Overlord of the Hauntwoods",
          "power": 6,
          "toughness": 5
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Everywhere",
          "count": 1
        }
      ]
    },
    {
      "name": "testCastImpendingSolemnityNextTurn",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Solemnity",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Overlord of the Hauntwoods",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Overlord of the Hauntwoods"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Cast with Impending"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Overlord of the Hauntwoods",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "assertType(hauntwoods, CardType.ENCHANTMENT, true)"
        },
        {
          "op": "unsupported",
          "source": "assertType(hauntwoods, CardType.CREATURE, true)"
        },
        {
          "op": "unsupported",
          "source": "assertSubtype(hauntwoods, SubType.AVATAR)"
        },
        {
          "op": "unsupported",
          "source": "assertSubtype(hauntwoods, SubType.HORROR)"
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Overlord of the Hauntwoods",
          "counter": "TIME",
          "count": 0
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Overlord of the Hauntwoods",
          "power": 6,
          "toughness": 5
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Everywhere",
          "count": 1
        }
      ]
    }
  ]
});
