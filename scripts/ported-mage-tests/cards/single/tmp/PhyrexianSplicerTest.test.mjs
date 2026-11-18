import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/tmp/PhyrexianSplicerTest.java",
  "tests": [
    {
      "name": "test_Normal",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Phyrexian Splicer",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Akroma, Angel of Wrath",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Augur il-Vec",
          "count": 1
        },
        {
          "op": "assertEachObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "seatIndependent": true,
          "selector": {
            "zone": "battlefield",
            "player": 0,
            "name": "Akroma, Angel of Wrath"
          },
          "count": 1,
          "checks": [
            {
              "property": "keyword",
              "value": "Trample",
              "expected": true
            }
          ]
        },
        {
          "op": "assertEachObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "seatIndependent": true,
          "selector": {
            "zone": "battlefield",
            "player": 0,
            "name": "Augur il-Vec"
          },
          "count": 1,
          "checks": [
            {
              "property": "keyword",
              "value": "Trample",
              "expected": false
            }
          ]
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "source": "Phyrexian Splicer",
          "ability": "{2}, {T}, Choose",
          "target": "Akroma, Angel of Wrath^Augur il-Vec"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Trample"
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "count": 1,
          "source": "Phyrexian Splicer",
          "abilityKind": "Activated",
          "objectTarget": "Akroma, Angel of Wrath"
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "count": 1,
          "source": "Phyrexian Splicer",
          "abilityKind": "Activated",
          "objectTarget": "Augur il-Vec"
        },
        {
          "op": "assertTappedCount",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Mountain",
          "tapped": true,
          "count": 2,
          "seatIndependent": true
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "resolutions": 1
        },
        {
          "op": "assertEachObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "seatIndependent": true,
          "selector": {
            "zone": "battlefield",
            "player": 0,
            "name": "Akroma, Angel of Wrath"
          },
          "count": 1,
          "checks": [
            {
              "property": "keyword",
              "value": "Trample",
              "expected": false
            }
          ]
        },
        {
          "op": "assertEachObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "seatIndependent": true,
          "selector": {
            "zone": "battlefield",
            "player": 0,
            "name": "Augur il-Vec"
          },
          "count": 1,
          "checks": [
            {
              "property": "keyword",
              "value": "Trample",
              "expected": true
            }
          ]
        },
        {
          "op": "assertEachObject",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "seatIndependent": true,
          "selector": {
            "zone": "battlefield",
            "player": 0,
            "name": "Akroma, Angel of Wrath"
          },
          "count": 1,
          "checks": [
            {
              "property": "keyword",
              "value": "Trample",
              "expected": true
            }
          ]
        },
        {
          "op": "assertEachObject",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "seatIndependent": true,
          "selector": {
            "zone": "battlefield",
            "player": 0,
            "name": "Augur il-Vec"
          },
          "count": 1,
          "checks": [
            {
              "property": "keyword",
              "value": "Trample",
              "expected": false
            }
          ]
        }
      ]
    }
  ]
});
