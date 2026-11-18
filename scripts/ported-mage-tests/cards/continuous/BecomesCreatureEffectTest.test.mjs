import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/continuous/BecomesCreatureEffectTest.java",
  "tests": [
    {
      "name": "testBecomesCreatureAllEffect",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 5
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Dryad Arbor",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Ambush Commander",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Ambush Commander"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        }
      ]
    },
    {
      "name": "testBecomesCreatureAttachedEffect",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Dryad Arbor",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Frogify",
          "count": 1
        },
        {
          "op": "castSpell",
          "name": "Frogify",
          "player": 0,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "target": "Dryad Arbor"
        },
        {
          "op": "assertStackObject",
          "source": "Frogify",
          "count": 1,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "objectTarget": "Dryad Arbor"
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
            "name": "Dryad Arbor"
          },
          "count": 1,
          "checks": [
            {
              "property": "power",
              "expected": 1
            },
            {
              "property": "toughness",
              "expected": 1
            },
            {
              "property": "cardType",
              "expected": true,
              "value": "Creature"
            },
            {
              "property": "cardType",
              "expected": false,
              "value": "Land"
            },
            {
              "property": "subtype",
              "expected": true,
              "value": "Frog"
            },
            {
              "property": "subtype",
              "expected": false,
              "value": "Dryad"
            },
            {
              "property": "colorCount",
              "expected": 1
            },
            {
              "property": "color",
              "expected": true,
              "value": "Blue"
            },
            {
              "property": "color",
              "expected": false,
              "value": "Green"
            }
          ]
        },
        {
          "op": "assertAbilityKindCount",
          "player": 0,
          "name": "Dryad Arbor",
          "count": 0,
          "kind": "activated"
        },
        {
          "op": "assertAbilityKindCount",
          "player": 0,
          "name": "Dryad Arbor",
          "count": 0,
          "kind": "static"
        },
        {
          "op": "assertAbilityKindCount",
          "player": 0,
          "name": "Dryad Arbor",
          "count": 0,
          "kind": "triggered"
        }
      ]
    }
  ]
});
