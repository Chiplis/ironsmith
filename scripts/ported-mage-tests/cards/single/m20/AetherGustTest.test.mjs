import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/m20/AetherGustTest.java",
  "tests": [
    {
      "name": "testAetherGustWorksWithSpells",
      "operations": [
        {
          "op": "clearZone",
          "player": 0,
          "zone": "library"
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Barkhide Troll",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Aether Gust",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Barkhide Troll"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Aether Gust",
          "target": "Barkhide Troll"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "END_TURN"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "execute"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Aether Gust",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Barkhide Troll",
          "count": 0
        },
        {
          "op": "assertLibraryCount",
          "player": 0,
          "count": 1
        }
      ]
    },
    {
      "name": "testAetherGustWorksWithPermanents",
      "operations": [
        {
          "op": "clearZone",
          "player": 0,
          "zone": "library"
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Barkhide Troll",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Aether Gust",
          "count": 1
        },
        {
          "op": "activateManaAbility",
          "source": "Forest",
          "ability": "Add {G}",
          "player": 0,
          "count": 2,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "castSpell",
          "name": "Barkhide Troll",
          "player": 0,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertStackObject",
          "source": "Barkhide Troll",
          "count": 1,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Barkhide Troll",
          "count": 1
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 0,
            "name": "Barkhide Troll"
          },
          "count": 1,
          "checks": [
            {
              "property": "color",
              "value": "green",
              "expected": true
            }
          ]
        },
        {
          "op": "activateManaAbility",
          "source": "Island",
          "ability": "Add {U}",
          "player": 0,
          "count": 2,
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "castSpell",
          "name": "Aether Gust",
          "player": 0,
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "target": "Barkhide Troll"
        },
        {
          "op": "assertStackObject",
          "source": "Aether Gust",
          "count": 1,
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "objectTarget": "Barkhide Troll",
          "targetCount": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Aether Gust",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Barkhide Troll",
          "count": 0
        },
        {
          "op": "assertLibraryCount",
          "player": 0,
          "name": "Barkhide Troll",
          "count": 1
        }
      ]
    }
  ]
});
