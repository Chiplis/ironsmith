import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/rna/PteramanderTest.java",
  "tests": [
    {
      "name": "testNoReduction",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 8
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Pteramander",
          "count": 1
        },
        {
          "op": "activateAbility",
          "source": "Pteramander",
          "ability": "Adapt 4",
          "player": 0,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertStackObject",
          "source": "Pteramander",
          "count": 1,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "abilityKind": "Activated"
        },
        {
          "op": "assertTappedCount",
          "player": 0,
          "name": "Island",
          "count": 8,
          "tapped": true
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Pteramander",
          "power": 5,
          "toughness": 5
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Pteramander",
          "count": 4,
          "counter": "+1/+1"
        }
      ]
    },
    {
      "name": "testReduction",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Pteramander",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Ancestral Recall",
          "count": 7
        },
        {
          "op": "activateAbility",
          "source": "Pteramander",
          "ability": "Adapt 4",
          "player": 0,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertStackObject",
          "source": "Pteramander",
          "count": 1,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "abilityKind": "Activated"
        },
        {
          "op": "assertTappedCount",
          "player": 0,
          "name": "Island",
          "count": 1,
          "tapped": true
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Pteramander",
          "power": 5,
          "toughness": 5
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Pteramander",
          "count": 4,
          "counter": "+1/+1"
        }
      ]
    }
  ]
});
