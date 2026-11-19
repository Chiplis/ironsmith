import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/spm/SpiderVerseTest.java",
  "tests": [
    {
      "name": "testSpiderVerse",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Spider-Verse",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Ashiok, Nightmare Muse",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "EXILED",
          "player": 1,
          "name": "Lightning Bolt",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "EXILED",
          "player": 1,
          "name": "Bear Cub",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "addCounters(1, PhaseStep.PRECOMBAT_MAIN, playerA, ashiokNightmareMuse, CounterType.LOYALTY, 9)"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "-7",
          "source": "Ashiok, Nightmare Muse"
        },
        {
          "op": "setOptionChoices",
          "player": 0,
          "kind": "boolean",
          "description": "Cast a spell without paying its mana cost?",
          "choices": [
            "Yes"
          ]
        },
        {
          "op": "setOptionChoices",
          "player": 0,
          "kind": "boolean",
          "description": "Cast a spell without paying its mana cost?",
          "choices": [
            "No"
          ]
        },
        {
          "op": "setOptionChoices",
          "player": 0,
          "kind": "boolean",
          "description": "Cast a spell without paying its mana cost?",
          "choices": [
            "No"
          ]
        },
        {
          "op": "setOptionChoices",
          "player": 0,
          "kind": "options",
          "description": "Choose one",
          "choices": [
            "Cast Lightning Bolt"
          ]
        },
        {
          "op": "setOptionChoices",
          "player": 0,
          "kind": "boolean",
          "description": "Copy it",
          "choices": [
            "Yes"
          ]
        },
        {
          "op": "setOptionChoices",
          "player": 0,
          "kind": "boolean",
          "description": "Perform the effect",
          "choices": [
            "Yes"
          ]
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": {
            "groups": [
              [
                3
              ]
            ]
          }
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": {
            "groups": [
              [
                1
              ]
            ]
          }
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Lightning Bolt",
          "count": 1
        },
        {
          "op": "assertExileCount",
          "player": 1,
          "name": "Bear Cub",
          "count": 1
        },
        {
          "op": "assertLife",
          "player": 3,
          "life": 17
        },
        {
          "op": "activateAbility",
          "turn": 5,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "-7",
          "source": "Ashiok, Nightmare Muse"
        },
        {
          "op": "setOptionChoices",
          "player": 0,
          "kind": "boolean",
          "description": "Cast a spell without paying its mana cost?",
          "choices": [
            "Yes"
          ]
        },
        {
          "op": "setOptionChoices",
          "player": 0,
          "kind": "boolean",
          "description": "Copy it",
          "choices": [
            "Yes"
          ]
        },
        {
          "op": "waitStackResolved",
          "turn": 5,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "name": "Bear Cub",
            "player": 0,
            "token": true
          },
          "count": 1,
          "checks": [
            {
              "property": "keyword",
              "value": "Haste",
              "expected": true
            }
          ]
        },
        {
          "op": "attack",
          "turn": 5,
          "player": 0,
          "attacker": "Bear Cub",
          "defender": "playerC"
        },
        {
          "op": "setStopAt",
          "turn": 5,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 17
        },
        {
          "op": "assertLife",
          "player": "playerD",
          "life": 17
        },
        {
          "op": "assertLife",
          "player": "playerC",
          "life": 18
        }
      ],
      "turnOrder": [
        0,
        3,
        2,
        1
      ]
    }
  ]
});
