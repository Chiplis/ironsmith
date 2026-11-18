import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/cost/additional/RemoveCounterCostTest.java",
  "tests": [
    {
      "name": "testNovijenSages",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Novijen Sages",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 7
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Novijen Sages"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Novijen Sages",
          "power": 4,
          "toughness": 4
        },
        {
          "op": "activateAbility",
          "player": 0,
          "source": "Novijen Sages",
          "ability": "{1}, Remove 2 +1/+1 counters from among creatures you control: Draw a card.",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "setOptionChoices",
          "kind": "distribution",
          "player": 0,
          "description": "Distribute 2 among targets (assign exactly 2 total)",
          "choices": [
            "Novijen Sages",
            "Novijen Sages"
          ]
        },
        {
          "op": "assertStackObject",
          "source": "Novijen Sages",
          "count": 1,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "abilityKind": "Activated"
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Novijen Sages",
          "power": 2,
          "toughness": 2
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Novijen Sages",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Novijen Sages",
          "power": 2,
          "toughness": 2
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "count": 1
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
