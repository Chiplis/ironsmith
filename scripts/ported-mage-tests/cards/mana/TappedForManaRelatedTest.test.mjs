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
          "op": "assertManaOptions",
          "player": 0,
          "turn": 3,
          "phase": "BEGIN_COMBAT",
          "options": [
            "{G}"
          ],
          "exact": true
        },
        {
          "op": "activateManaAbility",
          "turn": 3,
          "phase": "BEGIN_COMBAT",
          "player": 0,
          "source": "Gaea's Cradle",
          "ability": "{G}"
        },
        {
          "op": "activateManaAbility",
          "turn": 3,
          "phase": "BEGIN_COMBAT",
          "player": 0,
          "source": "Forest",
          "ability": "{G}"
        },
        {
          "op": "assertManaPool",
          "player": 0,
          "colors": "G",
          "amount": 1
        },
        {
          "op": "assertManaPool",
          "player": 0,
          "colors": "WUBRGC",
          "amount": 1
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
          "op": "assertManaOptions",
          "player": 0,
          "turn": 3,
          "phase": "BEGIN_COMBAT",
          "options": [
            "{G}{G}{G}{G}"
          ],
          "exact": true
        },
        {
          "op": "activateManaAbility",
          "turn": 3,
          "phase": "BEGIN_COMBAT",
          "player": 0,
          "source": "Gaea's Cradle",
          "ability": "{G}"
        },
        {
          "op": "activateManaAbility",
          "turn": 3,
          "phase": "BEGIN_COMBAT",
          "player": 0,
          "source": "Forest",
          "ability": "{G}"
        },
        {
          "op": "assertManaPool",
          "player": 0,
          "colors": "G",
          "amount": 4
        },
        {
          "op": "assertManaPool",
          "player": 0,
          "colors": "WUBRGC",
          "amount": 4
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
          "op": "assertManaOptions",
          "player": 0,
          "turn": 3,
          "phase": "BEGIN_COMBAT",
          "options": [
            "{G}{G}"
          ],
          "exact": true
        },
        {
          "op": "activateManaAbility",
          "turn": 3,
          "phase": "BEGIN_COMBAT",
          "player": 0,
          "source": "Forest",
          "ability": "{G}"
        },
        {
          "op": "assertManaPool",
          "player": 0,
          "colors": "G",
          "amount": 2
        },
        {
          "op": "assertManaPool",
          "player": 0,
          "colors": "WUBRGC",
          "amount": 2
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
          "op": "assertManaOptions",
          "player": 0,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "options": [
            "{C}"
          ],
          "exact": true,
          "prefundManaCosts": true
        },
        {
          "op": "activateManaAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "source": "Calciform Pools",
          "ability": "{C}"
        },
        {
          "op": "assertManaPool",
          "player": 0,
          "colors": "C",
          "amount": 1
        },
        {
          "op": "assertManaPool",
          "player": 0,
          "colors": "WUBRGC",
          "amount": 1
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
          "op": "assertManaOptions",
          "player": "playerA",
          "options": [
            "{W}{W}",
            "{W}{U}",
            "{U}{U}",
            "{C}"
          ],
          "exact": true,
          "prefundManaCosts": true
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
          "op": "assertManaOptions",
          "player": "playerA",
          "options": [
            "{W}{W}{W}",
            "{W}{W}{U}",
            "{U}{U}",
            "{C}"
          ],
          "exact": true,
          "prefundManaCosts": true
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
          "op": "assertManaOptions",
          "player": "playerA",
          "options": [
            "{C}{R}",
            "{B}"
          ],
          "exact": true,
          "prefundManaCosts": true
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
          "op": "assertManaOptions",
          "player": "playerA",
          "options": [
            "{C}{W}{W}",
            "{W}{B}",
            "{U}",
            "{R}"
          ],
          "exact": true,
          "prefundManaCosts": true
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
          "name": "Deathrite Shaman",
          "count": 1,
          "player": 0
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "name": "Mountain@land",
          "count": 1,
          "player": 0
        },
        {
          "op": "assertManaOptions",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "options": [
            ""
          ],
          "exact": true
        },
        {
          "op": "setOptionChoices",
          "description": "Add one mana of any color",
          "choices": [
            "Green"
          ],
          "kind": "colors",
          "player": 0
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "source": "Deathrite Shaman",
          "ability": "Exile target land card",
          "target": {
            "groups": [
              [
                "@land"
              ]
            ]
          }
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Deathrite Shaman",
          "count": 1,
          "abilityKind": "Activated",
          "objectTarget": "@land"
        },
        {
          "op": "assertManaPool",
          "player": 0,
          "colors": "WUBRGC",
          "amount": 0
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertExileCount",
          "player": 0,
          "name": "Mountain",
          "count": 1
        },
        {
          "op": "assertManaPool",
          "player": 0,
          "colors": "G",
          "amount": 1
        }
      ]
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
          "op": "assertManaOptions",
          "player": "playerA",
          "options": [
            "{U}{U}{U}{U}"
          ],
          "exact": true
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
          "op": "assertManaOptions",
          "player": 0,
          "turn": 1,
          "phase": "BEGIN_COMBAT",
          "options": [
            "{C}{C}{C}{C}{C}"
          ],
          "exact": true
        },
        {
          "op": "activateManaAbility",
          "turn": 1,
          "phase": "BEGIN_COMBAT",
          "player": 0,
          "source": "Chromatic Orrery",
          "ability": "{T}: Add {C}{C}{C}{C}{C}"
        },
        {
          "op": "assertManaPool",
          "player": 0,
          "colors": "C",
          "amount": 5
        },
        {
          "op": "assertManaPool",
          "player": 0,
          "colors": "WUBRGC",
          "amount": 5
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
          "op": "assertManaOptions",
          "player": "playerA",
          "options": [
            "{G}{G}{G}{G}{G}{G}{G}"
          ],
          "exact": true
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
          "op": "assertManaOptions",
          "player": "playerA",
          "options": [
            "{B}{B}{B}{B}{B}"
          ],
          "exact": true
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
          "op": "assertManaOptions",
          "player": "playerA",
          "options": [
            "{Any}"
          ],
          "exact": true
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 0,
            "name": "Pili-Pala"
          },
          "count": 1,
          "checks": [
            {
              "property": "attachmentCount",
              "expected": 1
            }
          ]
        },
        {
          "op": "assertAttachedTo",
          "player": 0,
          "attachment": "Paradise Mantle",
          "target": "Pili-Pala",
          "expected": true
        }
      ]
    }
  ]
});
