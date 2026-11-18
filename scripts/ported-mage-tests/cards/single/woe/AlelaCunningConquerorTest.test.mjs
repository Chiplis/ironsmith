import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/woe/AlelaCunningConquerorTest.java",
  "tests": [
    {
      "name": "attackSinglePlayer",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Alela, Cunning Conqueror",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Pestermite",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Grizzly Bears",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": "playerC",
          "name": "Ancient Carp",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": "playerD",
          "name": "Devoted Hero",
          "count": 1
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 1,
            "name": "Grizzly Bears"
          },
          "count": 1,
          "checks": [
            {
              "property": "goaded",
              "expected": false
            }
          ]
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 2,
            "name": "Ancient Carp"
          },
          "count": 1,
          "checks": [
            {
              "property": "goaded",
              "expected": false
            }
          ]
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 3,
            "name": "Devoted Hero"
          },
          "count": 1,
          "checks": [
            {
              "property": "goaded",
              "expected": false
            }
          ]
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Alela, Cunning Conqueror",
          "defender": "playerC"
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Pestermite",
          "defender": "playerC"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Ancient Carp"
        },
        {
          "op": "assertStackObject",
          "source": "Alela, Cunning Conqueror",
          "count": 1,
          "turn": 1,
          "phase": "COMBAT_DAMAGE",
          "abilityKind": "Triggered",
          "completeTargets": true
        },
        {
          "op": "assertStackObject",
          "source": "Alela, Cunning Conqueror",
          "count": 1,
          "turn": 1,
          "phase": "COMBAT_DAMAGE",
          "abilityKind": "Triggered",
          "objectTarget": "Ancient Carp"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "COMBAT_DAMAGE"
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 1,
            "name": "Grizzly Bears"
          },
          "count": 1,
          "checks": [
            {
              "property": "goaded",
              "expected": false
            }
          ]
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 2,
            "name": "Ancient Carp"
          },
          "count": 1,
          "checks": [
            {
              "property": "goaded",
              "expected": true
            }
          ]
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 3,
            "name": "Devoted Hero"
          },
          "count": 1,
          "checks": [
            {
              "property": "goaded",
              "expected": false
            }
          ]
        },
        {
          "op": "assertLife",
          "player": "playerC",
          "life": 36
        }
      ],
      "startingLife": 40,
      "turnOrder": [
        0,
        3,
        2,
        1
      ]
    },
    {
      "name": "attackTwoPlayers",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Alela, Cunning Conqueror",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Pestermite",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Grizzly Bears",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": "playerC",
          "name": "Ancient Carp",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": "playerD",
          "name": "Devoted Hero",
          "count": 1
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 1,
            "name": "Grizzly Bears"
          },
          "count": 1,
          "checks": [
            {
              "property": "goaded",
              "expected": false
            }
          ]
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 2,
            "name": "Ancient Carp"
          },
          "count": 1,
          "checks": [
            {
              "property": "goaded",
              "expected": false
            }
          ]
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 3,
            "name": "Devoted Hero"
          },
          "count": 1,
          "checks": [
            {
              "property": "goaded",
              "expected": false
            }
          ]
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Alela, Cunning Conqueror",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Pestermite",
          "defender": "playerD"
        },
        {
          "op": "assertStackObject",
          "source": "Alela, Cunning Conqueror",
          "count": 2,
          "turn": 1,
          "phase": "COMBAT_DAMAGE",
          "abilityKind": "Triggered",
          "completeTargets": true
        },
        {
          "op": "assertStackObject",
          "source": "Alela, Cunning Conqueror",
          "count": 1,
          "turn": 1,
          "phase": "COMBAT_DAMAGE",
          "abilityKind": "Triggered",
          "objectTarget": "Grizzly Bears"
        },
        {
          "op": "assertStackObject",
          "source": "Alela, Cunning Conqueror",
          "count": 1,
          "turn": 1,
          "phase": "COMBAT_DAMAGE",
          "abilityKind": "Triggered",
          "objectTarget": "Devoted Hero"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "COMBAT_DAMAGE"
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 1,
            "name": "Grizzly Bears"
          },
          "count": 1,
          "checks": [
            {
              "property": "goaded",
              "expected": true
            }
          ]
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 2,
            "name": "Ancient Carp"
          },
          "count": 1,
          "checks": [
            {
              "property": "goaded",
              "expected": false
            }
          ]
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 3,
            "name": "Devoted Hero"
          },
          "count": 1,
          "checks": [
            {
              "property": "goaded",
              "expected": true
            }
          ]
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 38
        },
        {
          "op": "assertLife",
          "player": "playerD",
          "life": 38
        }
      ],
      "startingLife": 40,
      "turnOrder": [
        0,
        3,
        2,
        1
      ]
    },
    {
      "name": "attackWithNotAFaerie",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Alela, Cunning Conqueror",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Storm Crow",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Grizzly Bears",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": "playerC",
          "name": "Ancient Carp",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": "playerD",
          "name": "Devoted Hero",
          "count": 1
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 1,
            "name": "Grizzly Bears"
          },
          "count": 1,
          "checks": [
            {
              "property": "goaded",
              "expected": false
            }
          ]
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 2,
            "name": "Ancient Carp"
          },
          "count": 1,
          "checks": [
            {
              "property": "goaded",
              "expected": false
            }
          ]
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 3,
            "name": "Devoted Hero"
          },
          "count": 1,
          "checks": [
            {
              "property": "goaded",
              "expected": false
            }
          ]
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Alela, Cunning Conqueror",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Storm Crow",
          "defender": "playerD"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Grizzly Bears"
        },
        {
          "op": "assertStackObject",
          "source": "Alela, Cunning Conqueror",
          "count": 1,
          "turn": 1,
          "phase": "COMBAT_DAMAGE",
          "abilityKind": "Triggered",
          "completeTargets": true
        },
        {
          "op": "assertStackObject",
          "source": "Alela, Cunning Conqueror",
          "count": 1,
          "turn": 1,
          "phase": "COMBAT_DAMAGE",
          "abilityKind": "Triggered",
          "objectTarget": "Grizzly Bears"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "COMBAT_DAMAGE"
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 1,
            "name": "Grizzly Bears"
          },
          "count": 1,
          "checks": [
            {
              "property": "goaded",
              "expected": true
            }
          ]
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 2,
            "name": "Ancient Carp"
          },
          "count": 1,
          "checks": [
            {
              "property": "goaded",
              "expected": false
            }
          ]
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 3,
            "name": "Devoted Hero"
          },
          "count": 1,
          "checks": [
            {
              "property": "goaded",
              "expected": false
            }
          ]
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 38
        },
        {
          "op": "assertLife",
          "player": "playerD",
          "life": 39
        }
      ],
      "startingLife": 40,
      "turnOrder": [
        0,
        3,
        2,
        1
      ]
    }
  ]
});
