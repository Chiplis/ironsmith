import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/spm/SuperiorSpiderManTest.java",
  "tests": [
    {
      "name": "testSuperiorSpiderMan",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Superior Spider-Man@spider",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Adelbert Steiner@steiner",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Underground Sea",
          "count": 4
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Superior Spider-Man",
          "holdStack": true,
          "objectAlias": "@spider"
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Superior Spider-Man",
          "count": 1,
          "isSpell": true
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Adelbert Steiner"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertExileCount",
          "player": 0,
          "name": "@steiner",
          "count": 1
        },
        {
          "op": "assertExileCount",
          "player": 0,
          "count": 1
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 0,
            "alias": "@spider"
          },
          "count": 1,
          "checks": [
            {
              "property": "subtype",
              "value": "Knight",
              "expected": true
            },
            {
              "property": "subtype",
              "value": "Spider",
              "expected": true
            },
            {
              "property": "subtype",
              "value": "Hero",
              "expected": true
            },
            {
              "property": "subtype",
              "value": "Human",
              "expected": true
            }
          ]
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Superior Spider-Man",
          "ability": "Lifelink",
          "expected": true
        }
      ]
    }
  ]
});
