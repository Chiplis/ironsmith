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
          "op": "assertManaOptions",
          "player": "playerA",
          "options": [
            "{Any}{Any}"
          ],
          "exact": true
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
          "op": "assertManaOptions",
          "player": "playerA",
          "options": [
            "{C}{C}{C}{C}"
          ],
          "exact": true
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
          "op": "assertManaOptions",
          "player": "playerA",
          "options": [
            "{B}{B}{Any}{Any}{Any}{Any}"
          ],
          "exact": true
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
          "op": "assertManaOptions",
          "player": "playerA",
          "options": [
            "{W}{W}{W}{W}",
            "{W}{W}{W}{B}",
            "{W}{W}{B}{B}",
            "{W}{B}{B}{B}",
            "{B}{B}{B}{B}"
          ],
          "exact": true,
          "prefundManaCosts": true
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
          "op": "assertManaOptions",
          "player": "playerA",
          "options": [
            "{Any}{Any}"
          ],
          "exact": true
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
          "op": "assertManaOptions",
          "player": "playerA",
          "options": [
            "{G}",
            "{W}{U}{B}",
            "{R}{R}{R}"
          ],
          "exact": true,
          "prefundManaCosts": true
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
          "op": "assertManaOptions",
          "player": "playerA",
          "options": [
            "{G}",
            "{W}{U}{B}",
            "{R}{R}{R}"
          ],
          "exact": true,
          "prefundManaCosts": true
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
          "op": "assertManaOptions",
          "player": "playerA",
          "options": [
            "{R}{R}{R}"
          ],
          "exact": true
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
          "op": "assertManaOptions",
          "player": "playerA",
          "options": [
            "{C}{G}{G}{U}{U}{U}{R}{R}",
            "{C}{G}{G}{G}{G}{U}{U}{U}",
            "{C}{G}{G}{G}{U}{U}{U}{R}",
            "{C}{C}{G}{G}{U}{U}{R}{R}",
            "{C}{C}{G}{G}{G}{G}{U}{U}",
            "{C}{C}{G}{G}{G}{U}{U}{R}",
            "{C}{R}{R}{G}{G}{W}{U}{U}",
            "{C}{G}{G}{G}{G}{W}{U}{U}",
            "{C}{R}{G}{G}{G}{W}{U}{U}"
          ],
          "exact": true,
          "prefundManaCosts": true,
          "ignoreGraveyardHistory": true,
          "branchLimit": 24000
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
          "op": "assertManaOptions",
          "player": "playerA",
          "options": [
            "{G}{G}{Any}{Any}"
          ],
          "exact": true
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
          "op": "assertManaOptions",
          "player": "playerA",
          "options": [
            "{R}{R}{R}{R}{R}{R}"
          ],
          "exact": true
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
          "op": "assertManaOptions",
          "player": "playerA",
          "options": [
            "{B}{B}{B}{B}"
          ],
          "exact": true
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
          "op": "assertManaOptions",
          "player": "playerA",
          "options": [
            "{C}{C}{B}{B}"
          ],
          "exact": true
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
          "op": "assertManaOptions",
          "player": "playerB",
          "options": [
            "{C}{C}"
          ],
          "exact": true
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
          "op": "assertManaOptions",
          "player": "playerA",
          "options": [
            "{R}{G}"
          ],
          "exact": true
        }
      ]
    }
  ]
});
