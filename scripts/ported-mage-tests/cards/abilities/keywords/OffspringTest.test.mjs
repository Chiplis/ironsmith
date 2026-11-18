import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/abilities/keywords/OffspringTest.java",
  "tests": [
    {
      "name": "testNoPay",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Iridescent Vinelasher",
          "count": 1
        },
        {
          "op": "castSpell",
          "name": "Iridescent Vinelasher",
          "player": 0,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertStackObject",
          "source": "Iridescent Vinelasher",
          "count": 1,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "resolutions": 1
        },
        {
          "op": "assertStackObject",
          "source": "Iridescent Vinelasher",
          "count": 0,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "abilityKind": "Triggered"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Iridescent Vinelasher",
          "count": 1
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 0,
            "name": "Iridescent Vinelasher",
            "token": false
          },
          "count": 1,
          "checks": []
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 0,
            "name": "Iridescent Vinelasher",
            "token": true
          },
          "count": 0,
          "checks": []
        },
        {
          "op": "assertTappedCount",
          "player": 0,
          "name": "Swamp",
          "count": 1,
          "tapped": true
        }
      ]
    },
    {
      "name": "testPay",
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
          "zone": "HAND",
          "player": 0,
          "name": "Iridescent Vinelasher",
          "count": 1
        },
        {
          "op": "setOptionChoices",
          "player": 0,
          "kind": "options",
          "description": "Choose optional costs for Iridescent Vinelasher",
          "choices": [
            "Offspring: 2"
          ]
        },
        {
          "op": "castSpell",
          "name": "Iridescent Vinelasher",
          "player": 0,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertStackObject",
          "source": "Iridescent Vinelasher",
          "count": 1,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "resolutions": 1
        },
        {
          "op": "assertStackObject",
          "source": "Iridescent Vinelasher",
          "count": 1,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "abilityKind": "Triggered"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Iridescent Vinelasher",
          "count": 2
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 0,
            "name": "Iridescent Vinelasher",
            "token": false
          },
          "count": 1,
          "checks": []
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 0,
            "name": "Iridescent Vinelasher",
            "token": true
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
            }
          ]
        },
        {
          "op": "assertTappedCount",
          "player": 0,
          "name": "Swamp",
          "count": 3,
          "tapped": true
        }
      ]
    }
  ]
});
