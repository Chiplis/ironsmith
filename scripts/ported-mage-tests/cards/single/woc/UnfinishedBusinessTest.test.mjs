import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/woc/UnfinishedBusinessTest.java",
  "tests": [
    {
      "name": "testShroud",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Unfinished Business",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Deadly Insect",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 5
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Blanchwood Armor",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Shuko",
          "count": 1
        },
        {
          "op": "castSpell",
          "name": "Unfinished Business",
          "player": 0,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "target": {
            "groups": [
              [
                "Deadly Insect"
              ],
              [
                "Blanchwood Armor",
                "Shuko"
              ]
            ]
          }
        },
        {
          "op": "assertStackObject",
          "source": "Unfinished Business",
          "count": 1,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "targetCount": 3,
          "objectTarget": "Deadly Insect"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Deadly Insect",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Blanchwood Armor",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Shuko",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Blanchwood Armor",
          "count": 0
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Shuko",
          "count": 0
        },
        {
          "op": "assertAttachedTo",
          "player": 0,
          "attachment": "\"Blanchwood Armor\"",
          "target": "\"Deadly Insect\"",
          "expected": true
        },
        {
          "op": "assertAttachedTo",
          "player": 0,
          "attachment": "\"Shuko\"",
          "target": "\"Deadly Insect\"",
          "expected": true
        }
      ]
    },
    {
      "name": "testProtection",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Unfinished Business",
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
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Apostle of Purifying Light",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 5
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Ghoulflesh",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Enormous Energy Blade",
          "count": 1
        },
        {
          "op": "castSpell",
          "name": "Unfinished Business",
          "player": 0,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "target": {
            "groups": [
              [
                "Apostle of Purifying Light"
              ],
              [
                "Ghoulflesh",
                "Enormous Energy Blade"
              ]
            ]
          }
        },
        {
          "op": "assertStackObject",
          "source": "Unfinished Business",
          "count": 1,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "targetCount": 3,
          "objectTarget": "Apostle of Purifying Light"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Apostle of Purifying Light",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Ghoulflesh",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Enormous Energy Blade",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Ghoulflesh",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Enormous Energy Blade",
          "count": 0
        },
        {
          "op": "assertAttachedTo",
          "player": 0,
          "attachment": "\"Enormous Energy Blade\"",
          "target": null,
          "expected": true
        },
        {
          "op": "assertTappedCount",
          "player": 0,
          "name": "Apostle of Purifying Light",
          "count": 1,
          "tapped": false
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 20
        }
      ]
    },
    {
      "name": "testExileCreatureBeforeResolution",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 5
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
          "player": 0,
          "name": "Unfinished Business",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Shuko",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Blanchwood Armor",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Grizzly Bears",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Apostle of Purifying Light",
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
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Unfinished Business"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Grizzly Bears"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Shuko^Blanchwood Armor"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "ability": "{2}: ",
          "target": "Grizzly Bears"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "END_TURN",
          "player": null
        },
        {
          "op": "execute"
        },
        {
          "op": "assertExileCount",
          "player": 0,
          "name": "Grizzly Bears",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Blanchwood Armor",
          "count": 1
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 20
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Shuko",
          "count": 1
        }
      ]
    }
  ]
});
