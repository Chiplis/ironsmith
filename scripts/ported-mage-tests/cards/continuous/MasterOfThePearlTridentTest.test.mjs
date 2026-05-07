import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/continuous/MasterOfThePearlTridentTest.java",
  "tests": [
    {
      "name": "testLordAbility",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Master of the Pearl Trident",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Merfolk of the Pearl Trident",
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
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Island",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Master of the Pearl Trident"
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Merfolk of the Pearl Trident",
          "defender": 1
        },
        {
          "op": "block",
          "turn": 3,
          "player": 1,
          "blocker": "Llanowar Elves",
          "attacker": "Merfolk of the Pearl Trident"
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
          "player": 0,
          "name": "Master of the Pearl Trident",
          "count": 1
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 18
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Merfolk of the Pearl Trident",
          "power": 2,
          "toughness": 2
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Merfolk of the Pearl Trident",
          "ability": "new IslandwalkAbility()",
          "expected": true
        }
      ]
    },
    {
      "name": "testLordAbilityGone",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Master of the Pearl Trident",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Merfolk of the Pearl Trident",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Swamp",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Llanowar Elves",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Island",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Murder",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Master of the Pearl Trident"
        },
        {
          "op": "castSpell",
          "turn": 3,
          "phase": "DECLARE_ATTACKERS",
          "player": 1,
          "name": "Murder",
          "target": "Master of the Pearl Trident"
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Merfolk of the Pearl Trident",
          "defender": 1
        },
        {
          "op": "block",
          "turn": 3,
          "player": 1,
          "blocker": "Llanowar Elves",
          "attacker": "Merfolk of the Pearl Trident"
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
          "op": "assertLife",
          "player": 1,
          "life": 20
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Merfolk of the Pearl Trident",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Llanowar Elves",
          "count": 0
        }
      ]
    },
    {
      "name": "testTurnToFrog",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Master of the Pearl Trident",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Merfolk of the Pearl Trident",
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
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Llanowar Elves",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Turn to Frog",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Master of the Pearl Trident"
        },
        {
          "op": "castSpell",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Turn to Frog",
          "target": "Master of the Pearl Trident"
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Merfolk of the Pearl Trident",
          "defender": 1
        },
        {
          "op": "block",
          "turn": 3,
          "player": 1,
          "blocker": "Llanowar Elves",
          "attacker": "Merfolk of the Pearl Trident"
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
          "op": "assertLife",
          "player": 1,
          "life": 20
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Merfolk of the Pearl Trident",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Llanowar Elves",
          "count": 0
        }
      ]
    },
    {
      "name": "testTurnToFrogAndMurder",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Master of the Pearl Trident",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Merfolk of the Pearl Trident",
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
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Swamp",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Llanowar Elves",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Turn to Frog",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Murder",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Master of the Pearl Trident"
        },
        {
          "op": "castSpell",
          "turn": 3,
          "phase": "BEGIN_COMBAT",
          "player": 1,
          "name": "Turn to Frog",
          "target": "Master of the Pearl Trident"
        },
        {
          "op": "castSpell",
          "turn": 3,
          "phase": "DECLARE_ATTACKERS",
          "player": 1,
          "name": "Murder",
          "target": "Master of the Pearl Trident"
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Merfolk of the Pearl Trident",
          "defender": 1
        },
        {
          "op": "block",
          "turn": 3,
          "player": 1,
          "blocker": "Llanowar Elves",
          "attacker": "Merfolk of the Pearl Trident"
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
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Turn to Frog",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Murder",
          "count": 1
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 20
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Merfolk of the Pearl Trident",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Llanowar Elves",
          "count": 0
        }
      ]
    },
    {
      "name": "testLooseAndGainControl",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Master of the Pearl Trident",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Merfolk of the Pearl Trident",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Mountain",
          "count": 5
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Zealous Conscripts",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Zealous Conscripts"
        },
        {
          "op": "castSpell",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Merfolk of the Pearl Trident"
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
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Master of the Pearl Trident",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Merfolk of the Pearl Trident",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Merfolk of the Pearl Trident",
          "power": 2,
          "toughness": 2
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Zealous Conscripts",
          "count": 1
        }
      ]
    }
  ]
});
