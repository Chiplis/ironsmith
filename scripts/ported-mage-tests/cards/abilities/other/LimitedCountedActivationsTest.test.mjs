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
          "source": "Dragon Whelp",
          "ability": "This creature gets +1/+0",
          "player": 0,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "activateAbility",
          "source": "Dragon Whelp",
          "ability": "This creature gets +1/+0",
          "player": 0,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "activateAbility",
          "source": "Dragon Whelp",
          "ability": "This creature gets +1/+0",
          "player": 0,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertStackObject",
          "source": "Dragon Whelp",
          "count": 3,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "abilityKind": "Activated"
        },
        {
          "op": "assertTappedCount",
          "player": 0,
          "name": "Mountain",
          "count": 3,
          "tapped": true
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Dragon Whelp",
          "power": 5,
          "toughness": 3
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Dragon Whelp",
          "defender": 1
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "END_COMBAT"
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 15
        },
        {
          "op": "assertStackObject",
          "source": "Dragon Whelp",
          "count": 0,
          "turn": 1,
          "phase": "END_TURN",
          "abilityKind": "Triggered"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "END_TURN"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Dragon Whelp",
          "count": 1
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
          "life": 20
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
          "source": "Dragon Whelp",
          "ability": "This creature gets +1/+0",
          "player": 0,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "activateAbility",
          "source": "Dragon Whelp",
          "ability": "This creature gets +1/+0",
          "player": 0,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "activateAbility",
          "source": "Dragon Whelp",
          "ability": "This creature gets +1/+0",
          "player": 0,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "activateAbility",
          "source": "Dragon Whelp",
          "ability": "This creature gets +1/+0",
          "player": 0,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertStackObject",
          "source": "Dragon Whelp",
          "count": 4,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "abilityKind": "Activated"
        },
        {
          "op": "assertTappedCount",
          "player": 0,
          "name": "Mountain",
          "count": 4,
          "tapped": true
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Dragon Whelp",
          "power": 6,
          "toughness": 3
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Dragon Whelp",
          "defender": 1
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "END_COMBAT"
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 14
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "END_TURN",
          "count": 4,
          "source": "Dragon Whelp",
          "abilityKind": "Triggered"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "END_TURN"
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
          "source": "Dragon Whelp",
          "ability": "This creature gets +1/+0",
          "player": 0,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "activateAbility",
          "source": "Dragon Whelp",
          "ability": "This creature gets +1/+0",
          "player": 0,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "activateAbility",
          "source": "Dragon Whelp",
          "ability": "This creature gets +1/+0",
          "player": 0,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "activateAbility",
          "source": "Dragon Whelp",
          "ability": "This creature gets +1/+0",
          "player": 0,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "activateAbility",
          "source": "Dragon Whelp",
          "ability": "This creature gets +1/+0",
          "player": 0,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertStackObject",
          "source": "Dragon Whelp",
          "count": 5,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "abilityKind": "Activated"
        },
        {
          "op": "assertTappedCount",
          "player": 0,
          "name": "Mountain",
          "count": 5,
          "tapped": true
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Dragon Whelp",
          "power": 7,
          "toughness": 3
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Dragon Whelp",
          "defender": 1
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "END_COMBAT"
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 13
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "END_TURN",
          "count": 5,
          "source": "Dragon Whelp",
          "abilityKind": "Triggered"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "END_TURN"
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
          "ability": "{R}: ",
          "source": "Dragon Whelp"
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
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Dragon Whelp",
          "defender": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "DECLARE_ATTACKERS",
          "player": 0,
          "ability": "{R}: ",
          "source": "Dragon Whelp"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "DECLARE_ATTACKERS",
          "player": 0,
          "ability": "{R}: ",
          "source": "Dragon Whelp"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "DECLARE_ATTACKERS",
          "player": 0,
          "ability": "{R}: ",
          "source": "Dragon Whelp"
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Dragon Whelp",
          "power": 5,
          "toughness": 3,
          "turn": 1,
          "phase": "DECLARE_ATTACKERS",
          "seatIndependent": true
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
          "ability": "{R}: ",
          "source": "Dragon Whelp"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "UPKEEP",
          "player": 0,
          "ability": "{R}: ",
          "source": "Dragon Whelp"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "UPKEEP",
          "player": 0,
          "ability": "{R}: ",
          "source": "Dragon Whelp"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "UPKEEP",
          "player": 0,
          "ability": "{R}: ",
          "source": "Dragon Whelp"
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
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Dragon Whelp",
          "defender": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "DECLARE_ATTACKERS",
          "player": 0,
          "ability": "{R}: ",
          "source": "Dragon Whelp"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "DECLARE_ATTACKERS",
          "player": 0,
          "ability": "{R}: ",
          "source": "Dragon Whelp"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "DECLARE_ATTACKERS",
          "player": 0,
          "ability": "{R}: ",
          "source": "Dragon Whelp"
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Dragon Whelp",
          "power": 5,
          "toughness": 3,
          "turn": 1,
          "phase": "DECLARE_ATTACKERS",
          "seatIndependent": true
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
