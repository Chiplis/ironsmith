import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/continuous/BecomesCreatureAttachedTest.java",
  "tests": [
    {
      "name": "test_CreatureLandWithColor",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Dryad Arbor",
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
          "op": "setStopAt",
          "turn": 1,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Dryad Arbor",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Dryad Arbor",
          "power": 1,
          "toughness": 1
        },
        {
          "op": "unsupported",
          "source": "assertColor(playerA, \"Forest\", \"WUBGR\", false)"
        },
        {
          "op": "unsupported",
          "source": "assertColor(playerA, \"Dryad Arbor\", \"G\", true)"
        },
        {
          "op": "unsupported",
          "source": "assertColor(playerA, \"Dryad Arbor\", \"WUBR\", false)"
        }
      ]
    },
    {
      "name": "test_AttachToLandWithColorReplace",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Wind Zendikon",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Dryad Arbor",
          "count": 1
        },
        {
          "op": "castSpell",
          "name": "Wind Zendikon",
          "player": 0,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "target": "Dryad Arbor"
        },
        {
          "op": "assertStackObject",
          "source": "Wind Zendikon",
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
              "expected": 2
            },
            {
              "property": "toughness",
              "expected": 2
            },
            {
              "property": "cardType",
              "expected": true,
              "value": "Creature"
            },
            {
              "property": "cardType",
              "expected": true,
              "value": "Land"
            },
            {
              "property": "subtype",
              "expected": true,
              "value": "Elemental"
            },
            {
              "property": "keyword",
              "expected": true,
              "value": "flying"
            },
            {
              "property": "colorCount",
              "expected": 1
            },
            {
              "property": "color",
              "expected": true,
              "value": "blue"
            },
            {
              "property": "color",
              "expected": false,
              "value": "green"
            }
          ]
        }
      ]
    },
    {
      "name": "test_AttachToLandWithColorAdd",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Deep Freeze",
          "count": 1
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
          "name": "Dryad Arbor",
          "count": 1
        },
        {
          "op": "castSpell",
          "name": "Deep Freeze",
          "player": 0,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "target": "Dryad Arbor"
        },
        {
          "op": "assertStackObject",
          "source": "Deep Freeze",
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
              "expected": 0
            },
            {
              "property": "toughness",
              "expected": 4
            },
            {
              "property": "cardType",
              "expected": true,
              "value": "Creature"
            },
            {
              "property": "cardType",
              "expected": true,
              "value": "Land"
            },
            {
              "property": "subtype",
              "expected": true,
              "value": "Wall"
            },
            {
              "property": "keyword",
              "expected": true,
              "value": "defender"
            },
            {
              "property": "colorCount",
              "expected": 2
            },
            {
              "property": "color",
              "expected": true,
              "value": "blue"
            },
            {
              "property": "color",
              "expected": true,
              "value": "green"
            }
          ]
        }
      ]
    }
  ]
});
