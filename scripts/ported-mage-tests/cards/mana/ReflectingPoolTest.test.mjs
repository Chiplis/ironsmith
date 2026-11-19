import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/mana/ReflectingPoolTest.java",
  "tests": [
    {
      "name": "testTriggeredManaAbility",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Lightning Bolt",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Reflecting Pool",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Crumbling Vestige",
          "count": 1
        },
        {
          "op": "playLand",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Crumbling Vestige"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Red"
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
          "name": "Lightning Bolt",
          "target": 1
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
          "name": "Crumbling Vestige",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Lightning Bolt",
          "count": 1
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 20
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 17
        }
      ]
    },
    {
      "name": "testWithExoticOrchard",
      "operations": [
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
          "player": 0,
          "name": "Reflecting Pool",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Exotic Orchard",
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
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "options": [
            "{R}{R}"
          ],
          "exact": true
        }
      ]
    },
    {
      "name": "test2WithExoticOrchard",
      "operations": [
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
          "player": 0,
          "name": "Reflecting Pool",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Exotic Orchard",
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
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "options": [
            "{R}{R}{R}"
          ],
          "exact": true
        }
      ]
    },
    {
      "name": "testWith2ExoticOrchard",
      "operations": [
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
          "player": 0,
          "name": "Reflecting Pool",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Exotic Orchard",
          "count": 2
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
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "options": [
            "{R}{R}{R}"
          ],
          "exact": true
        }
      ]
    },
    {
      "name": "testWithGaeasCradle",
      "operations": [
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
          "name": "Reflecting Pool",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Gaea's Cradle",
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
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "options": [
            "{G}{G}"
          ],
          "exact": true
        }
      ]
    },
    {
      "name": "testWithDifferentLands",
      "operations": [
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
          "name": "Reflecting Pool",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Exotic Orchard",
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
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Exotic Orchard",
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
          "op": "setStopAt",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertManaOptions",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "exact": true,
          "options": [
            "{G}{G}{G}",
            "{G}{G}{W}",
            "{G}{W}{W}"
          ]
        },
        {
          "op": "assertManaOptions",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "exact": true,
          "options": [
            "{G}{W}",
            "{W}{W}"
          ]
        }
      ]
    },
    {
      "name": "testReflectingPoolGiveNonMana",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Augmenting Automaton",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Reflecting Pool",
          "count": 1
        },
        {
          "op": "assertNoAction",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "label": "Cast Augmenting Automaton",
          "kind": "cast_spell",
          "sourceZone": "hand",
          "checkPayment": true
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
          "op": "assertManaPool",
          "player": 0,
          "colors": "WUBRGC",
          "amount": 0
        }
      ]
    },
    {
      "name": "testReflectingPoolGiveNonMana2",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Augmenting Automaton",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Reflecting Pool",
          "count": 2
        },
        {
          "op": "assertNoAction",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "label": "Cast Augmenting Automaton",
          "kind": "cast_spell",
          "sourceZone": "hand",
          "checkPayment": true
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
      "name": "testReflectingPoolGiveBasicManaNeed",
      "operations": [
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
          "name": "Balduvian Bears",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Reflecting Pool",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Balduvian Bears"
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
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Balduvian Bears",
          "count": 1
        },
        {
          "op": "assertTappedCount",
          "player": 0,
          "name": "Forest",
          "count": 1,
          "tapped": true
        },
        {
          "op": "assertTappedCount",
          "player": 0,
          "name": "Reflecting Pool",
          "count": 1,
          "tapped": true
        }
      ]
    },
    {
      "name": "testReflectingPoolGiveBasicManaNotNeed",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Balduvian Bears",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Reflecting Pool",
          "count": 1
        },
        {
          "op": "assertNoAction",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "label": "Cast Balduvian Bears",
          "kind": "cast_spell",
          "sourceZone": "hand",
          "checkPayment": true
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
      "name": "testReflectingPoolAnyManaNeedWithoutCondition",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "City of Brass",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Razorclaw Bear",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Reflecting Pool",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Upwelling",
          "count": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Green"
        },
        {
          "op": "activateManaAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "source": "City of Brass",
          "ability": "Add one mana of any color",
          "count": 1
        },
        {
          "op": "setOptionChoices",
          "player": 0,
          "kind": "options",
          "description": "Choose one",
          "choices": [
            "{G}"
          ]
        },
        {
          "op": "activateManaAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "source": "Reflecting Pool",
          "ability": "Add one mana",
          "count": 1
        },
        {
          "op": "assertManaPool",
          "player": 0,
          "colors": "G",
          "amount": 2
        },
        {
          "op": "activateManaAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "source": "Plains",
          "ability": "Add {W}",
          "count": 2
        },
        {
          "op": "assertManaPool",
          "player": 0,
          "colors": "W",
          "amount": 2
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Razorclaw Bear",
          "holdStack": true
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Razorclaw Bear",
          "count": 1,
          "isSpell": true
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
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
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Razorclaw Bear",
          "count": 1
        }
      ]
    },
    {
      "name": "testReflectingPoolAnyManaNeedWithCondition",
      "operations": [
        {
          "op": "addCardWithEntryChoice",
          "player": 0,
          "name": "Cavern of Souls",
          "choice": "Goblin"
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Balduvian Bears",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Reflecting Pool",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Upwelling",
          "count": 1
        },
        {
          "op": "setChoice",
          "value": "{G}"
        },
        {
          "op": "activateManaAbility",
          "player": 0,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Reflecting Pool",
          "ability": "Add one mana of any type"
        },
        {
          "op": "assertManaPool",
          "player": 0,
          "colors": "G",
          "amount": 1
        },
        {
          "op": "activateManaAbility",
          "player": 0,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Cavern of Souls",
          "ability": "{T}: Add {C}"
        },
        {
          "op": "assertManaPool",
          "player": 0,
          "colors": "GC",
          "amount": 2
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Balduvian Bears"
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
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Balduvian Bears",
          "count": 1
        },
        {
          "op": "assertTappedCount",
          "player": 0,
          "name": "Cavern of Souls",
          "count": 1,
          "tapped": true
        },
        {
          "op": "assertTappedCount",
          "player": 0,
          "name": "Reflecting Pool",
          "count": 1,
          "tapped": true
        }
      ]
    },
    {
      "name": "testReflectingPoolAnyManaTapped",
      "operations": [
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
          "name": "City of Brass",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Reflecting Pool",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Upwelling",
          "count": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Black"
        },
        {
          "op": "activateManaAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{T}: Add one mana of any",
          "count": 1,
          "source": "City of Brass"
        },
        {
          "op": "activateManaAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{T}: Add {W}",
          "count": 1,
          "source": "Plains"
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
          "source": "assertTapped(\"City of Brass\", true)"
        },
        {
          "op": "unsupported",
          "source": "assertTapped(\"Plains\", true)"
        },
        {
          "op": "unsupported",
          "source": "assertTapped(\"Reflecting Pool\", false)"
        },
        {
          "op": "assertManaPool",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "colors": "B",
          "amount": 1
        },
        {
          "op": "assertManaPool",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "colors": "W",
          "amount": 1
        }
      ]
    },
    {
      "name": "testWithTriomeAndManaConfluence",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Academy Ruins",
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
          "name": "Reflecting Pool",
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
            "{C}{C}{Any}",
            "{C}{Any}{Any}"
          ],
          "exact": true
        }
      ]
    },
    {
      "name": "testWithCalciformPools",
      "operations": [
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
          "name": "Reflecting Pool",
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
            "{C}{C}",
            "{C}{W}",
            "{C}{U}"
          ],
          "exact": true,
          "prefundManaCosts": true
        }
      ]
    }
  ]
});
