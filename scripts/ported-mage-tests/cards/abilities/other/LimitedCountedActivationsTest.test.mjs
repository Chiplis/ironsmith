import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/abilities/other/LimitedCountedActivationsTest.java",
  "tests": [
    {
      "name": "testDragonWhelpActivatedThreeTimes",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Dragon Whelp",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{R}: "
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{R}: "
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{R}: "
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Dragon Whelp",
          "defender": 1
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "UPKEEP"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Dragon Whelp",
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
          "life": 15
        }
      ]
    },
    {
      "name": "testDragonWhelpActivatedFourTimes",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Dragon Whelp",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{R}: "
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{R}: "
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{R}: "
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{R}: "
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Dragon Whelp",
          "defender": 1
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "UPKEEP"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Dragon Whelp",
          "count": 0
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Dragon Whelp",
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
          "life": 14
        }
      ]
    },
    {
      "name": "testDragonWhelpActivatedFiveTimes",
      "operations": [
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
          "name": "Dragon Whelp",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{R}: "
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{R}: "
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{R}: "
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{R}: "
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{R}: "
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Dragon Whelp",
          "defender": 1
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "UPKEEP"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Dragon Whelp",
          "count": 0
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Dragon Whelp",
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
          "life": 13
        }
      ]
    },
    {
      "name": "testDragonWhelpTwoObjects",
      "operations": [
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
          "name": "Swamp",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Dragon Whelp",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Reanimate",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Unnatural Speed",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Swamp",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Terror",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "UPKEEP",
          "player": 0,
          "ability": "{R}: "
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "UPKEEP",
          "player": 1,
          "name": "Terror",
          "target": "Dragon Whelp"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Reanimate",
          "target": "Dragon Whelp"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "BEGIN_COMBAT",
          "player": 0,
          "name": "Unnatural Speed",
          "target": "Dragon Whelp"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "DECLARE_ATTACKERS",
          "player": 0,
          "ability": "{R}: "
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "DECLARE_ATTACKERS",
          "player": 0,
          "ability": "{R}: "
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "DECLARE_ATTACKERS",
          "player": 0,
          "ability": "{R}: "
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Dragon Whelp",
          "defender": 1
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "UPKEEP"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Unnatural Speed",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Reanimate",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Terror",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Dragon Whelp",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Dragon Whelp",
          "power": 2,
          "toughness": 3
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Dragon Whelp",
          "count": 0
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 16
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 15
        }
      ]
    },
    {
      "name": "testDragonWhelpDontSacrificeNewObject",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 8
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Dragon Whelp",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Reanimate",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Unnatural Speed",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Swamp",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Terror",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "UPKEEP",
          "player": 0,
          "ability": "{R}: "
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "UPKEEP",
          "player": 0,
          "ability": "{R}: "
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "UPKEEP",
          "player": 0,
          "ability": "{R}: "
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "UPKEEP",
          "player": 0,
          "ability": "{R}: "
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "UPKEEP",
          "player": null
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "UPKEEP",
          "player": 1,
          "name": "Terror",
          "target": "Dragon Whelp"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Reanimate",
          "target": "Dragon Whelp"
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
          "phase": "BEGIN_COMBAT",
          "player": 0,
          "name": "Unnatural Speed",
          "target": "Dragon Whelp"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "DECLARE_ATTACKERS",
          "player": 0,
          "ability": "{R}: "
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "DECLARE_ATTACKERS",
          "player": 0,
          "ability": "{R}: "
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "DECLARE_ATTACKERS",
          "player": 0,
          "ability": "{R}: "
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Dragon Whelp",
          "defender": 1
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "UPKEEP"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Unnatural Speed",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Reanimate",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Terror",
          "count": 1
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 16
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 15
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Dragon Whelp",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Dragon Whelp",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Dragon Whelp",
          "power": 2,
          "toughness": 3
        }
      ]
    }
  ]
});
