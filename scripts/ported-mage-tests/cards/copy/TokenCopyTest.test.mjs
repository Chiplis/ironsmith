import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/copy/TokenCopyTest.java",
  "tests": [
    {
      "name": "testCopyDFC",
      "operations": [
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
          "name": "Kessig Prowler",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Rite of Replication",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Rite of Replication",
          "target": "Kessig Prowler"
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
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Rite of Replication",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Kessig Prowler",
          "count": 2
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Sinuous Predator",
          "count": 0
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 0,
            "name": "Kessig Prowler"
          },
          "checks": [
            {
              "property": "power",
              "expected": 2
            },
            {
              "property": "toughness",
              "expected": 1
            },
            {
              "property": "colorCount",
              "expected": 1
            },
            {
              "property": "color",
              "expected": true,
              "value": "green"
            },
            {
              "property": "subtype",
              "expected": true,
              "value": "Werewolf"
            },
            {
              "property": "subtype",
              "expected": true,
              "value": "Horror"
            },
            {
              "property": "subtype",
              "expected": false,
              "value": "Eldrazi"
            },
            {
              "property": "doubleFaced",
              "expected": true
            },
            {
              "property": "transformed",
              "expected": false
            },
            {
              "property": "manaValue",
              "expected": 1
            }
          ],
          "count": 2
        }
      ]
    },
    {
      "name": "testCopyDFCAndTransform",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Tropical Island",
          "count": 14
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Kessig Prowler",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Rite of Replication",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Rite of Replication",
          "target": "Kessig Prowler"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": false
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{4}{G}"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{4}{G}"
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
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Rite of Replication",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Kessig Prowler",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Sinuous Predator",
          "count": 2
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 0,
            "name": "Sinuous Predator"
          },
          "checks": [
            {
              "property": "power",
              "expected": 4
            },
            {
              "property": "toughness",
              "expected": 4
            },
            {
              "property": "colorCount",
              "expected": 0
            },
            {
              "property": "subtype",
              "expected": true,
              "value": "Werewolf"
            },
            {
              "property": "subtype",
              "expected": false,
              "value": "Horror"
            },
            {
              "property": "subtype",
              "expected": true,
              "value": "Eldrazi"
            },
            {
              "property": "doubleFaced",
              "expected": true
            },
            {
              "property": "transformed",
              "expected": true
            },
            {
              "property": "manaValue",
              "expected": 1
            }
          ],
          "count": 2
        }
      ]
    },
    {
      "name": "testCopyTransformedDFC",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Tropical Island",
          "count": 9
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Kessig Prowler",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Rite of Replication",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{4}{G}"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Rite of Replication",
          "target": "Sinuous Predator"
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
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Rite of Replication",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Kessig Prowler",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Sinuous Predator",
          "count": 2
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 0,
            "name": "Sinuous Predator"
          },
          "checks": [
            {
              "property": "power",
              "expected": 4
            },
            {
              "property": "toughness",
              "expected": 4
            },
            {
              "property": "colorCount",
              "expected": 0
            },
            {
              "property": "subtype",
              "expected": true,
              "value": "Werewolf"
            },
            {
              "property": "subtype",
              "expected": false,
              "value": "Horror"
            },
            {
              "property": "subtype",
              "expected": true,
              "value": "Eldrazi"
            },
            {
              "property": "doubleFaced",
              "expected": true
            },
            {
              "property": "transformed",
              "expected": true
            },
            {
              "property": "manaValue",
              "expected": 1
            }
          ],
          "count": 2
        }
      ]
    },
    {
      "name": "testBackFromTheBrink",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Tropical Island",
          "count": 7
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Back from the Brink",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Kessig Prowler",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "Exile"
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
          "op": "assertExileCount",
          "player": 0,
          "name": "Kessig Prowler",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Kessig Prowler",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Sinuous Predator",
          "count": 0
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 0,
            "name": "Kessig Prowler"
          },
          "checks": [
            {
              "property": "power",
              "expected": 2
            },
            {
              "property": "toughness",
              "expected": 1
            },
            {
              "property": "colorCount",
              "expected": 1
            },
            {
              "property": "color",
              "expected": true,
              "value": "green"
            },
            {
              "property": "subtype",
              "expected": true,
              "value": "Werewolf"
            },
            {
              "property": "subtype",
              "expected": true,
              "value": "Horror"
            },
            {
              "property": "subtype",
              "expected": false,
              "value": "Eldrazi"
            },
            {
              "property": "doubleFaced",
              "expected": true
            },
            {
              "property": "transformed",
              "expected": false
            },
            {
              "property": "manaValue",
              "expected": 1
            }
          ],
          "count": 1
        }
      ]
    },
    {
      "name": "testBackFromTheBrinkTransform",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Tropical Island",
          "count": 12
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Back from the Brink",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Kessig Prowler",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "Exile"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "ability": "{4}{G}"
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
          "op": "assertExileCount",
          "player": 0,
          "name": "Kessig Prowler",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Kessig Prowler",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Sinuous Predator",
          "count": 1
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 0,
            "name": "Sinuous Predator"
          },
          "checks": [
            {
              "property": "power",
              "expected": 4
            },
            {
              "property": "toughness",
              "expected": 4
            },
            {
              "property": "colorCount",
              "expected": 0
            },
            {
              "property": "subtype",
              "expected": true,
              "value": "Werewolf"
            },
            {
              "property": "subtype",
              "expected": false,
              "value": "Horror"
            },
            {
              "property": "subtype",
              "expected": true,
              "value": "Eldrazi"
            },
            {
              "property": "doubleFaced",
              "expected": true
            },
            {
              "property": "transformed",
              "expected": true
            },
            {
              "property": "manaValue",
              "expected": 1
            }
          ],
          "count": 1
        }
      ]
    }
  ]
});
