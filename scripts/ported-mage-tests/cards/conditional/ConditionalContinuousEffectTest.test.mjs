import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/conditional/ConditionalContinuousEffectTest.java",
  "tests": [
    {
      "name": "testManorGargoyle",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Manor Gargoyle",
          "count": 1
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 0,
            "name": "Manor Gargoyle"
          },
          "count": 1,
          "checks": [
            {
              "property": "keyword",
              "expected": true,
              "value": "defender"
            },
            {
              "property": "keyword",
              "expected": true,
              "value": "indestructible"
            }
          ]
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "source": "Manor Gargoyle",
          "ability": "loses defender and gains flying"
        },
        {
          "op": "assertStackObject",
          "source": "Manor Gargoyle",
          "count": 1,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "abilityKind": "Activated"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertTappedCount",
          "player": 0,
          "name": "Mountain",
          "count": 1,
          "tapped": true
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 0,
            "name": "Manor Gargoyle"
          },
          "count": 1,
          "checks": [
            {
              "property": "keyword",
              "expected": false,
              "value": "defender"
            },
            {
              "property": "keyword",
              "expected": false,
              "value": "indestructible"
            },
            {
              "property": "keyword",
              "expected": true,
              "value": "flying"
            }
          ]
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Manor Gargoyle",
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
          "life": 16
        },
        {
          "op": "waitStackResolved",
          "turn": 2,
          "phase": "UPKEEP"
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 0,
            "name": "Manor Gargoyle"
          },
          "count": 1,
          "checks": [
            {
              "property": "keyword",
              "expected": true,
              "value": "defender"
            },
            {
              "property": "keyword",
              "expected": true,
              "value": "indestructible"
            }
          ]
        }
      ]
    }
  ]
});
