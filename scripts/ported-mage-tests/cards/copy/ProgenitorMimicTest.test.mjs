import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/copy/ProgenitorMimicTest.java",
  "tests": [
    {
      "name": "testCloneTriggered",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Runeclaw Bear",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Progenitor Mimic@mimic",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Island",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Forest",
          "count": 3
        },
        {
          "op": "castSpell",
          "name": "Progenitor Mimic",
          "player": 1,
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "objectAlias": "@mimic"
        },
        {
          "op": "assertStackObject",
          "source": "Progenitor Mimic",
          "count": 1,
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "isSpell": true
        },
        {
          "op": "setOptionChoices",
          "player": 1,
          "kind": "options",
          "description": "Choose what to copy as this permanent enters",
          "choices": [
            "Enter as a copy of Runeclaw Bear"
          ]
        },
        {
          "op": "waitStackResolved",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 1,
            "alias": "@mimic"
          },
          "count": 1,
          "checks": []
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Runeclaw Bear",
          "count": 1
        },
        {
          "op": "assertStackObject",
          "source": "@mimic",
          "count": 1,
          "turn": 4,
          "phase": "UPKEEP",
          "abilityKind": "Triggered",
          "controller": 1
        },
        {
          "op": "waitStackResolved",
          "turn": 4,
          "phase": "UPKEEP"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Runeclaw Bear",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Runeclaw Bear",
          "count": 2
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 20
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 20
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 1,
            "name": "Runeclaw Bear",
            "token": false
          },
          "count": 1,
          "checks": []
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 1,
            "name": "Runeclaw Bear",
            "token": true
          },
          "count": 1,
          "checks": []
        }
      ]
    },
    {
      "name": "testTwoMimic",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Runeclaw Bear",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Narrow Escape",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Progenitor Mimic",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Island",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Forest",
          "count": 3
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Progenitor Mimic"
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": "Runeclaw Bear"
        },
        {
          "op": "castSpell",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Narrow Escape",
          "target": "Runeclaw Bear"
        },
        {
          "op": "castSpell",
          "turn": 4,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Progenitor Mimic"
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": "Runeclaw Bear"
        },
        {
          "op": "setStopAt",
          "turn": 6,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
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
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Narrow Escape",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Runeclaw Bear",
          "count": 0
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "Runeclaw Bear",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Runeclaw Bear",
          "count": 6
        }
      ]
    },
    {
      "name": "testChangeControl",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Captivating Vampire",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Child of Night",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Progenitor Mimic@mimic",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Island",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Forest",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Bloodrage Vampire",
          "count": 1
        },
        {
          "op": "castSpell",
          "name": "Progenitor Mimic",
          "player": 1,
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "objectAlias": "@mimic"
        },
        {
          "op": "assertStackObject",
          "source": "Progenitor Mimic",
          "count": 1,
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "isSpell": true
        },
        {
          "op": "setOptionChoices",
          "player": 1,
          "kind": "options",
          "description": "Choose what to copy as this permanent enters",
          "choices": [
            "Enter as a copy of Captivating Vampire"
          ]
        },
        {
          "op": "waitStackResolved",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 1,
            "alias": "@mimic"
          },
          "count": 1,
          "checks": []
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Captivating Vampire",
          "count": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Captivating Vampire"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Child of Night"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Child of Night"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Child of Night"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Child of Night"
        },
        {
          "op": "activateAbility",
          "source": "Captivating Vampire",
          "ability": "Gain control of target creature",
          "player": 0,
          "turn": 2,
          "phase": "POSTCOMBAT_MAIN",
          "target": "@mimic"
        },
        {
          "op": "assertStackObject",
          "source": "Captivating Vampire",
          "count": 1,
          "turn": 2,
          "phase": "POSTCOMBAT_MAIN",
          "abilityKind": "Activated",
          "objectTarget": "Captivating Vampire"
        },
        {
          "op": "waitStackResolved",
          "turn": 2,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 0,
            "alias": "@mimic"
          },
          "count": 1,
          "checks": []
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Captivating Vampire",
          "count": 2
        },
        {
          "op": "assertPowerToughness",
          "player": 1,
          "name": "Bloodrage Vampire",
          "power": 3,
          "toughness": 1
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 0,
            "name": "Captivating Vampire"
          },
          "count": 2,
          "checks": [
            {
              "property": "power",
              "expected": 3
            },
            {
              "property": "toughness",
              "expected": 3
            }
          ]
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 0,
            "name": "Child of Night"
          },
          "count": 4,
          "checks": [
            {
              "property": "power",
              "expected": 4
            },
            {
              "property": "toughness",
              "expected": 3
            }
          ]
        }
      ]
    },
    {
      "name": "testDeadbridgeChant",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Deadbridge Chant",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Progenitor Mimic",
          "count": 10
        },
        {
          "op": "skipInitShuffling"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Deadbridge Chant"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Silvercoat Lion"
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Deadbridge Chant",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 2
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Silvercoat Lion",
          "power": 2,
          "toughness": 2
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Progenitor Mimic",
          "count": 9
        }
      ]
    }
  ]
});
