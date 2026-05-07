import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/enchantments/RoleTest.java",
  "tests": [
    {
      "name": "testRegular",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Nexus Wardens",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Cursed Courtier",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Cursed Courtier"
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
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Cursed Courtier",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Cursed",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Cursed Courtier",
          "power": 1,
          "toughness": 1
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 22
        }
      ]
    },
    {
      "name": "testReplace",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plateau",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Nexus Wardens",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Cursed Courtier",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Monstrous Rage",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Cursed Courtier"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Monstrous Rage",
          "target": "Cursed Courtier"
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
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Cursed Courtier",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Monstrous Rage",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Cursed",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Monster",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Cursed Courtier",
          "power": 6,
          "toughness": 4
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Cursed Courtier",
          "ability": "Trample",
          "expected": true
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 24
        }
      ]
    },
    {
      "name": "testReplaceResponse",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plateau",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Nexus Wardens",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Cursed Courtier",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Monstrous Rage",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Cursed Courtier"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null,
          "once": true
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Monstrous Rage",
          "target": "Cursed Courtier"
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
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Cursed Courtier",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Monstrous Rage",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Cursed",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Monster",
          "count": 0
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Cursed Courtier",
          "power": 3,
          "toughness": 1
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Cursed Courtier",
          "ability": "Trample",
          "expected": false
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 24
        }
      ]
    },
    {
      "name": "testSeparatePlayer",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 3
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
          "player": 0,
          "name": "Nexus Wardens",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Nexus Wardens",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Cursed Courtier",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Monstrous Rage",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Cursed Courtier"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 1,
          "name": "Monstrous Rage",
          "target": "Cursed Courtier"
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
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Cursed Courtier",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Monstrous Rage",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Cursed",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Monster",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Cursed Courtier",
          "power": 4,
          "toughness": 2
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Cursed Courtier",
          "ability": "Trample",
          "expected": true
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 22
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 22
        }
      ]
    },
    {
      "name": "testDoesntEnter",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Scrubland",
          "count": 6
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Nexus Wardens",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Cursed Courtier",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Murder",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Cursed Courtier"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Murder",
          "target": "Cursed Courtier"
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
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Cursed Courtier",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Cursed",
          "count": 0
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Cursed Courtier",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Murder",
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
      "name": "testProtectionNotCreated",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Azorius First-Wing",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Nexus Wardens",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Monstrous Rage",
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
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Monstrous Rage",
          "target": "Azorius First-Wing"
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
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Monstrous Rage",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Azorius First-Wing",
          "power": 4,
          "toughness": 2
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 20
        }
      ]
    },
    {
      "name": "testBecomeBrutes",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Runeclaw Bear",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Nexus Wardens",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Become Brutes",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 2
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Runeclaw Bear^Nexus Wardens"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Become Brutes"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "<i>Constellation</i>"
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
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Become Brutes",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Runeclaw Bear",
          "power": 3,
          "toughness": 3
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Nexus Wardens",
          "power": 2,
          "toughness": 5
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 24
        }
      ]
    }
  ]
});
