import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/mom/GhaltaAndMavrenTest.java",
  "tests": [
    {
      "name": "test_ghalta_attack_alone_dinosaur",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Ghalta and Mavren",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Blood Artist",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Ghalta and Mavren",
          "defender": 1
        },
        {
          "op": "setOptionChoices",
          "player": 0,
          "kind": "options",
          "description": "Choose a mode",
          "choices": [
            "Create a tapped and attacking X/X green Dinosaur creature token with trample, where X is the greatest power among other attacking creatures."
          ]
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": 1
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 0,
            "token": true
          },
          "count": 0,
          "checks": []
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 7
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 21
        }
      ]
    },
    {
      "name": "test_ghalta_attack_alone_vampire",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Ghalta and Mavren",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Blood Artist",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Ghalta and Mavren",
          "defender": 1
        },
        {
          "op": "setOptionChoices",
          "player": 0,
          "kind": "options",
          "description": "Choose a mode",
          "choices": [
            "Create X 1/1 white Vampire creature tokens with lifelink, where X is the number of other attacking creatures."
          ]
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 0,
            "token": true
          },
          "count": 0,
          "checks": []
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 8
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 20
        }
      ]
    },
    {
      "name": "test_vanguard_attack_alone_dinosaur",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Ghalta and Mavren",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Elite Vanguard",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Elite Vanguard",
          "defender": 1
        },
        {
          "op": "setOptionChoices",
          "player": 0,
          "kind": "options",
          "description": "Choose a mode",
          "choices": [
            "Create a tapped and attacking X/X green Dinosaur creature token with trample, where X is the greatest power among other attacking creatures."
          ]
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 0,
            "token": true
          },
          "count": 1,
          "checks": [
            {
              "property": "subtype",
              "value": "Dinosaur",
              "expected": true
            },
            {
              "property": "power",
              "expected": 2
            },
            {
              "property": "toughness",
              "expected": 2
            },
            {
              "property": "keyword",
              "value": "Trample",
              "expected": true
            }
          ]
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 16
        }
      ]
    },
    {
      "name": "test_bear_attack_alone_vampire",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Ghalta and Mavren",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Elite Vanguard",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Elite Vanguard",
          "defender": 1
        },
        {
          "op": "setOptionChoices",
          "player": 0,
          "kind": "options",
          "description": "Choose a mode",
          "choices": [
            "Create X 1/1 white Vampire creature tokens with lifelink, where X is the number of other attacking creatures."
          ]
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 0,
            "token": true
          },
          "count": 1,
          "checks": [
            {
              "property": "subtype",
              "value": "Vampire",
              "expected": true
            },
            {
              "property": "power",
              "expected": 1
            },
            {
              "property": "toughness",
              "expected": 1
            },
            {
              "property": "keyword",
              "value": "Lifelink",
              "expected": true
            }
          ]
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 18
        }
      ]
    },
    {
      "name": "test_vanguard_and_ghalta_attack_dinosaur",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Ghalta and Mavren",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Elite Vanguard",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Ghalta and Mavren",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Elite Vanguard",
          "defender": 1
        },
        {
          "op": "setOptionChoices",
          "player": 0,
          "kind": "options",
          "description": "Choose a mode",
          "choices": [
            "Create a tapped and attacking X/X green Dinosaur creature token with trample, where X is the greatest power among other attacking creatures."
          ]
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 0,
            "token": true
          },
          "count": 1,
          "checks": [
            {
              "property": "subtype",
              "value": "Dinosaur",
              "expected": true
            },
            {
              "property": "power",
              "expected": 2
            },
            {
              "property": "toughness",
              "expected": 2
            },
            {
              "property": "keyword",
              "value": "Trample",
              "expected": true
            }
          ]
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 4
        }
      ]
    },
    {
      "name": "test_vanguard_and_ghalta_attack_vampire",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Ghalta and Mavren",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Elite Vanguard",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Ghalta and Mavren",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Elite Vanguard",
          "defender": 1
        },
        {
          "op": "setOptionChoices",
          "player": 0,
          "kind": "options",
          "description": "Choose a mode",
          "choices": [
            "Create X 1/1 white Vampire creature tokens with lifelink, where X is the number of other attacking creatures."
          ]
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 0,
            "token": true
          },
          "count": 1,
          "checks": [
            {
              "property": "subtype",
              "value": "Vampire",
              "expected": true
            },
            {
              "property": "power",
              "expected": 1
            },
            {
              "property": "toughness",
              "expected": 1
            },
            {
              "property": "keyword",
              "value": "Lifelink",
              "expected": true
            }
          ]
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 6
        }
      ]
    },
    {
      "name": "test_varied_attackers_dinosaur",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Ghalta and Mavren",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Elite Vanguard",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Barony Vampire",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Armored Wolf-Rider",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Elite Vanguard",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Barony Vampire",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Armored Wolf-Rider",
          "defender": 1
        },
        {
          "op": "setOptionChoices",
          "player": 0,
          "kind": "options",
          "description": "Choose a mode",
          "choices": [
            "Create a tapped and attacking X/X green Dinosaur creature token with trample, where X is the greatest power among other attacking creatures."
          ]
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 0,
            "token": true
          },
          "count": 1,
          "checks": [
            {
              "property": "subtype",
              "value": "Dinosaur",
              "expected": true
            },
            {
              "property": "power",
              "expected": 4
            },
            {
              "property": "toughness",
              "expected": 4
            },
            {
              "property": "keyword",
              "value": "Trample",
              "expected": true
            }
          ]
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 7
        }
      ]
    },
    {
      "name": "test_5_attackers_vampire",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Ghalta and Mavren",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Elite Vanguard",
          "count": 5
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Elite Vanguard",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Elite Vanguard",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Elite Vanguard",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Elite Vanguard",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Elite Vanguard",
          "defender": 1
        },
        {
          "op": "setOptionChoices",
          "player": 0,
          "kind": "options",
          "description": "Choose a mode",
          "choices": [
            "Create X 1/1 white Vampire creature tokens with lifelink, where X is the number of other attacking creatures."
          ]
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 0,
            "token": true
          },
          "count": 5,
          "checks": [
            {
              "property": "subtype",
              "value": "Vampire",
              "expected": true
            },
            {
              "property": "power",
              "expected": 1
            },
            {
              "property": "toughness",
              "expected": 1
            },
            {
              "property": "keyword",
              "value": "Lifelink",
              "expected": true
            }
          ]
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": "20 - 2 * 5"
        }
      ]
    }
  ]
});
