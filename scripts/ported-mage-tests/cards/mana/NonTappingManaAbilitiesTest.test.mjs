import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/mana/NonTappingManaAbilitiesTest.java",
  "tests": [
    {
      "name": "druidsRepositoryTest",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Alaborn Grenadier",
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
          "name": "Druids' Repository",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Silvercoat Lion",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Silvercoat Lion",
          "defender": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Whenever a creature you control"
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
          "op": "assertTappedCount",
          "name": "Silvercoat Lion",
          "tapped": true,
          "count": 2
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Druids' Repository",
          "counter": "CHARGE",
          "count": 2
        },
        {
          "op": "unsupported",
          "source": "assertManaOptions(\"{Any}{Any}\", manaOptions)"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Alaborn Grenadier"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "White"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "White"
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
          "op": "assertCounterCount",
          "player": 0,
          "name": "Druids' Repository",
          "counter": "CHARGE",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Alaborn Grenadier",
          "count": 1
        }
      ]
    },
    {
      "name": "TestWorkhorse",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Workhorse",
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
          "source": "assertManaOptions(\"{C}{C}{C}{C}\", manaOptions)"
        }
      ]
    },
    {
      "name": "TestMorselhoarder",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Morselhoarder",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
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
          "source": "assertManaOptions(\"{B}{B}{Any}{Any}{Any}{Any}\", manaOptions)"
        }
      ]
    },
    {
      "name": "TestFarrelitePriest",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Farrelite Priest",
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
          "op": "setStopAt",
          "turn": 1,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "unsupported",
          "source": "assertManaOptions(\"{W}{W}{W}{W}\", manaOptions)"
        },
        {
          "op": "unsupported",
          "source": "assertManaOptions(\"{W}{W}{W}{B}\", manaOptions)"
        },
        {
          "op": "unsupported",
          "source": "assertManaOptions(\"{W}{W}{B}{B}\", manaOptions)"
        },
        {
          "op": "unsupported",
          "source": "assertManaOptions(\"{W}{B}{B}{B}\", manaOptions)"
        },
        {
          "op": "unsupported",
          "source": "assertManaOptions(\"{B}{B}{B}{B}\", manaOptions)"
        }
      ]
    },
    {
      "name": "TestCrystallineCrawler",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Crystalline Crawler",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "addCounters(1, PhaseStep.PRECOMBAT_MAIN, playerA, \"Crystalline Crawler\", CounterType.P1P1, 2)"
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
          "source": "assertManaOptions(\"{Any}{Any}\", manaOptions)"
        }
      ]
    },
    {
      "name": "TestCoalGolemAndDromarsAttendant",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
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
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Dromar's Attendant",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Coal Golem",
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
          "source": "assertManaOptions(\"{G}\", manaOptions)"
        },
        {
          "op": "unsupported",
          "source": "assertManaOptions(\"{W}{U}{B}\", manaOptions)"
        },
        {
          "op": "unsupported",
          "source": "assertManaOptions(\"{R}{R}{R}\", manaOptions)"
        }
      ]
    },
    {
      "name": "TestCoalGolemAndDromarsAttendantOrder2",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
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
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Coal Golem",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Dromar's Attendant",
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
          "source": "assertManaOptions(\"{G}\", manaOptions)"
        },
        {
          "op": "unsupported",
          "source": "assertManaOptions(\"{W}{U}{B}\", manaOptions)"
        },
        {
          "op": "unsupported",
          "source": "assertManaOptions(\"{R}{R}{R}\", manaOptions)"
        }
      ]
    },
    {
      "name": "TestJunglePatrol",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Jungle Patrol",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{1}{G}, {T}: Create"
        },
        {
          "op": "activateAbility",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{1}{G}, {T}: Create"
        },
        {
          "op": "activateAbility",
          "turn": 5,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{1}{G}, {T}: Create"
        },
        {
          "op": "setStopAt",
          "turn": 5,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Wood",
          "count": 3
        },
        {
          "op": "unsupported",
          "source": "assertManaOptions(\"{R}{R}{R}\", manaOptions)"
        }
      ]
    },
    {
      "name": "TestSquanderedResources",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
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
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Taiga",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Calciform Pools",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "River of Tears",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Squandered Resources",
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
          "source": "assertManaOptions(\"{C}{G}{G}{U}{U}{U}{R}{R}\", manaOptions)"
        },
        {
          "op": "unsupported",
          "source": "assertManaOptions(\"{C}{G}{G}{G}{G}{U}{U}{U}\", manaOptions)"
        },
        {
          "op": "unsupported",
          "source": "assertManaOptions(\"{C}{G}{G}{G}{U}{U}{U}{R}\", manaOptions)"
        },
        {
          "op": "unsupported",
          "source": "assertManaOptions(\"{C}{C}{G}{G}{U}{U}{R}{R}\", manaOptions)"
        },
        {
          "op": "unsupported",
          "source": "assertManaOptions(\"{C}{C}{G}{G}{G}{G}{U}{U}\", manaOptions)"
        },
        {
          "op": "unsupported",
          "source": "assertManaOptions(\"{C}{C}{G}{G}{G}{U}{U}{R}\", manaOptions)"
        },
        {
          "op": "unsupported",
          "source": "assertManaOptions(\"{C}{R}{R}{G}{G}{W}{U}{U}\", manaOptions)"
        },
        {
          "op": "unsupported",
          "source": "assertManaOptions(\"{C}{G}{G}{G}{G}{W}{U}{U}\", manaOptions)"
        },
        {
          "op": "unsupported",
          "source": "assertManaOptions(\"{C}{R}{G}{G}{G}{W}{U}{U}\", manaOptions)"
        }
      ]
    },
    {
      "name": "TestSquanderedResourcesWithManaConfluence",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
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
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mana Confluence",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Squandered Resources",
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
          "source": "assertManaOptions(\"{G}{G}{Any}{Any}\", manaOptions)"
        }
      ]
    },
    {
      "name": "TestTreasonousOgre",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Treasonous Ogre",
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
          "source": "assertManaOptions(\"{R}{R}{R}{R}{R}{R}\", manaOptions)"
        }
      ]
    },
    {
      "name": "TestSquanderedResourcesTwoSwamps",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Squandered Resources",
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
          "source": "assertManaOptions(\"{B}{B}{B}{B}\", manaOptions)"
        }
      ]
    },
    {
      "name": "Test_ManaCache",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mana Cache",
          "count": 1
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Mana Cache",
          "counter": "CHARGE",
          "count": 2
        },
        {
          "op": "unsupported",
          "source": "assertManaOptions(\"{C}{C}{B}{B}\", manaOptions)"
        }
      ]
    },
    {
      "name": "Test_ManaCacheOpponent",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mana Cache",
          "count": 1
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
          "op": "assertCounterCount",
          "player": 0,
          "name": "Mana Cache",
          "counter": "CHARGE",
          "count": 2
        },
        {
          "op": "unsupported",
          "source": "assertManaOptions(\"{C}{C}\", manaOptions)"
        }
      ]
    },
    {
      "name": "Test_ManaCacheActivate",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Upwelling",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mana Cache",
          "count": 1
        },
        {
          "op": "activateManaAbility",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "ability": "Remove a charge counter",
          "count": 1
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
          "op": "unsupported",
          "source": "assertManaPool(playerB, ManaType.COLORLESS, 1)"
        }
      ]
    },
    {
      "name": "testAvailableManaWithSpiritGuides",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Simian Spirit Guide",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Elvish Spirit Guide",
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
          "source": "assertManaOptions(\"{R}{G}\", manaOptions)"
        }
      ]
    }
  ]
});
