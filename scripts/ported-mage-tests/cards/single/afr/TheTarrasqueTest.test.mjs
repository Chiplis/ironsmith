import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/afr/TheTarrasqueTest.java",
  "tests": [
    {
      "name": "testCast",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 9
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "The Tarrasque",
          "count": 1
        },
        {
          "op": "castSpell",
          "name": "The Tarrasque",
          "player": 0,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertStackObject",
          "source": "The Tarrasque",
          "count": 1,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertTappedCount",
          "player": 0,
          "name": "Forest",
          "count": 9,
          "tapped": true
        },
        {
          "op": "assertAttackDefenders",
          "turn": 1,
          "phase": "DECLARE_ATTACKERS",
          "player": 0,
          "attacker": "The Tarrasque",
          "defenders": [
            1
          ]
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "END_TURN"
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 0,
            "name": "The Tarrasque"
          },
          "count": 1,
          "checks": [
            {
              "property": "keyword",
              "expected": true,
              "value": "haste"
            },
            {
              "property": "keyword",
              "expected": true,
              "value": "ward"
            }
          ]
        },
        {
          "op": "waitStackResolved",
          "turn": 2,
          "phase": "END_TURN"
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 0,
            "name": "The Tarrasque"
          },
          "count": 1,
          "checks": [
            {
              "property": "keyword",
              "expected": true,
              "value": "haste"
            },
            {
              "property": "keyword",
              "expected": true,
              "value": "ward"
            }
          ]
        },
        {
          "op": "waitStackResolved",
          "turn": 3,
          "phase": "END_TURN"
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 0,
            "name": "The Tarrasque"
          },
          "count": 1,
          "checks": [
            {
              "property": "keyword",
              "expected": true,
              "value": "haste"
            },
            {
              "property": "keyword",
              "expected": true,
              "value": "ward"
            }
          ]
        },
        {
          "op": "waitStackResolved",
          "turn": 4,
          "phase": "END_TURN"
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 0,
            "name": "The Tarrasque"
          },
          "count": 1,
          "checks": [
            {
              "property": "keyword",
              "expected": true,
              "value": "haste"
            },
            {
              "property": "keyword",
              "expected": true,
              "value": "ward"
            }
          ]
        }
      ]
    },
    {
      "name": "testNotCast",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Elvish Piper",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "The Tarrasque",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{G},"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "The Tarrasque",
          "ability": "Haste",
          "expected": false
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "The Tarrasque",
          "ability": "Haste",
          "expected": false
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "The Tarrasque",
          "ability": "Haste",
          "expected": false
        },
        {
          "op": "setStopAt",
          "turn": 4,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "The Tarrasque",
          "ability": "Haste",
          "expected": false
        }
      ]
    },
    {
      "name": "testSakashima",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Elvish Piper",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "The Tarrasque",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Sakashima the Impostor",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{G},"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "The Tarrasque"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Sakashima the Impostor"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Sakashima the Impostor",
          "power": 10,
          "toughness": 10
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "The Tarrasque",
          "ability": "Haste",
          "expected": false
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Sakashima the Impostor",
          "ability": "Haste",
          "expected": true
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "The Tarrasque",
          "ability": "Haste",
          "expected": false
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Sakashima the Impostor",
          "ability": "Haste",
          "expected": true
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "The Tarrasque",
          "ability": "Haste",
          "expected": false
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Sakashima the Impostor",
          "ability": "Haste",
          "expected": true
        },
        {
          "op": "setStopAt",
          "turn": 4,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "The Tarrasque",
          "ability": "Haste",
          "expected": false
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Sakashima the Impostor",
          "ability": "Haste",
          "expected": true
        }
      ]
    }
  ]
});
