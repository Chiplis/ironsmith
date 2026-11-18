import { registerPortedMageTests } from "../../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/abilities/oneshot/counterspell/NotOfThisWorldTest.java",
  "tests": [
    {
      "name": "testCounterFirstSpell",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "The Abyss",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Island",
          "count": 7
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Not of This World",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Ruhan of the Fomori",
          "count": 1
        },
        {
          "op": "addTarget",
          "player": 1,
          "target": "Ruhan of the Fomori"
        },
        {
          "op": "assertStackObject",
          "turn": 2,
          "phase": "UPKEEP",
          "source": "The Abyss",
          "count": 1,
          "abilityKind": "Triggered",
          "objectTarget": "Ruhan of the Fomori",
          "completeTargets": true
        },
        {
          "op": "assertPowerToughness",
          "player": 1,
          "name": "Ruhan of the Fomori",
          "power": 7,
          "toughness": 7
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "UPKEEP",
          "player": 1,
          "name": "Not of This World",
          "target": "stack ability (At the beginning of each player's upkeep"
        },
        {
          "op": "assertStackObject",
          "turn": 2,
          "phase": "UPKEEP",
          "source": "Not of This World",
          "count": 1,
          "targetCount": 1,
          "onTop": true
        },
        {
          "op": "waitStackResolved",
          "turn": 2,
          "phase": "UPKEEP",
          "resolutions": 1
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertHandCount",
          "player": 1,
          "name": "Not of This World",
          "count": 0
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Not of This World",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Ruhan of the Fomori",
          "count": 1
        },
        {
          "op": "assertTappedCount",
          "player": 1,
          "name": "Island",
          "tapped": true,
          "count": 0
        }
      ]
    }
  ]
});
