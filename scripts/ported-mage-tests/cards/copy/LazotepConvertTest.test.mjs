import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/copy/LazotepConvertTest.java",
  "tests": [
    {
      "name": "testInvastionAmonkhetTransformed",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Badlands",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Underground Sea",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Mutagen Connoisseur",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Invasion of Amonkhet@siege",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Char",
          "count": 1
        },
        {
          "op": "castSpell",
          "name": "Invasion of Amonkhet",
          "player": 0,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "objectAlias": "@siege"
        },
        {
          "op": "assertStackObject",
          "source": "Invasion of Amonkhet",
          "count": 1,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "isSpell": true
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertBattleProtector",
          "player": 0,
          "protector": 1,
          "name": "Invasion of Amonkhet"
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "@siege",
          "count": 4,
          "counter": "defense"
        },
        {
          "op": "castSpell",
          "name": "Char",
          "player": 0,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "target": "@siege"
        },
        {
          "op": "assertStackObject",
          "source": "Char",
          "count": 1,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "isSpell": true,
          "objectTarget": "Invasion of Amonkhet"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "resolutions": 1
        },
        {
          "op": "assertStackObject",
          "source": "Invasion of Amonkhet",
          "count": 1,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "abilityKind": "Triggered"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "resolutions": 1
        },
        {
          "op": "assertStackObject",
          "source": "Lazotep Convert",
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
            "Enter as a copy of Mutagen Connoisseur"
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
            "alias": "@siege"
          },
          "count": 1,
          "checks": []
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Mutagen Connoisseur",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Mutagen Connoisseur",
          "count": 1
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 18
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "@siege",
          "power": 5,
          "toughness": 4
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 0,
            "alias": "@siege"
          },
          "count": 1,
          "checks": [
            {
              "property": "subtype",
              "value": "Vedalken",
              "expected": true
            },
            {
              "property": "subtype",
              "value": "Mutant",
              "expected": true
            },
            {
              "property": "subtype",
              "value": "Zombie",
              "expected": true
            },
            {
              "property": "transformed",
              "expected": true
            },
            {
              "property": "color",
              "value": "black",
              "expected": true
            },
            {
              "property": "color",
              "value": "green",
              "expected": true
            },
            {
              "property": "color",
              "value": "blue",
              "expected": true
            },
            {
              "property": "colorCount",
              "expected": 3
            }
          ]
        }
      ]
    }
  ]
});
