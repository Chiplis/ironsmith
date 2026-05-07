import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/rules/DontUntapTest.java",
  "tests": [
    {
      "name": "testTritonTactics",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Triton Tactics",
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
          "name": "Soulmender",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 4,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "ability": "{T}: You gain 1 life"
        },
        {
          "op": "attack",
          "turn": 4,
          "player": 1,
          "attacker": "Silvercoat Lion",
          "defender": 1
        },
        {
          "op": "castSpell",
          "turn": 4,
          "phase": "DECLARE_ATTACKERS",
          "player": 0,
          "name": "Triton Tactics",
          "target": "Soulmender"
        },
        {
          "op": "block",
          "turn": 4,
          "player": 0,
          "blocker": "Soulmender",
          "attacker": "Silvercoat Lion"
        },
        {
          "op": "setStopAt",
          "turn": 6,
          "phase": "DRAW"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Triton Tactics",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Soulmender",
          "power": 1,
          "toughness": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 1,
          "name": "Silvercoat Lion",
          "power": 2,
          "toughness": 2
        },
        {
          "op": "unsupported",
          "source": "assertTapped(\"Silvercoat Lion\", true)"
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 21
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 20
        }
      ]
    },
    {
      "name": "TestAjaniVengeantFirst",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Sublime Archangel",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Ajani Vengeant",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Sublime Archangel",
          "defender": 1
        },
        {
          "op": "activateAbility",
          "turn": 2,
          "phase": "POSTCOMBAT_MAIN",
          "player": 1,
          "ability": "+1: Target permanent doesn't",
          "target": "Sublime Archangel"
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "DRAW"
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
          "life": 15
        },
        {
          "op": "unsupported",
          "source": "assertTapped(\"Sublime Archangel\", true)"
        }
      ]
    },
    {
      "name": "testExhaustionEffect",
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
          "name": "Swamp",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Reassembling Skeleton",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Island",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Exhaustion",
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
          "op": "castSpell",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Exhaustion",
          "target": 0
        },
        {
          "op": "activateAbility",
          "turn": 2,
          "phase": "END_TURN",
          "player": 0,
          "ability": "{1}{B}:"
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
          "op": "unsupported",
          "source": "assertTapped(\"Silvercoat Lion\", true)"
        },
        {
          "op": "unsupported",
          "source": "assertTapped(\"Reassembling Skeleton\", true)"
        },
        {
          "op": "unsupported",
          "source": "assertTapped(\"Swamp\", true)"
        }
      ]
    },
    {
      "name": "testManaVaporsEffect",
      "operations": [
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
          "name": "Lightning Bolt",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Island",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Mana Vapors",
          "count": 1
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
          "op": "castSpell",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Mana Vapors",
          "target": 0
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
          "op": "unsupported",
          "source": "assertTapped(\"Mountain\", true)"
        }
      ]
    },
    {
      "name": "testMisstepEffect",
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
          "name": "Swamp",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Reassembling Skeleton",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Island",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Misstep",
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
          "op": "castSpell",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Misstep",
          "target": 0
        },
        {
          "op": "activateAbility",
          "turn": 2,
          "phase": "END_TURN",
          "player": 0,
          "ability": "{1}{B}:"
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
          "op": "unsupported",
          "source": "assertTapped(\"Silvercoat Lion\", true)"
        },
        {
          "op": "unsupported",
          "source": "assertTapped(\"Reassembling Skeleton\", true)"
        }
      ]
    },
    {
      "name": "testPollenLullabyEffect",
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
          "name": "Swamp",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Reassembling Skeleton",
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
          "name": "Pollen Lullaby",
          "count": 1
        },
        {
          "op": "clearZone",
          "player": 0,
          "zone": "library"
        },
        {
          "op": "clearZone",
          "player": 1,
          "zone": "library"
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Island",
          "count": 8
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 1,
          "name": "Silvercoat Lion",
          "count": 8
        },
        {
          "op": "skipInitShuffling"
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Silvercoat Lion",
          "defender": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "DECLARE_BLOCKERS",
          "player": 1,
          "name": "Pollen Lullaby"
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": "PlayerA"
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": false
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": false
        },
        {
          "op": "activateAbility",
          "turn": 2,
          "phase": "END_TURN",
          "player": 0,
          "ability": "{1}{B}:"
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
          "op": "unsupported",
          "source": "assertTapped(\"Silvercoat Lion\", true)"
        },
        {
          "op": "unsupported",
          "source": "assertTapped(\"Reassembling Skeleton\", true)"
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 20
        }
      ]
    },
    {
      "name": "testIcebreakerKrakenEffect",
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
          "name": "Marble Chalice",
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
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Reassembling Skeleton",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Island",
          "count": 12
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Icebreaker Kraken",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{T}:"
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Silvercoat Lion",
          "defender": 1
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Icebreaker Kraken"
        },
        {
          "op": "addTarget",
          "player": 1,
          "target": 0
        },
        {
          "op": "activateAbility",
          "turn": 2,
          "phase": "END_TURN",
          "player": 0,
          "ability": "{1}{B}:"
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
          "source": "assertTapped(\"Silvercoat Lion\", true)"
        },
        {
          "op": "unsupported",
          "source": "assertTapped(\"Marble Chalice\", true)"
        },
        {
          "op": "unsupported",
          "source": "assertTapped(\"Reassembling Skeleton\", true)"
        }
      ]
    }
  ]
});
