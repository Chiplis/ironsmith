import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/abilities/equipped/ScytheOfTheWretchedTest.java",
  "tests": [
    {
      "name": "testEquipAlive",
      "operations": [
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
          "name": "Plains",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Scythe of the Wretched",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "ability": "Equip {4}",
          "target": "Silvercoat Lion"
        },
        {
          "op": "attack",
          "turn": 2,
          "player": 1,
          "attacker": "Silvercoat Lion",
          "defender": 1
        },
        {
          "op": "block",
          "turn": 2,
          "player": 0,
          "blocker": "Soulmender",
          "attacker": "Silvercoat Lion"
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
          "name": "Soulmender",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Soulmender",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 1,
          "name": "Soulmender",
          "power": 3,
          "toughness": 3
        },
        {
          "op": "assertPowerToughness",
          "player": 1,
          "name": "Silvercoat Lion",
          "power": 2,
          "toughness": 2
        }
      ]
    },
    {
      "name": "testEquipDied",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Oreskos Swiftclaw",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Plains",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Scythe of the Wretched",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "ability": "Equip {4}",
          "target": "Silvercoat Lion"
        },
        {
          "op": "attack",
          "turn": 2,
          "player": 1,
          "attacker": "Silvercoat Lion",
          "defender": 1
        },
        {
          "op": "block",
          "turn": 2,
          "player": 0,
          "blocker": "Oreskos Swiftclaw",
          "attacker": "Silvercoat Lion"
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
          "name": "Oreskos Swiftclaw",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Oreskos Swiftclaw",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 1,
          "name": "Oreskos Swiftclaw",
          "power": 5,
          "toughness": 3
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Silvercoat Lion",
          "count": 1
        }
      ]
    },
    {
      "name": "testEquipDiedInCombat",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Serra Angel",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Plains",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Scythe of the Wretched",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "ability": "Equip {4}",
          "target": "Silvercoat Lion"
        },
        {
          "op": "attack",
          "turn": 2,
          "player": 1,
          "attacker": "Silvercoat Lion",
          "defender": 1
        },
        {
          "op": "block",
          "turn": 2,
          "player": 0,
          "blocker": "Serra Angel",
          "attacker": "Silvercoat Lion"
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
          "name": "Serra Angel",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Serra Angel",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 1,
          "name": "Serra Angel",
          "power": 6,
          "toughness": 6
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Silvercoat Lion",
          "count": 1
        }
      ]
    },
    {
      "name": "testEquipDiedInCombat2",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Serra Angel",
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
          "name": "Disenchant",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Plains",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Scythe of the Wretched",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "ability": "Equip {4}",
          "target": "Silvercoat Lion"
        },
        {
          "op": "attack",
          "turn": 2,
          "player": 1,
          "attacker": "Silvercoat Lion",
          "defender": 1
        },
        {
          "op": "block",
          "turn": 2,
          "player": 0,
          "blocker": "Serra Angel",
          "attacker": "Silvercoat Lion"
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "COMBAT_DAMAGE",
          "player": 0,
          "name": "Disenchant",
          "target": "Scythe of the Wretched"
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
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Disenchant",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Scythe of the Wretched",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Serra Angel",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Serra Angel",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 1,
          "name": "Serra Angel",
          "power": 4,
          "toughness": 4
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Silvercoat Lion",
          "count": 1
        }
      ]
    }
  ]
});
