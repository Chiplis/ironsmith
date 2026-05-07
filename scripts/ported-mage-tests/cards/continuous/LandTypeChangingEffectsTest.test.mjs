import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/continuous/LandTypeChangingEffectsTest.java",
  "tests": [
    {
      "name": "testMagusOfTheMoonAndChromaticLantern",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Magus of the Moon",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Canopy Vista",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Plains",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Chromatic Lantern",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Chromatic Lantern"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
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
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Chromatic Lantern",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "assertType(\"Canopy Vista\", CardType.LAND, SubType.MOUNTAIN)"
        },
        {
          "op": "assertAbility",
          "player": 1,
          "name": "Canopy Vista",
          "ability": "new AnyColorManaAbility()",
          "expected": true
        }
      ]
    },
    {
      "name": "testChromaticLanternBeforeMagusOfTheMoon",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Magus of the Moon",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Canopy Vista",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Plains",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Chromatic Lantern",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Chromatic Lantern"
        },
        {
          "op": "castSpell",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Magus of the Moon"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
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
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Chromatic Lantern",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Magus of the Moon",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "assertType(\"Canopy Vista\", CardType.LAND, SubType.MOUNTAIN)"
        },
        {
          "op": "assertAbility",
          "player": 1,
          "name": "Canopy Vista",
          "ability": "new AnyColorManaAbility()",
          "expected": true
        }
      ]
    },
    {
      "name": "testLandDoesNotLooseOtherAbilities",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Aquitect's Will",
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
          "name": "Forbidding Watchtower",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Plains",
          "count": 2
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Aquitect's Will",
          "target": "Forbidding Watchtower"
        },
        {
          "op": "activateAbility",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "ability": "{1}{W}:"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
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
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Aquitect's Will",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Forbidding Watchtower",
          "count": 1
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Forbidding Watchtower",
          "counter": "FLOOD",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "assertType(\"Forbidding Watchtower\", CardType.LAND, SubType.ISLAND)"
        },
        {
          "op": "assertPowerToughness",
          "player": 1,
          "name": "Forbidding Watchtower",
          "power": 1,
          "toughness": 5
        }
      ]
    },
    {
      "name": "testBloodMoonBeforeUrborg",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Blood Moon",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Urborg, Tomb of Yawgmoth",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Canopy Vista",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Blood Moon"
        },
        {
          "op": "playLand",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Urborg, Tomb of Yawgmoth"
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
          "player": 0,
          "name": "Blood Moon",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Urborg, Tomb of Yawgmoth",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "assertType(canopyvista, CardType.LAND, SubType.MOUNTAIN)"
        },
        {
          "op": "unsupported",
          "source": "assertNotSubtype(canopyvista, SubType.ISLAND)"
        },
        {
          "op": "unsupported",
          "source": "assertNotSubtype(canopyvista, SubType.SWAMP)"
        },
        {
          "op": "unsupported",
          "source": "assertType(urborgtoy, CardType.LAND, SubType.MOUNTAIN)"
        },
        {
          "op": "unsupported",
          "source": "assertNotSubtype(urborgtoy, SubType.SWAMP)"
        }
      ]
    },
    {
      "name": "testBloodMoonAfterUrborg",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Blood Moon",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Urborg, Tomb of Yawgmoth",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Canopy Vista",
          "count": 1
        },
        {
          "op": "playLand",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Urborg, Tomb of Yawgmoth"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Blood Moon"
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
          "player": 0,
          "name": "Blood Moon",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Urborg, Tomb of Yawgmoth",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "assertType(canopyvista, CardType.LAND, SubType.MOUNTAIN)"
        },
        {
          "op": "unsupported",
          "source": "assertNotSubtype(canopyvista, SubType.ISLAND)"
        },
        {
          "op": "unsupported",
          "source": "assertNotSubtype(canopyvista, SubType.SWAMP)"
        },
        {
          "op": "unsupported",
          "source": "assertType(urborgtoy, CardType.LAND, SubType.MOUNTAIN)"
        },
        {
          "op": "unsupported",
          "source": "assertNotSubtype(urborgtoy, SubType.SWAMP)"
        }
      ]
    },
    {
      "name": "testCormusBellAfterUrborg",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Urborg, Tomb of Yawgmoth",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Kormus Bell",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Quicksilver Fountain",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Forest",
          "count": 3
        },
        {
          "op": "playLand",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Urborg, Tomb of Yawgmoth"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Kormus Bell"
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Quicksilver Fountain"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Mountain"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Urborg, Tomb of Yawgmoth",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Kormus Bell",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Quicksilver Fountain",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Mountain",
          "count": 0
        }
      ]
    },
    {
      "name": "testBloodSunWithUrborgtoyAndStormtideLeviathanMan",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Urborg, Tomb of Yawgmoth",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Blood Sun",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Stormtide Leviathan",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Darksteel Citadel",
          "count": 1
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "unsupported",
          "source": "assertType(urborgtoy, CardType.LAND, SubType.SWAMP)"
        },
        {
          "op": "unsupported",
          "source": "assertType(\"Mountain\", CardType.LAND, SubType.SWAMP)"
        },
        {
          "op": "unsupported",
          "source": "assertType(urborgtoy, CardType.LAND, SubType.ISLAND)"
        },
        {
          "op": "unsupported",
          "source": "assertType(\"Mountain\", CardType.LAND, SubType.ISLAND)"
        }
      ]
    },
    {
      "name": "testOrcishFarmer",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Orcish Farmer",
          "count": 2
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
          "name": "Reliquary Tower",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{T}: Target",
          "target": "Plains"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{T}: Target",
          "target": "Reliquary Tower"
        },
        {
          "op": "unsupported",
          "source": "checkSubType(\"Plains is Swamp on same turn\", 1, PhaseStep.END_TURN, playerA, \"Plains\", SubType.SWAMP, true)"
        },
        {
          "op": "unsupported",
          "source": "checkSubType(\"Plains is no longer Plains on same turn\", 1, PhaseStep.END_TURN, playerA, \"Plains\", SubType.PLAINS, false)"
        },
        {
          "op": "unsupported",
          "source": "checkSubType(\"Reliquary Tower is Swamp on same turn\", 1, PhaseStep.END_TURN, playerB, \"Reliquary Tower\", SubType.SWAMP, true)"
        },
        {
          "op": "unsupported",
          "source": "checkSubType(\"Plains is Swamp on next turn\", 2, PhaseStep.UPKEEP, playerA, \"Plains\", SubType.SWAMP, true)"
        },
        {
          "op": "unsupported",
          "source": "checkSubType(\"Plains is no longer Plains on next turn\", 2, PhaseStep.UPKEEP, playerA, \"Plains\", SubType.PLAINS, false)"
        },
        {
          "op": "unsupported",
          "source": "checkSubType(\"Reliquary Tower no longer Swamp on next turn\", 2, PhaseStep.UPKEEP, playerB, \"Reliquary Tower\", SubType.SWAMP, false)"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "UPKEEP"
        },
        {
          "op": "execute"
        },
        {
          "op": "unsupported",
          "source": "assertNotSubtype(\"Plains\", SubType.SWAMP)"
        },
        {
          "op": "unsupported",
          "source": "assertSubtype(\"Plains\", SubType.PLAINS)"
        }
      ]
    }
  ]
});
