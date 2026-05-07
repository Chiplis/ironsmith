import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/m21/BasrisSolidarityTest.java",
  "tests": [
    {
      "name": "addCountersToControlledCreatures",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Savannah Lions",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Scryb Sprites",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Raging Goblin",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Basri's Solidarity",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Basri's Solidarity"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "execute"
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Savannah Lions",
          "counter": "P1P1",
          "count": 1
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Scryb Sprites",
          "counter": "P1P1",
          "count": 1
        },
        {
          "op": "assertCounterCount",
          "player": 1,
          "name": "Raging Goblin",
          "counter": "P1P1",
          "count": 0
        }
      ]
    }
  ]
});
