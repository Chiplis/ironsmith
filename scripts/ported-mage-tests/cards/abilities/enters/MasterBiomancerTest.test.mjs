import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/abilities/enters/MasterBiomancerTest.java",
  "tests": [
    {
      "name": "testCreatureGetsCounters",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 5
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Master Biomancer",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Mindeye Drake",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Mindeye Drake"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "execute"
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
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Master Biomancer",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Mindeye Drake",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Master Biomancer",
          "power": 2,
          "toughness": 4
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Mindeye Drake",
          "power": 4,
          "toughness": 7
        }
      ]
    },
    {
      "name": "testCreatureGetsDoubleCountersFromCorpsejackMenace",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Master Biomancer",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Corpsejack Menace",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Mindeye Drake",
          "count": 1
        },
        {
          "op": "castSpell",
          "name": "Corpsejack Menace",
          "player": 0,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertStackObject",
          "source": "Corpsejack Menace",
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
          "op": "assertCounterCount",
          "player": 0,
          "name": "Corpsejack Menace",
          "counter": "+1/+1",
          "count": 2
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Corpsejack Menace",
          "power": 6,
          "toughness": 6
        },
        {
          "op": "castSpell",
          "name": "Mindeye Drake",
          "player": 0,
          "turn": 3,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertStackObject",
          "source": "Mindeye Drake",
          "count": 1,
          "turn": 3,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "waitStackResolved",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Mindeye Drake",
          "counter": "+1/+1",
          "count": 4
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Mindeye Drake",
          "power": 6,
          "toughness": 9
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Master Biomancer",
          "power": 2,
          "toughness": 4
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
        }
      ]
    },
    {
      "name": "testWithProgenitorMimic",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
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
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Master Biomancer@original",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Progenitor Mimic@mimic",
          "count": 1
        },
        {
          "op": "castSpell",
          "name": "Progenitor Mimic",
          "player": 0,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "objectAlias": "@mimic"
        },
        {
          "op": "assertStackObject",
          "source": "Progenitor Mimic",
          "count": 1,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "setOptionChoices",
          "player": 0,
          "description": "Choose what to copy as this permanent enters",
          "kind": "options",
          "choices": [
            {
              "description": "Enter as a copy of Master Biomancer",
              "object": "@original"
            }
          ]
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Master Biomancer",
          "count": 2
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "@original",
          "power": 2,
          "toughness": 4
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "@mimic",
          "power": 4,
          "toughness": 6
        },
        {
          "op": "assertStackObject",
          "source": "@mimic",
          "count": 1,
          "turn": 3,
          "phase": "UPKEEP",
          "abilityKind": "Triggered"
        },
        {
          "op": "waitStackResolved",
          "turn": 3,
          "phase": "UPKEEP"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Master Biomancer",
          "count": 3
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 0,
            "name": "Master Biomancer",
            "token": true
          },
          "count": 1,
          "checks": [
            {
              "property": "power",
              "expected": 8
            },
            {
              "property": "toughness",
              "expected": 10
            }
          ]
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
        }
      ]
    }
  ]
});
