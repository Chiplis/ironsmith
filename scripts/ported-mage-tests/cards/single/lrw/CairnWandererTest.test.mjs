import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/lrw/CairnWandererTest.java",
  "tests": [
    {
      "name": "TestCairnWandererEffect",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Cairn Wanderer",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Lantern Kami",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Prickly Boggart",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Serra Zealot",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Fencing Ace",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Typhoid Rats",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 1,
          "name": "Raging Goblin",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 1,
          "name": "Zodiac Rooster",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 1,
          "name": "Trained Caracal",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 1,
          "name": "Progenitus",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 1,
          "name": "Tree Monkey",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 1,
          "name": "Defiant Elf",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 1,
          "name": "Elvish Lookout",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 1,
          "name": "Veteran Cavalier",
          "count": 1
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
            "name": "Cairn Wanderer"
          },
          "count": 1,
          "checks": [
            {
              "property": "ability",
              "kind": "static",
              "value": "flying",
              "expected": true
            },
            {
              "property": "ability",
              "kind": "static",
              "value": "fear",
              "expected": true
            },
            {
              "property": "ability",
              "kind": "static",
              "value": "first strike",
              "expected": true
            },
            {
              "property": "ability",
              "kind": "static",
              "value": "double strike",
              "expected": true
            },
            {
              "property": "ability",
              "kind": "static",
              "value": "deathtouch",
              "expected": true
            },
            {
              "property": "ability",
              "kind": "static",
              "value": "haste",
              "expected": true
            },
            {
              "property": "ability",
              "kind": "static",
              "value": "plainswalk",
              "expected": true
            },
            {
              "property": "ability",
              "kind": "static",
              "value": "lifelink",
              "expected": true
            },
            {
              "property": "ability",
              "kind": "static",
              "value": "protection from everything",
              "expected": true
            },
            {
              "property": "ability",
              "kind": "static",
              "value": "reach",
              "expected": true
            },
            {
              "property": "ability",
              "kind": "static",
              "value": "shroud",
              "expected": true
            },
            {
              "property": "ability",
              "kind": "static",
              "value": "trample",
              "expected": true
            },
            {
              "property": "ability",
              "kind": "static",
              "value": "vigilance",
              "expected": true
            }
          ]
        }
      ]
    }
  ]
});
