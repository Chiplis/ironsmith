import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/copy/TheMimeoplasmTest.java",
  "tests": [
    {
      "name": "testCloneMimeoplasm",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "The Mimeoplasm@mimeo",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Clone@clone",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 1,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 1,
          "name": "Aven Riftwatcher",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 7
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
          "name": "The Mimeoplasm",
          "player": 0,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "objectAlias": "@mimeo"
        },
        {
          "op": "assertStackObject",
          "source": "The Mimeoplasm",
          "count": 1,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "isSpell": true
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Aven Riftwatcher"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Silvercoat Lion"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 0,
            "alias": "@mimeo"
          },
          "count": 1,
          "checks": []
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "@mimeo",
          "power": 4,
          "toughness": 5
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "@mimeo",
          "count": 2,
          "counter": "+1/+1"
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "@mimeo",
          "count": 3,
          "counter": "time"
        },
        {
          "op": "assertExileCount",
          "player": 1,
          "name": "Aven Riftwatcher",
          "count": 1
        },
        {
          "op": "assertExileCount",
          "player": 1,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 22
        },
        {
          "op": "castSpell",
          "name": "Clone",
          "player": 0,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "objectAlias": "@clone"
        },
        {
          "op": "assertStackObject",
          "source": "Clone",
          "count": 1,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "isSpell": true
        },
        {
          "op": "setOptionChoices",
          "player": 0,
          "kind": "options",
          "description": "Choose what to copy as this permanent enters",
          "choices": [
            "Enter as a copy of Aven Riftwatcher"
          ]
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 0,
            "alias": "@clone"
          },
          "count": 1,
          "checks": []
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "@clone",
          "power": 2,
          "toughness": 3
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "@clone",
          "count": 0,
          "counter": "+1/+1"
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "@clone",
          "count": 3,
          "counter": "time"
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 24
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 20
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Aven Riftwatcher",
          "count": 2
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "Clone",
          "count": 0
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": null,
          "count": 0
        }
      ]
    }
  ]
});
