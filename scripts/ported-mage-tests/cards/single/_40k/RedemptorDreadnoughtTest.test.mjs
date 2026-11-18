import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/_40k/RedemptorDreadnoughtTest.java",
  "tests": [
    {
      "name": "testCastNoAdditionalCost",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Redemptor Dreadnought",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Warpath Ghoul",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Wastes",
          "count": 5
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Redemptor Dreadnought"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": false
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Redemptor Dreadnought",
          "count": 1,
          "onTop": true
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "resolutions": 1
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
          "name": "Warpath Ghoul",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Redemptor Dreadnought",
          "power": 4,
          "toughness": 4
        },
        {
          "op": "assertExileCount",
          "player": 0,
          "name": "Warpath Ghoul",
          "count": 0
        }
      ]
    },
    {
      "name": "testCastAdditionalCostAndTrigger",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Redemptor Dreadnought",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Warpath Ghoul",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Wastes",
          "count": 5
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Redemptor Dreadnought"
        },
        {
          "op": "setOptionChoices",
          "player": 0,
          "kind": "options",
          "description": "Choose optional costs for Redemptor Dreadnought",
          "choices": [
            "Additional: special"
          ]
        },
        {
          "op": "setObjectChoices",
          "player": 0,
          "description": "Choose an object to exile: Exile a creature card from your graveyard",
          "choices": [
            "Warpath Ghoul"
          ]
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Redemptor Dreadnought",
          "count": 1
        },
        {
          "op": "assertExileCount",
          "player": 0,
          "name": "Warpath Ghoul",
          "count": 1
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Redemptor Dreadnought",
          "power": 4,
          "toughness": 4
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Redemptor Dreadnought",
          "defender": 1
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "END_TURN"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "execute"
        },
        {
          "op": "assertExileCount",
          "player": 0,
          "name": "Warpath Ghoul",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Redemptor Dreadnought",
          "power": 7,
          "toughness": 7
        }
      ]
    }
  ]
});
