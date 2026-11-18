import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/dis/OccularHaloTest.java",
  "tests": [
    {
      "name": "test_Vigilance",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Ocular Halo",
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
          "name": "Tundra",
          "count": 5
        },
        {
          "op": "castSpell",
          "name": "Ocular Halo",
          "player": 0,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "target": "Elite Vanguard"
        },
        {
          "op": "assertStackObject",
          "source": "Ocular Halo",
          "count": 1,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "objectTarget": "Elite Vanguard"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertAttachedTo",
          "player": 0,
          "attachment": "Ocular Halo",
          "target": "Elite Vanguard",
          "expected": true
        },
        {
          "op": "activateAbility",
          "source": "Ocular Halo",
          "ability": "gains vigilance",
          "player": 0,
          "turn": 1,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "assertStackObject",
          "source": "Ocular Halo",
          "count": 1,
          "turn": 1,
          "phase": "BEGIN_COMBAT",
          "abilityKind": "Activated"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 0,
            "name": "Elite Vanguard"
          },
          "count": 1,
          "checks": [
            {
              "property": "keyword",
              "expected": true,
              "value": "vigilance"
            }
          ]
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Elite Vanguard",
          "defender": 1
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "END_COMBAT"
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 18
        },
        {
          "op": "assertTappedCount",
          "player": 0,
          "name": "Elite Vanguard",
          "count": 1,
          "tapped": false
        }
      ]
    }
  ]
});
