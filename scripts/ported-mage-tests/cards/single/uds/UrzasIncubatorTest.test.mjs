import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/uds/UrzasIncubatorTest.java",
  "tests": [
    {
      "name": "testEldraziCostReduction",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Urza's Incubator",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Eldrazi Displacer",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Eldrazi Mimic",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 4
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Urza's Incubator"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Eldrazi"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Eldrazi Displacer"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Eldrazi Mimic"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Eldrazi Mimic"
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
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Urza's Incubator",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Eldrazi Displacer",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Eldrazi Mimic",
          "count": 2
        }
      ]
    },
    {
      "name": "testEldraziCostReductionWastesRequirement",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Urza's Incubator",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Thought-Knot Seer",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 5
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Urza's Incubator"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Eldrazi"
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "label": "Cast Thought-Knot",
          "expected": false
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
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Urza's Incubator",
          "count": 1
        }
      ]
    }
  ]
});
