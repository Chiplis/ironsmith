import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/dka/GravecrawlerTest.java",
  "tests": [
    {
      "name": "testShouldBeCastable",
      "operations": [
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Gravecrawler",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Black Cat",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Gravecrawler"
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
          "name": "Gravecrawler",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "count": 0,
          "name": 0
        }
      ]
    },
    {
      "name": "testShouldNotBeCastable",
      "operations": [
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Gravecrawler",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 1
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "label": "Cast Gravecrawler",
          "expected": false
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
          "name": "Gravecrawler",
          "count": 0
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Gravecrawler",
          "count": 1
        }
      ]
    },
    {
      "name": "testCopiedCantBlockAbilityWorks",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Cryptoplasm",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Fervor",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Elite Vanguard",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Gravecrawler",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Llanowar Elves",
          "count": 1
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Gravecrawler"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Cryptoplasm"
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Elite Vanguard",
          "defender": 1
        },
        {
          "op": "block",
          "turn": 3,
          "player": 1,
          "blocker": "Gravecrawler",
          "attacker": "Elite Vanguard"
        },
        {
          "op": "attack",
          "turn": 4,
          "player": 1,
          "attacker": "Llanowar Elves",
          "defender": 1
        },
        {
          "op": "block",
          "turn": 4,
          "player": 0,
          "blocker": "Gravecrawler",
          "attacker": "Llanowar Elves"
        },
        {
          "op": "setStopAt",
          "turn": 4,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Gravecrawler",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Gravecrawler",
          "count": 1
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 18
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 19
        }
      ]
    },
    {
      "name": "testCantBlockAbilityAfterChangeZone",
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
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Fervor",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Elite Vanguard",
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
          "name": "Swamp",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Gravecrawler",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Walking Corpse",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Lightning Bolt",
          "target": "Gravecrawler"
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Gravecrawler"
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Elite Vanguard",
          "defender": 1
        },
        {
          "op": "block",
          "turn": 3,
          "player": 1,
          "blocker": "Gravecrawler",
          "attacker": "Elite Vanguard"
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Gravecrawler",
          "count": 1
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 18
        }
      ]
    }
  ]
});
