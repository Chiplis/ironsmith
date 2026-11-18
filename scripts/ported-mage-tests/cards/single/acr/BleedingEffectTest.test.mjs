import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/acr/BleedingEffectTest.java",
  "tests": [
    {
      "name": "testAbilitiesGained",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Bleeding Effect",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Knight of Malice",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Boggart Brute",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Rumbling Baloth",
          "count": 1
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "BEGIN_COMBAT",
          "source": "Bleeding Effect",
          "count": 1,
          "abilityKind": "Triggered"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 0,
            "name": "Rumbling Baloth"
          },
          "count": 1,
          "checks": [
            {
              "property": "ability",
              "kind": "static",
              "value": "first strike",
              "expected": true
            },
            {
              "property": "ability",
              "kind": "static",
              "value": "hexproof from white",
              "expected": true
            },
            {
              "property": "ability",
              "kind": "static",
              "value": "menace",
              "expected": true
            },
            {
              "property": "ability",
              "kind": "static",
              "value": "flying",
              "expected": false
            },
            {
              "property": "ability",
              "kind": "static",
              "value": "hexproof from blue",
              "expected": false
            }
          ]
        }
      ]
    }
  ]
});
