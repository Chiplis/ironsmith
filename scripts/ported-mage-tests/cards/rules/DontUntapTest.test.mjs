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
          "op": "attack",
          "turn": 4,
          "player": 1,
          "attacker": "Silvercoat Lion",
          "defender": 0
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
          "op": "assertStackObject",
          "turn": 4,
          "phase": "DECLARE_ATTACKERS",
          "source": "Triton Tactics",
          "count": 1,
          "onTop": true,
          "objectTarget": "Soulmender"
        },
        {
          "op": "waitStackResolved",
          "turn": 4,
          "phase": "DECLARE_ATTACKERS",
          "resolutions": 1
        },
        {
          "op": "block",
          "turn": 4,
          "player": 0,
          "blocker": "Soulmender",
          "attacker": "Silvercoat Lion"
        },
        {
          "op": "assertLife",
          "turn": 4,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "life": 20,
          "seatIndependent": true
        },
        {
          "op": "activateAbility",
          "turn": 4,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "ability": "{T}: You gain 1 life",
          "source": "Soulmender"
        },
        {
          "op": "assertStackObject",
          "turn": 4,
          "phase": "POSTCOMBAT_MAIN",
          "source": "Soulmender",
          "count": 1,
          "onTop": true,
          "abilityKind": "Activated"
        },
        {
          "op": "waitStackResolved",
          "turn": 4,
          "phase": "POSTCOMBAT_MAIN",
          "resolutions": 1
        },
        {
          "op": "assertTappedCount",
          "player": 1,
          "name": "Silvercoat Lion",
          "tapped": true,
          "count": 1
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
          "op": "assertTappedCount",
          "player": 1,
          "name": "Silvercoat Lion",
          "count": 1,
          "tapped": true
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
          "op": "assertStackObject",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "source": "Exhaustion",
          "count": 1,
          "onTop": true,
          "controller": 1,
          "targetCount": 1
        },
        {
          "op": "waitStackResolved",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "resolutions": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Exhaustion",
          "count": 1
        },
        {
          "op": "assertTappedCount",
          "player": 0,
          "name": "Silvercoat Lion",
          "tapped": true,
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 2,
          "phase": "END_TURN",
          "player": 0,
          "ability": "{1}{B}:",
          "source": "Reassembling Skeleton"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Reassembling Skeleton"
        },
        {
          "op": "assertStackObject",
          "turn": 2,
          "phase": "END_TURN",
          "source": "Reassembling Skeleton",
          "abilityKind": "Activated",
          "count": 1,
          "onTop": true
        },
        {
          "op": "waitStackResolved",
          "turn": 2,
          "phase": "END_TURN",
          "resolutions": 1
        },
        {
          "op": "assertTappedCount",
          "player": 0,
          "name": "Reassembling Skeleton",
          "tapped": true,
          "count": 1
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
          "op": "assertTappedCount",
          "player": 0,
          "name": "Silvercoat Lion",
          "tapped": true,
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "assertTapped(\"Reassembling Skeleton\", true)"
        },
        {
          "op": "assertTappedCount",
          "player": 0,
          "name": "Swamp",
          "tapped": true,
          "count": 2
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
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Lightning Bolt",
          "count": 1,
          "onTop": true
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "resolutions": 1
        },
        {
          "op": "assertTappedCount",
          "player": 0,
          "name": "Mountain",
          "tapped": true,
          "count": 1
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
          "op": "assertStackObject",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "source": "Mana Vapors",
          "count": 1,
          "onTop": true,
          "controller": 1,
          "targetCount": 1
        },
        {
          "op": "waitStackResolved",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "resolutions": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Mana Vapors",
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
          "op": "assertTappedCount",
          "player": 0,
          "name": "Mountain",
          "tapped": true,
          "count": 1
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
          "ability": "{1}{B}:",
          "source": "Reassembling Skeleton"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Reassembling Skeleton"
        },
        {
          "op": "assertStackObject",
          "turn": 2,
          "phase": "END_TURN",
          "source": "Reassembling Skeleton",
          "abilityKind": "Activated",
          "count": 1,
          "onTop": true
        },
        {
          "op": "waitStackResolved",
          "turn": 2,
          "phase": "END_TURN",
          "resolutions": 1
        },
        {
          "op": "assertTappedCount",
          "player": 0,
          "name": "Reassembling Skeleton",
          "tapped": true,
          "count": 1
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
          "op": "assertTappedCount",
          "player": 0,
          "name": "Swamp",
          "tapped": false,
          "count": 2
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
          "op": "addTarget",
          "player": 0,
          "target": "TestPlayer.TARGET_SKIP"
        },
        {
          "op": "addTarget",
          "player": 1,
          "target": "TestPlayer.TARGET_SKIP"
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "DECLARE_BLOCKERS",
          "source": "Pollen Lullaby",
          "count": 1,
          "onTop": true,
          "controller": 1
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "DECLARE_BLOCKERS",
          "resolutions": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Pollen Lullaby",
          "count": 1
        },
        {
          "op": "assertTappedCount",
          "player": 0,
          "name": "Silvercoat Lion",
          "tapped": true,
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 2,
          "phase": "END_TURN",
          "player": 0,
          "ability": "{1}{B}:",
          "source": "Reassembling Skeleton"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Reassembling Skeleton"
        },
        {
          "op": "assertStackObject",
          "turn": 2,
          "phase": "END_TURN",
          "source": "Reassembling Skeleton",
          "abilityKind": "Activated",
          "count": 1,
          "onTop": true
        },
        {
          "op": "waitStackResolved",
          "turn": 2,
          "phase": "END_TURN",
          "resolutions": 1
        },
        {
          "op": "assertTappedCount",
          "player": 0,
          "name": "Reassembling Skeleton",
          "tapped": true,
          "count": 1
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
          "op": "assertTappedCount",
          "player": 0,
          "name": "Silvercoat Lion",
          "tapped": true,
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "assertTapped(\"Reassembling Skeleton\", true)"
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 20
        },
        {
          "op": "assertTappedCount",
          "player": 0,
          "name": "Swamp",
          "tapped": false,
          "count": 2
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
          "ability": "{1}{B}:",
          "source": "Reassembling Skeleton"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Reassembling Skeleton"
        },
        {
          "op": "assertStackObject",
          "turn": 2,
          "phase": "END_TURN",
          "source": "Reassembling Skeleton",
          "abilityKind": "Activated",
          "count": 1,
          "onTop": true
        },
        {
          "op": "waitStackResolved",
          "turn": 2,
          "phase": "END_TURN",
          "resolutions": 1
        },
        {
          "op": "assertTappedCount",
          "player": 0,
          "name": "Reassembling Skeleton",
          "tapped": true,
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
          "source": "assertTapped(\"Silvercoat Lion\", true)"
        },
        {
          "op": "unsupported",
          "source": "assertTapped(\"Marble Chalice\", true)"
        },
        {
          "op": "unsupported",
          "source": "assertTapped(\"Reassembling Skeleton\", true)"
        },
        {
          "op": "assertTappedCount",
          "player": 0,
          "name": "Swamp",
          "tapped": false,
          "count": 2
        }
      ]
    }
  ]
});
