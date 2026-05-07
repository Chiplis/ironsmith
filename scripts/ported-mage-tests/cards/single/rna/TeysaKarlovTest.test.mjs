import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/rna/TeysaKarlovTest.java",
  "tests": [
    {
      "name": "testTriggers",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Teysa Karlov",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Revel in Riches",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Bloodthrone Vampire",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Ministrant of Obligation",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "Sacrifice"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Ministrant of Obligation"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Afterlife"
        },
        {
          "op": "setStrictChooseMode",
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
          "name": "Bloodthrone Vampire",
          "power": 3,
          "toughness": 3
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Ministrant of Obligation",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Spirit Token",
          "count": 4
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Treasure Token",
          "count": 1
        }
      ]
    }
  ]
});
