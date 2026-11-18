import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/neo/EaterOfVirtueTest.java",
  "tests": [
    {
      "name": "testEaterOfVirtue",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Eater of Virtue",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 5
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Adult Gold Dragon",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Balduvian Bears",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Doom Blade",
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
          "op": "activateAbility",
          "source": "Eater of Virtue",
          "ability": "Equip {1}",
          "player": 0,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "target": "Adult Gold Dragon"
        },
        {
          "op": "assertStackObject",
          "source": "Eater of Virtue",
          "count": 1,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "abilityKind": "Activated",
          "objectTarget": "Adult Gold Dragon"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertAttachedTo",
          "player": 0,
          "attachment": "Eater of Virtue",
          "target": "Adult Gold Dragon",
          "expected": true
        },
        {
          "op": "castSpell",
          "name": "Doom Blade",
          "player": 1,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "target": "Adult Gold Dragon"
        },
        {
          "op": "assertStackObject",
          "source": "Doom Blade",
          "count": 1,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "objectTarget": "Adult Gold Dragon"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "resolutions": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Adult Gold Dragon",
          "count": 1
        },
        {
          "op": "assertStackObject",
          "source": "Eater of Virtue",
          "count": 1,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "abilityKind": "Triggered"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertExileCount",
          "player": 0,
          "name": "Adult Gold Dragon",
          "count": 1
        },
        {
          "op": "activateAbility",
          "source": "Eater of Virtue",
          "ability": "Equip {1}",
          "player": 0,
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "target": "Balduvian Bears"
        },
        {
          "op": "assertStackObject",
          "source": "Eater of Virtue",
          "count": 1,
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "abilityKind": "Activated",
          "objectTarget": "Balduvian Bears"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "assertAttachedTo",
          "player": 0,
          "attachment": "Eater of Virtue",
          "target": "Balduvian Bears",
          "expected": true
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 0,
            "name": "Balduvian Bears"
          },
          "count": 1,
          "checks": [
            {
              "property": "keyword",
              "expected": true,
              "value": "flying"
            },
            {
              "property": "keyword",
              "expected": true,
              "value": "lifelink"
            },
            {
              "property": "keyword",
              "expected": true,
              "value": "haste"
            }
          ]
        }
      ]
    }
  ]
});
