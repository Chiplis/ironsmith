import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/replacement/DoublingSeasonTest.java",
  "tests": [
    {
      "name": "test_Counters_ByTrigger",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Doubling Season",
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
          "zone": "HAND",
          "player": 0,
          "name": "Pallid Mycoderm",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Pallid Mycoderm"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
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
          "name": "Pallid Mycoderm",
          "counter": "SPORE",
          "count": 2
        }
      ]
    },
    {
      "name": "test_Tokens_ByActivate",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Doubling Season",
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
          "zone": "HAND",
          "player": 0,
          "name": "Pallid Mycoderm",
          "count": 1
        },
        {
          "op": "castSpell",
          "name": "Pallid Mycoderm",
          "player": 0,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertStackObject",
          "source": "Pallid Mycoderm",
          "count": 1,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Pallid Mycoderm",
          "count": 0,
          "counter": "spore"
        },
        {
          "op": "waitStackResolved",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Pallid Mycoderm",
          "count": 2,
          "counter": "spore"
        },
        {
          "op": "waitStackResolved",
          "turn": 5,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Pallid Mycoderm",
          "count": 4,
          "counter": "spore"
        },
        {
          "op": "activateAbility",
          "source": "Pallid Mycoderm",
          "ability": "Create a 1/1 green Saproling",
          "player": 0,
          "turn": 5,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertStackObject",
          "source": "Pallid Mycoderm",
          "count": 1,
          "turn": 5,
          "phase": "PRECOMBAT_MAIN",
          "abilityKind": "Activated"
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Pallid Mycoderm",
          "count": 1,
          "counter": "spore"
        },
        {
          "op": "waitStackResolved",
          "turn": 5,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Saproling Token",
          "count": 2
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 20
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 20
        }
      ]
    },
    {
      "name": "test_Counters_ByPreventEffect",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Doubling Season",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Silvercoat Lion",
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
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Test of Faith",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Mountain",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Plains",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Lightning Helix",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Test of Faith",
          "target": "Silvercoat Lion"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 1,
          "name": "Lightning Helix",
          "target": "Silvercoat Lion"
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
          "op": "assertLife",
          "player": 0,
          "life": 20
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 23
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Silvercoat Lion",
          "counter": "P1P1",
          "count": 6
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Silvercoat Lion",
          "power": 8,
          "toughness": 8
        }
      ]
    },
    {
      "name": "test_Tokens_ByPlaneswalkerActivate",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Liliana, Dreadhorde General",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Doubling Season",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "+1"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
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
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Zombie Token",
          "count": 2
        }
      ]
    },
    {
      "name": "test_RiteOfReplication_Tokens_ByMultipleEffects",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Doubling Season",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 9
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Rite of Replication",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Venerable Monk",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Rite of Replication",
          "target": "Venerable Monk"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "setStrictChooseMode",
          "value": true
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
          "op": "assertLife",
          "player": 0,
          "life": "20 + 10 * 2"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Venerable Monk",
          "count": 10
        }
      ]
    },
    {
      "name": "test_RiteOfReplication_TokensFromController_CanAttack",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Rite of the Raging Storm",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 5
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Doubling Season",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Rite of the Raging Storm"
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Lightning Rager",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Lightning Rager",
          "defender": 1
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
          "name": "Rite of the Raging Storm",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Lightning Rager",
          "count": 2
        },
        {
          "op": "assertTappedCount",
          "name": "Lightning Rager",
          "tapped": true,
          "count": 2
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 10
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 20
        }
      ]
    },
    {
      "name": "test_RiteOfReplication_TokensFromOpponent_CanNotAttack",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Rite of the Raging Storm",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 5
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Doubling Season",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Rite of the Raging Storm"
        },
        {
          "op": "attack",
          "turn": 2,
          "player": 1,
          "attacker": "Lightning Rager",
          "defender": 0,
          "expectedLegal": false
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
          "name": "Rite of the Raging Storm",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Lightning Rager",
          "count": 2
        },
        {
          "op": "assertTappedCount",
          "name": "Lightning Rager",
          "tapped": true,
          "count": 0
        }
      ]
    },
    {
      "name": "test_AddCardOrderDepends",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Tibalt, the Fiend-Blooded",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Doubling Season",
          "count": 1
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Tibalt, the Fiend-Blooded",
          "count": 2,
          "counter": "LOYALTY",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
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
          "op": "assertLife",
          "player": 0,
          "life": 20
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 20
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Tibalt, the Fiend-Blooded",
          "counter": "LOYALTY",
          "count": 2
        }
      ]
    },
    {
      "name": "test_LoyaltyCounters_DoubleSeason",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Doubling Season",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Tibalt, the Fiend-Blooded",
          "count": 1
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Tibalt, the Fiend-Blooded",
          "count": 4,
          "counter": "LOYALTY",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "+1: Draw a card, then discard a card at random.",
          "source": "Tibalt, the Fiend-Blooded"
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Tibalt, the Fiend-Blooded",
          "count": 1,
          "abilityKind": "Activated"
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Tibalt, the Fiend-Blooded",
          "count": 5,
          "counter": "LOYALTY",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "holdStack": true
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "resolutions": 1
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
          "op": "assertLife",
          "player": 0,
          "life": 20
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 20
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Tibalt, the Fiend-Blooded",
          "counter": "LOYALTY",
          "count": 5
        }
      ]
    },
    {
      "name": "test_LoyaltyCounters_Pir",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Pir, Imaginative Rascal",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Tibalt, the Fiend-Blooded",
          "count": 1
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Tibalt, the Fiend-Blooded",
          "count": 3,
          "counter": "LOYALTY",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "+1: Draw a card, then discard a card at random.",
          "source": "Tibalt, the Fiend-Blooded"
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Tibalt, the Fiend-Blooded",
          "count": 1,
          "abilityKind": "Activated"
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Tibalt, the Fiend-Blooded",
          "count": 5,
          "counter": "LOYALTY",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "holdStack": true
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "resolutions": 1
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
          "op": "assertLife",
          "player": 0,
          "life": 20
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 20
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Tibalt, the Fiend-Blooded",
          "counter": "LOYALTY",
          "count": 5
        }
      ]
    },
    {
      "name": "test_LoyaltyCounters_SeasonAndPir_SeasonFirst",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Doubling Season",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Pir, Imaginative Rascal",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Chandra, Fire Artisan",
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
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Chandra, Fire Artisan"
        },
        {
          "op": "setOptionChoices",
          "player": 0,
          "kind": "replacement_effect",
          "description": "Choose which replacement effect to apply",
          "choices": [
            "Doubling Season"
          ]
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Chandra, Fire Artisan",
          "count": 1
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "resolutions": 1
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Chandra, Fire Artisan",
          "count": 9,
          "counter": "LOYALTY",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "+1",
          "source": "Chandra, Fire Artisan"
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Chandra, Fire Artisan",
          "count": 1,
          "abilityKind": "Activated"
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Chandra, Fire Artisan",
          "count": 13,
          "counter": "LOYALTY",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "holdStack": true
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "resolutions": 1
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
          "op": "assertCounterCount",
          "player": 0,
          "name": "Chandra, Fire Artisan",
          "counter": "LOYALTY",
          "count": 13
        }
      ]
    },
    {
      "name": "test_LoyaltyCounters_SeasonAndPir_PirFirst",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Doubling Season",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Pir, Imaginative Rascal",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Chandra, Fire Artisan",
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
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Chandra, Fire Artisan"
        },
        {
          "op": "setOptionChoices",
          "player": 0,
          "kind": "replacement_effect",
          "description": "Choose which replacement effect to apply",
          "choices": [
            "Pir, Imaginative Rascal"
          ]
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Chandra, Fire Artisan",
          "count": 1
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "resolutions": 1
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Chandra, Fire Artisan",
          "count": 10,
          "counter": "LOYALTY",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "+1",
          "source": "Chandra, Fire Artisan"
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Chandra, Fire Artisan",
          "count": 1,
          "abilityKind": "Activated"
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Chandra, Fire Artisan",
          "count": 14,
          "counter": "LOYALTY",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "holdStack": true
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "resolutions": 1
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
          "op": "assertCounterCount",
          "player": 0,
          "name": "Chandra, Fire Artisan",
          "counter": "LOYALTY",
          "count": 14
        }
      ]
    }
  ]
});
