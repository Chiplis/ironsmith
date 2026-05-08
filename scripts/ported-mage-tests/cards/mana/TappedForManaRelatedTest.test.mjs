import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/mana/TappedForManaRelatedTest.java",
  "tests": [
    {
      "name": "TestCradleWithWildGrowthNoCreatures",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Gaea's Cradle",
          "count": 1
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
          "name": "Wild Growth",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Wild Growth",
          "target": "Gaea's Cradle"
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
          "player": 0,
          "name": "Wild Growth",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "assertManaOptions(\"{G}{G}\", manaOptions)"
        }
      ]
    },
    {
      "name": "TestCradleWithWildGrowthTwoCreatures",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Gaea's Cradle",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 2
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
          "name": "Wild Growth",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Wild Growth",
          "target": "Gaea's Cradle"
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
          "player": 0,
          "name": "Wild Growth",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "assertManaOptions(\"{G}{G}{G}{G}\", manaOptions)"
        }
      ]
    },
    {
      "name": "TestWildGrowth",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 2
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
          "name": "Wild Growth",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Wild Growth",
          "target": "Forest"
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
          "player": 0,
          "name": "Wild Growth",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "assertManaOptions(\"{G}{G}\", manaOptions)"
        }
      ]
    },
    {
      "name": "TestCalciformPools",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Calciform Pools",
          "count": 1
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "unsupported",
          "source": "assertManaOptions(\"{C}\", manaOptions)"
        }
      ]
    },
    {
      "name": "TestCalciformPools2Counter",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Calciform Pools",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "addCounters(1, PhaseStep.PRECOMBAT_MAIN, playerA, \"Calciform Pools\", CounterType.STORAGE, 2)"
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
          "op": "assertCounterCount",
          "player": 0,
          "name": "Calciform Pools",
          "counter": "STORAGE",
          "count": 2
        },
        {
          "op": "unsupported",
          "source": "assertManaOptions(\"{W}{W}\", manaOptions)"
        },
        {
          "op": "unsupported",
          "source": "assertManaOptions(\"{W}{U}\", manaOptions)"
        },
        {
          "op": "unsupported",
          "source": "assertManaOptions(\"{U}{U}\", manaOptions)"
        },
        {
          "op": "unsupported",
          "source": "assertManaOptions(\"{C}\", manaOptions)"
        }
      ]
    },
    {
      "name": "TestCalciformPools2CounterAndTrigger",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Calciform Pools",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "addCounters(1, PhaseStep.PRECOMBAT_MAIN, playerA, \"Calciform Pools\", CounterType.STORAGE, 2)"
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Caged Sun",
          "count": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "White"
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
          "op": "assertCounterCount",
          "player": 0,
          "name": "Calciform Pools",
          "counter": "STORAGE",
          "count": 2
        },
        {
          "op": "unsupported",
          "source": "assertManaOptions(\"{W}{W}{W}\", manaOptions)"
        },
        {
          "op": "unsupported",
          "source": "assertManaOptions(\"{W}{W}{U}\", manaOptions)"
        },
        {
          "op": "unsupported",
          "source": "assertManaOptions(\"{U}{U}\", manaOptions)"
        },
        {
          "op": "unsupported",
          "source": "assertManaOptions(\"{C}\", manaOptions)"
        }
      ]
    },
    {
      "name": "TestCastleSengir",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Castle Sengir",
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
          "op": "setStopAt",
          "turn": 1,
          "phase": "UPKEEP"
        },
        {
          "op": "execute"
        },
        {
          "op": "unsupported",
          "source": "assertManaOptions(\"{C}{R}\", manaOptions)"
        },
        {
          "op": "unsupported",
          "source": "assertManaOptions(\"{B}\", manaOptions)"
        }
      ]
    },
    {
      "name": "TestCastleSengir2",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Castle Sengir",
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
          "op": "setStopAt",
          "turn": 1,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "unsupported",
          "source": "assertManaOptions(\"{C}{W}{W}\", manaOptions)"
        },
        {
          "op": "unsupported",
          "source": "assertManaOptions(\"{W}{B}\", manaOptions)"
        },
        {
          "op": "unsupported",
          "source": "assertManaOptions(\"{U}\", manaOptions)"
        },
        {
          "op": "unsupported",
          "source": "assertManaOptions(\"{R}\", manaOptions)"
        }
      ]
    },
    {
      "name": "TestDeathriteShaman",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Deathrite Shaman",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Mountain",
          "count": 3
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
          "op": "unsupported",
          "source": "assertManaOptions(\"{Any}\", manaOptions)"
        }
      ],
      "skip": "upstream @Ignore"
    },
    {
      "name": "TestEyeOfRamos",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Eye of Ramos",
          "count": 2
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
          "op": "unsupported",
          "source": "assertManaOptions(\"{U}{U}{U}{U}\", manaOptions)"
        }
      ]
    },
    {
      "name": "TestChromaticOrrery",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Chromatic Orrery",
          "count": 1
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
          "op": "unsupported",
          "source": "assertManaOptions(\"{C}{C}{C}{C}{C}\", manaOptions)"
        }
      ]
    },
    {
      "name": "TestViridianJoiner",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Viridian Joiner",
          "count": 1
        },
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
          "player": 1,
          "name": "Forest",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Giant Growth",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Viridian Joiner"
        },
        {
          "op": "castSpell",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Giant Growth",
          "target": "Viridian Joiner"
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
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Giant Growth",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Viridian Joiner",
          "power": 4,
          "toughness": 5
        },
        {
          "op": "unsupported",
          "source": "assertManaOptions(\"{G}{G}{G}{G}{G}{G}{G}\", manaOptions)"
        }
      ]
    },
    {
      "name": "TestPriestOfYawgmoth",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Priest of Yawgmoth",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Abandoned Sarcophagus",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Accorder's Shield",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Adarkar Sentinel",
          "count": 1
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "unsupported",
          "source": "assertManaOptions(\"{B}{B}{B}{B}{B}\", manaOptions)"
        }
      ]
    },
    {
      "name": "TestParadiseMantle",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Paradise Mantle",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Pili-Pala",
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
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "Equip"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Pili-Pala"
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
          "op": "unsupported",
          "source": "assertManaOptions(\"{Any}\", manaOptions)"
        }
      ]
    }
  ]
});
